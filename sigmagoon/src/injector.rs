use std::ffi::CString;
use std::fs;
use std::path::{Path, PathBuf};
use std::ptr::null;

use windows_sys::Win32::{
    Foundation::CloseHandle,
    System::{
        Diagnostics::Debug::WriteProcessMemory,
        LibraryLoader::{GetModuleHandleA, GetProcAddress},
        Memory::{
            MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_EXECUTE_READWRITE, PAGE_READWRITE,
            VirtualAllocEx, VirtualFreeEx,
        },
        Threading::{
            CreateRemoteThread, INFINITE, OpenProcess, PROCESS_CREATE_THREAD,
            PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
            WaitForSingleObject,
        },
    },
};

use memory::{DeltaruneView, find_process_id};

pub fn find_latest_dll() -> Result<PathBuf, String> {
    let candidates = [
        PathBuf::from("speed.dll"),
        PathBuf::from("target/release/speed.dll"),
        PathBuf::from("target/debug/speed.dll"),
    ];

    let mut latest: Option<(PathBuf, std::time::SystemTime)> = None;

    for path in &candidates {
        if path.exists() {
            if let Ok(metadata) = fs::metadata(path) {
                if let Ok(modified) = metadata.modified() {
                    match latest {
                        None => {
                            latest = Some((path.clone(), modified));
                        }
                        Some((_, ref lat_mod)) => {
                            if modified > *lat_mod {
                                latest = Some((path.clone(), modified));
                            }
                        }
                    }
                }
            }
        }
    }

    match latest {
        Some((path, _)) => {
            let abs_path = fs::canonicalize(&path).map_err(|e| {
                format!("failed to get absolute path for {}: {}", path.display(), e)
            })?;
            Ok(abs_path)
        }
        None => Err(
            "could not find speed.dll in current directory, target/release/ or target/debug/"
                .to_string(),
        ),
    }
}

pub fn remote_thread_inject(pid: u32, dll_path: &Path) -> Result<(), String> {
    let dll_path_str = dll_path
        .to_str()
        .ok_or_else(|| "dll path contains invalid unicode".to_string())?;
    let dll_path_cstring =
        CString::new(dll_path_str).map_err(|e| format!("invalid dll path cstring: {}", e))?;
    let bytes = dll_path_cstring.as_bytes_with_nul();

    let h_proc = unsafe {
        OpenProcess(
            PROCESS_CREATE_THREAD
                | PROCESS_QUERY_INFORMATION
                | PROCESS_VM_OPERATION
                | PROCESS_VM_WRITE
                | PROCESS_VM_READ,
            0,
            pid,
        )
    };

    if h_proc.is_null() {
        return Err(format!(
            "failed to open process (pid: {}). Error: {}",
            pid,
            std::io::Error::last_os_error()
        ));
    }

    let load_library_addr = unsafe {
        let kernel32 = GetModuleHandleA(b"kernel32.dll\0".as_ptr());
        if kernel32.is_null() {
            CloseHandle(h_proc);
            return Err("failed to get handle for kernel32.dll".to_string());
        }
        GetProcAddress(kernel32, b"LoadLibraryA\0".as_ptr())
    };

    if load_library_addr.is_none() {
        unsafe { CloseHandle(h_proc) };
        return Err("failed to find LoadLibraryA in kernel32.dll".to_string());
    }

    let load_library_addr = load_library_addr.unwrap();

    let remote_mem = unsafe {
        VirtualAllocEx(
            h_proc,
            null(),
            bytes.len(),
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        )
    };

    if remote_mem.is_null() {
        unsafe { CloseHandle(h_proc) };
        return Err(format!(
            "failed to allocate memory in target process: {}",
            std::io::Error::last_os_error()
        ));
    }

    let mut bytes_written = 0;
    let write_ok = unsafe {
        WriteProcessMemory(
            h_proc,
            remote_mem,
            bytes.as_ptr() as _,
            bytes.len(),
            &mut bytes_written,
        )
    };

    if write_ok == 0 {
        unsafe {
            VirtualFreeEx(h_proc, remote_mem, 0, MEM_RELEASE);
            CloseHandle(h_proc);
        }
        return Err(format!(
            "failed to write memory in target process: {}",
            std::io::Error::last_os_error()
        ));
    }

    let h_thread = unsafe {
        CreateRemoteThread(
            h_proc,
            null(),
            0,
            std::mem::transmute(load_library_addr),
            remote_mem,
            0,
            std::ptr::null_mut(),
        )
    };

    if h_thread.is_null() {
        unsafe {
            VirtualFreeEx(h_proc, remote_mem, 0, MEM_RELEASE);
            CloseHandle(h_proc);
        }
        return Err(format!(
            "failed to create remote thread: {}",
            std::io::Error::last_os_error()
        ));
    }

    unsafe {
        WaitForSingleObject(h_thread, INFINITE);
        CloseHandle(h_thread);
        VirtualFreeEx(h_proc, remote_mem, 0, MEM_RELEASE);
        CloseHandle(h_proc);
    }

    Ok(())
}

