use std::{
    ffi::c_void,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use ilhook::x64::{CallbackOption, HookFlags, HookPoint, HookType, Hooker, Registers};

use windows_sys::Win32::{
    Foundation::HMODULE,
    Media::timeGetTime,
    System::{
        Console::AllocConsole,
        LibraryLoader::DisableThreadLibraryCalls,
        Performance::QueryPerformanceCounter,
        SystemInformation::{GetTickCount, GetTickCount64},
        SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
    },
};

static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);
static LOAD_GUARD: Mutex<()> = Mutex::new(());
static SPEED: Mutex<f64> = Mutex::new(-1.0);

#[allow(dead_code)]
struct HookPointsHolder(Vec<HookPoint>);
unsafe impl Send for HookPointsHolder {}
unsafe impl Sync for HookPointsHolder {}

static MANAGER: OnceLock<HookPointsHolder> = OnceLock::new();

pub const ENV_VAR_NAME: &str = "SIGMAGOON_SPD_VAL";
pub const CHECK_ENV_VAR_EVERY_MS: u64 = 500;

fn get_speed() -> f64 {
    *SPEED.lock().unwrap()
}

fn set_speed(speed: f64) {
    *SPEED.lock().unwrap() = speed;
}

fn install_hook(target: usize, hook_type: HookType, name: &str) -> Result<HookPoint, String> {
    let hooker = Hooker::new(
        target,
        hook_type,
        CallbackOption::None,
        0,
        HookFlags::empty(),
    );
    unsafe {
        hooker
            .hook()
            .map_err(|e| format!("failed to hook {}: {}", name, e))
    }
}

fn get_speed_from_env() -> Option<f64> {
    std::env::var(ENV_VAR_NAME)
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
}

fn current_speed() -> f64 {
    get_speed()
}

fn scale_u32(real: u32, speed: f64) -> u32 {
    if speed < 0.0 {
        return real;
    }
    ((real as f64) * speed) as u32
}

fn scale_u64(real: u64, speed: f64) -> u64 {
    if speed < 0.0 {
        return real;
    }
    ((real as f64) * speed) as u64
}

unsafe extern "win64" fn hook_get_tick_count(
    _reg: *mut Registers,
    ori_func_ptr: usize,
    _: usize,
) -> usize {
    let orig: unsafe extern "system" fn() -> u32 = unsafe { std::mem::transmute(ori_func_ptr) };
    let real = unsafe { orig() };
    scale_u32(real, current_speed()) as usize
}

unsafe extern "win64" fn hook_get_tick_count_64(
    _reg: *mut Registers,
    ori_func_ptr: usize,
    _: usize,
) -> usize {
    let orig: unsafe extern "system" fn() -> u64 = unsafe { std::mem::transmute(ori_func_ptr) };
    let real = unsafe { orig() };
    scale_u64(real, current_speed()) as usize
}

unsafe extern "win64" fn hook_time_get_time(
    _reg: *mut Registers,
    ori_func_ptr: usize,
    _: usize,
) -> usize {
    let orig: unsafe extern "system" fn() -> u32 = unsafe { std::mem::transmute(ori_func_ptr) };
    let real = unsafe { orig() };
    scale_u32(real, current_speed()) as usize
}

unsafe extern "win64" fn hook_query_performance_counter(
    reg: *mut Registers,
    ori_func_ptr: usize,
    _: usize,
) -> usize {
    unsafe {
        let counter_ptr = (*reg).rcx as *mut i64;

        let orig: unsafe extern "system" fn(*mut i64) -> i32 = std::mem::transmute(ori_func_ptr);
        let result = orig(counter_ptr);

        let speed = current_speed();
        if result != 0 && !counter_ptr.is_null() && speed >= 0.0 {
            *counter_ptr = ((*counter_ptr as f64) * speed) as i64;
        }

        result as usize
    }
}

fn init_manager() -> Result<(), String> {
    if let Some(initial) = get_speed_from_env() {
        set_speed(initial);
    }

    let hook_points = vec![
        install_hook(
            GetTickCount as *const () as usize,
            HookType::Retn(hook_get_tick_count),
            "GetTickCount",
        )?,
        install_hook(
            GetTickCount64 as *const () as usize,
            HookType::Retn(hook_get_tick_count_64),
            "GetTickCount64",
        )?,
        install_hook(
            timeGetTime as *const () as usize,
            HookType::Retn(hook_time_get_time),
            "timeGetTime",
        )?,
        install_hook(
            QueryPerformanceCounter as *const () as usize,
            HookType::Retn(hook_query_performance_counter),
            "QueryPerformanceCounter",
        )?,
    ];

    MANAGER
        .set(HookPointsHolder(hook_points))
        .map_err(|_| "SpeedCheat already initialized".to_string())
}

fn dll_main_attach(hinst_dll: HMODULE) -> i32 {
    let Ok(_lock) = LOAD_GUARD.try_lock() else {
        return 0;
    };

    unsafe {
        DisableThreadLibraryCalls(hinst_dll);
        AllocConsole();
    }

    if let Err(e) = init_manager() {
        println!("failed to initialize SpeedCheat: {}", e);
        println!("press enter to exit");
        let mut input = String::new();
        let _ = std::io::stdin().read_line(&mut input);
        return 0;
    }

    spawn_env_watcher();
    1
}

fn dll_main_detach(_hinst_dll: HMODULE) -> i32 {
    SHUTDOWN_FLAG.store(true, Ordering::SeqCst);
    1
}

fn spawn_env_watcher() {
    println!(
        "env_watcher started. will check for speed value change every {} ms.",
        CHECK_ENV_VAR_EVERY_MS
    );
    println!(
        "dont close this window as it will close the game too! to change speed, you can use the sigmagoon repl :)"
    );
    std::thread::spawn(|| {
        let mut last_speed = get_speed();

        while !SHUTDOWN_FLAG.load(Ordering::Acquire) {
            if let Some(new_speed) = get_speed_from_env() {
                if new_speed != last_speed {
                    println!("new speed detected: {}", new_speed);
                    set_speed(new_speed);
                    last_speed = new_speed;
                }
            }
            std::thread::sleep(Duration::from_millis(CHECK_ENV_VAR_EVERY_MS));
        }
    });
}

#[unsafe(no_mangle)]
pub extern "system" fn DllMain(
    hinst_dll: HMODULE,
    fdw_reason: u32,
    _lpv_reserved: *mut c_void,
) -> i32 {
    match fdw_reason {
        DLL_PROCESS_ATTACH => dll_main_attach(hinst_dll),
        DLL_PROCESS_DETACH => dll_main_detach(hinst_dll),
        _ => 1,
    }
}
