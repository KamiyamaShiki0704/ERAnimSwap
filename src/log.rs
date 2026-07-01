use std::{
    ffi::{CString, c_void},
    fmt::Arguments,
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use windows::{
    Win32::{
        Foundation::HMODULE,
        System::{Diagnostics::Debug::OutputDebugStringA, LibraryLoader::GetModuleFileNameW},
    },
    core::PCSTR,
};

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();
static LOG_LOCK: Mutex<()> = Mutex::new(());

pub fn initialize(module: usize) {
    let path = module_path(module)
        .map(|path| path.with_file_name("weapon_animation_hotreload.log"))
        .unwrap_or_else(|| PathBuf::from("weapon_animation_hotreload.log"));
    let _ = LOG_PATH.set(path.clone());

    if let Ok(_guard) = LOG_LOCK.lock()
        && let Ok(mut file) = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
    {
        let _ = writeln!(file, "=== weapon-animation-hotreload log session ===");
        let _ = writeln!(file, "log_path={}", path.display());
    }

    line(format_args!("logger initialized"));
}

pub fn line(args: Arguments<'_>) {
    let mut text = args.to_string();
    text.push('\n');
    text.retain(|c| c != '\0');

    write_debug_string(&text);
    write_log_file(&text);
}

pub fn module_path(module: usize) -> Option<PathBuf> {
    let mut buffer = [0u16; 32768];
    let len = unsafe { GetModuleFileNameW(Some(HMODULE(module as *mut c_void)), &mut buffer) };

    if len == 0 {
        return None;
    }

    Some(PathBuf::from(String::from_utf16_lossy(
        &buffer[..len as usize],
    )))
}

fn write_debug_string(text: &str) {
    let Ok(cstr) = CString::new(text) else {
        return;
    };

    unsafe {
        OutputDebugStringA(PCSTR(cstr.as_ptr().cast()));
    }
}

fn write_log_file(text: &str) {
    let Some(path) = LOG_PATH.get() else {
        return;
    };

    let Ok(_guard) = LOG_LOCK.lock() else {
        return;
    };

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(text.as_bytes());
    }
}