pub fn remote_set_env(pid: u32, name: &str, value: &str) -> Result<(), String> {
    let name_cstring =
        CString::new(name).map_err(|e| format!("invalid environment name: {}", e))?;
    let value_cstring =
        CString::new(value).map_err(|e| format!("invalid environment value: {}", e))?;

    let name_bytes = name_cstring.as_bytes_with_nul();
    let value_bytes = value_cstring.as_bytes_with_nul();

    let name_len = name_bytes.len();
    let value_len = value_bytes.len();

    // align shellcode offset to 16 bytes
    let shellcode_offset = ((name_len + value_len + 15) / 16) * 16;

    let h_proc = unsafe {
        OpenProcess(
            PROCESS_CREATE_THREAD
                | PROCESS_QUERY_INFORMATION
                | PROCESS_VM_OPERATION
                | PROCESS_VM_WRITE
                | PROCESS_VM_READ,
            0,
            pid,
        )
    };

    if h_proc.is_null() {
        return Err(format!(
            "failed to open process (pid: {}): {}",
            pid,
            std::io::Error::last_os_error()
        ));
    }

    let set_env_addr = unsafe {
        let kernel32 = GetModuleHandleA(b"kernel32.dll\0".as_ptr());
        if kernel32.is_null() {
            CloseHandle(h_proc);
            return Err("failed to get handle for kernel32.dll".to_string());
        }
        GetProcAddress(kernel32, b"SetEnvironmentVariableA\0".as_ptr())
    };

    if set_env_addr.is_none() {
        unsafe { CloseHandle(h_proc) };
        return Err("failed to find SetEnvironmentVariableA in kernel32.dll".to_string());
    }

    let set_env_addr = set_env_addr.unwrap();

    // construct shellcode
    let mut shellcode = Vec::with_capacity(41); // sub, 3x mov imm64, call, add, ret
    // sub rsp, 40 (0x28)
    shellcode.extend_from_slice(&[0x48, 0x83, 0xEC, 0x28]);
    // over but whatever
    let total_size = shellcode_offset + 100;

    let remote_mem = unsafe {
        VirtualAllocEx(
            h_proc,
            null(),
            total_size,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        )
    };

    if remote_mem.is_null() {
        unsafe { CloseHandle(h_proc) };
        return Err(format!(
            "failed to allocate memory in target process: {}",
            std::io::Error::last_os_error()
        ));
    }

    let remote_base = remote_mem as usize;
    let remote_name_addr = remote_base;
    let remote_value_addr = remote_base + name_len;
    let remote_shellcode_addr = remote_base + shellcode_offset;

    // mov rcx, remote_name_addr
    shellcode.extend_from_slice(&[0x48, 0xB9]);
    shellcode.extend_from_slice(&(remote_name_addr as u64).to_ne_bytes());
    // mov rdx, remote_value_addr
    shellcode.extend_from_slice(&[0x48, 0xBA]);
    shellcode.extend_from_slice(&(remote_value_addr as u64).to_ne_bytes());
    // mov rax, SetEnvironmentVariableA
    shellcode.extend_from_slice(&[0x48, 0xB8]);
    shellcode.extend_from_slice(&(set_env_addr as u64).to_ne_bytes());
    // call rax
    shellcode.extend_from_slice(&[0xFF, 0xD0]);
    // add rsp, 40 (0x28)
    shellcode.extend_from_slice(&[0x48, 0x83, 0xC4, 0x28]);
    // ret
    shellcode.extend_from_slice(&[0xC3]);

    // write all buffers
    let mut buffer = vec![0u8; shellcode_offset + shellcode.len()];
    buffer[0..name_len].copy_from_slice(name_bytes);
    buffer[name_len..(name_len + value_len)].copy_from_slice(value_bytes);
    buffer[shellcode_offset..(shellcode_offset + shellcode.len())].copy_from_slice(&shellcode);

    let mut bytes_written = 0;
    let write_ok = unsafe {
        WriteProcessMemory(
            h_proc,
            remote_mem,
            buffer.as_ptr() as _,
            buffer.len(),
            &mut bytes_written,
        )
    };

    if write_ok == 0 {
        unsafe {
            VirtualFreeEx(h_proc, remote_mem, 0, MEM_RELEASE);
            CloseHandle(h_proc);
        }
        return Err(format!(
            "failed to write memory in target process: {}",
            std::io::Error::last_os_error()
        ));
    }

    let h_thread = unsafe {
        CreateRemoteThread(
            h_proc,
            null(),
            0,
            std::mem::transmute(remote_shellcode_addr),
            null(),
            0,
            std::ptr::null_mut(),
        )
    };

    if h_thread.is_null() {
        unsafe {
            VirtualFreeEx(h_proc, remote_mem, 0, MEM_RELEASE);
            CloseHandle(h_proc);
        }
        return Err(format!(
            "failed to create remote thread for environment setting: {}",
            std::io::Error::last_os_error()
        ));
    }

    unsafe {
        WaitForSingleObject(h_thread, INFINITE);
        CloseHandle(h_thread);
        VirtualFreeEx(h_proc, remote_mem, 0, MEM_RELEASE);
        CloseHandle(h_proc);
    }

    Ok(())
}

pub fn inject_speed_cheat(speed: f64) -> Result<(), String> {
    let pid = find_process_id(DeltaruneView::PROCESS_NAME)
        .ok_or_else(|| format!("{} is not running!", DeltaruneView::PROCESS_NAME))?;
    let dll_path = find_latest_dll()?;
    let val_str = speed.to_string();
    remote_set_env(pid, "SIGMAGOON_SPD_VAL", &val_str)?;
    remote_thread_inject(pid, &dll_path)?;
    Ok(())
}
