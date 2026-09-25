#![allow(
    non_snake_case,
    reason = "Keep the public DLL filename CapacityExpansion.dll."
)]

use std::ffi::c_void;

use windows::{
    Win32::{
        Foundation::{HINSTANCE, HMODULE},
        System::LibraryLoader::{
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_PIN,
            GetModuleHandleExW, GetModuleHandleW,
        },
    },
    core::{PCWSTR, w},
};

#[path = "../log.rs"]
mod log;
mod patch;

/// # Safety
/// Windows loader entry point. Startup loading only; keep loaded until exit.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllMain(module: HINSTANCE, reason: u32, _: *mut c_void) -> i32 {
    if reason != 1 {
        return 1;
    }
    process_attach(module)
}

// Keep process-only stack frames out of DLL_THREAD_ATTACH/DETACH, which run
// on the game's small native worker stacks before their entry point executes.
#[inline(never)]
fn process_attach(module: HINSTANCE) -> i32 {
    // Pin before starting the logger so an early FreeLibrary cannot unload it.
    let mut pinned = HMODULE::default();
    if unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_PIN,
            PCWSTR(module.0.cast()),
            &mut pinned,
        )
    }
    .is_err()
    {
        return 0;
    }

    // Do not defer installation to the logger: workers may be created as
    // soon as the loader returns. This path does no file I/O or allocation.
    let result = apply_to_host();
    let module = module.0 as usize;
    let _ = std::thread::Builder::new()
        .name("animation-capacity-log".into())
        .spawn(move || {
            log::initialize(module);
            log::line(format_args!(
                "animation-capacity {} (legacy weapon swap/reload disabled; old TOML ignored)",
                env!("CARGO_PKG_VERSION")
            ));
            match result {
                Ok(()) => log::line(format_args!(
                    "CAPACITY_PATCH_APPLIED rva=0x{:X} EzWork temporary budget={} -> {} bytes; CLIP_INDEX_PATCH_APPLIED valid indices=0..65533; -1/-2 reserved",
                    patch::PATCH_RVA, patch::OLD_BUDGET, patch::NEW_BUDGET
                )),
                Err(error) => log::line(format_args!("CAPACITY_PATCH_REFUSED: {error}")),
            }
            log::line(format_args!(
                "Startup loading and full restart required. Legacy swap/reload disabled. Clip-index expansion is bounded to 65534 combined slots, not unlimited TAE/event capacity."
            ));
        });
    1
}

fn apply_to_host() -> Result<(), &'static str> {
    let host = unsafe { GetModuleHandleW(None) }.map_err(|_| "cannot locate host image")?;
    let named = unsafe { GetModuleHandleW(w!("eldenring.exe")) }
        .map_err(|_| "host is not eldenring.exe; no memory writes")?;
    if host != named {
        return Err("host is not eldenring.exe; no memory writes");
    }
    unsafe { patch::apply(host.0 as usize) }
}
