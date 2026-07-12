mod error;

use std::{
    ffi::CStr,
    mem::{MaybeUninit, size_of},
    ptr::null_mut,
};

use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE},
    System::{
        Diagnostics::{
            Debug::{ReadProcessMemory, WriteProcessMemory},
            ToolHelp::{
                CreateToolhelp32Snapshot, MODULEENTRY32, Module32First, Module32Next,
                PROCESSENTRY32, Process32First, Process32Next, TH32CS_SNAPMODULE,
                TH32CS_SNAPMODULE32, TH32CS_SNAPPROCESS,
            },
        },
        Threading::{
            OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION, PROCESS_VM_READ,
            PROCESS_VM_WRITE,
        },
    },
};

pub use error::{DeltaruneError, Result};

pub struct DeltaruneView {
    process: HANDLE,
    pub module_base: usize,
}

impl DeltaruneView {
    pub const PROCESS_NAME: &str = "DELTARUNE.exe";

    pub fn new() -> Result<Self> {
        let pid = find_process_id(Self::PROCESS_NAME)
            .ok_or_else(|| DeltaruneError::ProcessNotFound(Self::PROCESS_NAME.to_string()))?;

        let process = unsafe {
            OpenProcess(
                PROCESS_QUERY_INFORMATION
                    | PROCESS_VM_READ
                    | PROCESS_VM_WRITE
                    | PROCESS_VM_OPERATION,
                0,
                pid,
            )
        };

        if process == null_mut() {
            return Err(DeltaruneError::OpenProcessError(pid));
        }

        let module_base = find_module(pid, Self::PROCESS_NAME)
            .ok_or_else(|| DeltaruneError::ModuleNotFound(Self::PROCESS_NAME.to_string()))?;

        Ok(Self {
            process,
            module_base,
        })
    }

    pub fn read<T: Copy>(&self, address: usize) -> Result<T> {
        let mut value = MaybeUninit::<T>::uninit();
        let mut bytes_read = 0;

        let ok = unsafe {
            ReadProcessMemory(
                self.process,
                address as _,
                value.as_mut_ptr() as _,
                size_of::<T>(),
                &mut bytes_read,
            )
        };

        if ok == 0 {
            return Err(DeltaruneError::ReadError {
                address,
                source: std::io::Error::last_os_error().into(),
            });
        }

        Ok(unsafe { value.assume_init() })
    }

    pub fn write<T: Copy>(&self, address: usize, value: T) -> Result<()> {
        let mut bytes_written = 0;

        let ok = unsafe {
            WriteProcessMemory(
                self.process,
                address as _,
                &value as *const _ as _,
                size_of::<T>(),
                &mut bytes_written,
            )
        };

        if ok == 0 {
            return Err(DeltaruneError::WriteError {
                address,
                source: std::io::Error::last_os_error().into(),
            });
        }

        Ok(())
    }

    pub fn resolve_pointer(&self, base_offset: usize, offsets: &[isize]) -> Result<usize> {
        let mut address = self.module_base + base_offset;

        for &offset in offsets {
            let ptr: usize = self.read(address)?;
            address = (ptr as isize + offset) as usize;
        }

        Ok(address)
    }

    pub fn write_to_chain<T: Copy>(
        &self,
        base_offset: usize,
        offsets: &[isize],
        value: T,
    ) -> Result<()> {
        if offsets.is_empty() {
            return self.write(self.module_base + base_offset, value);
        }
        let final_address = self.resolve_pointer(base_offset, offsets)?;
        self.write(final_address, value)
    }

    pub fn read_from_chain<T: Copy>(&self, base_offset: usize, offsets: &[isize]) -> Result<T> {
        if offsets.is_empty() {
            return self.read(self.module_base + base_offset);
        }
        let final_address = self.resolve_pointer(base_offset, offsets)?;
        self.read(final_address)
    }

    pub fn modify_chain<T: Copy>(
        &self,
        base_offset: usize,
        offsets: &[isize],
        f: impl Fn(T) -> T,
    ) -> Result<()> {
        let value: T = self.read_from_chain(base_offset, offsets)?;
        self.write_to_chain(base_offset, offsets, f(value))
    }
}

impl Drop for DeltaruneView {
    fn drop(&mut self) {
        if self.process != null_mut() {
            unsafe {
                CloseHandle(self.process);
            }
        }
    }
}

pub fn find_process_id(name: &str) -> Option<u32> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };

    if snapshot == INVALID_HANDLE_VALUE {
        return None;
    }

    let mut entry = MaybeUninit::<PROCESSENTRY32>::uninit();
    unsafe {
        (*entry.as_mut_ptr()).dwSize = size_of::<PROCESSENTRY32>() as u32;
    }

    let mut pid = None;

    if unsafe { Process32First(snapshot, entry.as_mut_ptr()) != 0 } {
        let mut entry = unsafe { entry.assume_init() };
        loop {
            let exe = unsafe { CStr::from_ptr(entry.szExeFile.as_ptr() as _) }.to_string_lossy();

            if exe.eq_ignore_ascii_case(name) {
                pid = Some(entry.th32ProcessID);
                break;
            }

            if unsafe { Process32Next(snapshot, &mut entry) == 0 } {
                break;
            }
        }
    }

    unsafe { CloseHandle(snapshot) };
    pid
}

fn find_module(pid: u32, module: &str) -> Option<usize> {
    let snapshot =
        unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid) };

    if snapshot == INVALID_HANDLE_VALUE {
        return None;
    }

    let mut entry = MaybeUninit::<MODULEENTRY32>::uninit();
    unsafe {
        (*entry.as_mut_ptr()).dwSize = size_of::<MODULEENTRY32>() as u32;
    }

    let mut base = None;

    if unsafe { Module32First(snapshot, entry.as_mut_ptr()) != 0 } {
        let mut entry = unsafe { entry.assume_init() };
        loop {
            let module_name =
                unsafe { CStr::from_ptr(entry.szModule.as_ptr() as _) }.to_string_lossy();

            if module_name.eq_ignore_ascii_case(module) {
                base = Some(entry.modBaseAddr as usize);
                break;
            }

            if unsafe { Module32Next(snapshot, &mut entry) == 0 } {
                break;
            }
        }
    }

    unsafe { CloseHandle(snapshot) };
    base
}
