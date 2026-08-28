use std::{thread, time::Duration, time::Instant};

use eldenring::{
    cs::{CSTaskGroupIndex, CSTaskImp},
    fd4::FD4TaskData,
};
use fromsoftware_shared::{FromStatic, InstanceError, RecurringTask};
use pelite::pe64::{Pe, PeView};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

const REGISTER_TASK_PATTERN: &[pelite::pattern::Atom] =
    pelite::pattern!("e8 ? ? ? ? 48 8b 0d ? ? ? ? 4c 8b c7 8b d3 e8 $ { ' }");

type RegisterTaskFn =
    unsafe extern "C" fn(&CSTaskImp, CSTaskGroupIndex, &RecurringTask<FD4TaskData>);

#[derive(Clone, Copy)]
pub(crate) struct RuntimeApi {
    register_task: RegisterTaskFn,
    register_task_rva: u32,
}

impl RuntimeApi {
    pub(crate) fn resolve_current() -> Result<Self, String> {
        let module = unsafe { GetModuleHandleW(None) }
            .map_err(|err| format!("GetModuleHandleW(None) failed: {err:?}"))?;
        let base = module.0 as usize;
        if base == 0 {
            return Err("eldenring.exe module base is null".to_string());
        }

        let pe = unsafe { PeView::module(base as *const u8) };
        let register_task_rva = resolve_register_task_rva(pe)?;
        Ok(Self {
            register_task: unsafe {
                std::mem::transmute::<usize, RegisterTaskFn>(base + register_task_rva as usize)
            },
            register_task_rva,
        })
    }

    pub(crate) fn register_task_rva(self) -> u32 {
        self.register_task_rva
    }

    pub(crate) fn install_recurring_task<F>(
        self,
        cs_task: &'static CSTaskImp,
        group: CSTaskGroupIndex,
        execute: F,
    ) where
        F: FnMut(&FD4TaskData) + Send + 'static,
    {
        // The game owns this task for the remainder of the process lifetime.
        let task = Box::leak(Box::new(RecurringTask::new(execute)));
        unsafe { (self.register_task)(cs_task, group, task) };
    }
}

fn resolve_register_task_rva<'a, P: Pe<'a>>(pe: P) -> Result<u32, String> {
    let mut captures = [0; 2];
    if !pe
        .scanner()
        .finds_code(REGISTER_TASK_PATTERN, &mut captures)
    {
        return Err("register_task signature was missing or ambiguous".to_string());
    }

    let rva = captures[1];
    if rva == 0 {
        return Err("register_task signature resolved a null RVA".to_string());
    }
    Ok(rva)
}

pub(crate) fn wait_for_cs_task(timeout: Duration) -> Result<&'static CSTaskImp, String> {
    let started = Instant::now();

    loop {
        let current_error = match unsafe { CSTaskImp::instance() } {
            Ok(cs_task) => return Ok(cs_task),
            Err(InstanceError::NotFound(name)) => format!("singleton not found: {name}"),
            Err(InstanceError::Null(name)) => format!("singleton not initialized: {name}"),
        };

        if started.elapsed() >= timeout {
            return Err(format!(
                "CSTaskImp was unavailable after {:.1} seconds ({current_error})",
                timeout.as_secs_f32()
            ));
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use pelite::pe64::{Pe, PeFile};

    use super::*;

    const REGISTER_TASK_RAW_AOB: &str =
        "E8 ?? ?? ?? ?? 48 8B 0D ?? ?? ?? ?? 4C 8B C7 8B D3 E8 ?? ?? ?? ??";

    fn configured_game_executable() -> Option<PathBuf> {
        std::env::var_os("ER_GAME_EXE").map(PathBuf::from)
    }

    #[test]
    fn compatibility_runtime_symbols_resolve_from_configured_game_executable() {
        let Some(path) = configured_game_executable() else {
            eprintln!("ER_GAME_EXE is not set; static game compatibility check skipped");
            return;
        };

        let bytes = fs::read(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        let pe = PeFile::from_bytes(&bytes)
            .unwrap_or_else(|err| panic!("failed to parse {}: {err:?}", path.display()));
        let register_task_rva = resolve_register_task_rva(pe)
            .unwrap_or_else(|err| panic!("register_task compatibility check failed: {err}"));

        let text = pe
            .section_headers()
            .by_name(".text")
            .and_then(|section| pe.get_section_bytes(section).ok())
            .expect(".text section missing or malformed");
        let fd4_singleton_candidates = from_singleton::find::fd4_singleton_pat_iter(text).count();
        assert!(
            fd4_singleton_candidates > 0,
            "FD4 singleton reflection signature produced no candidates"
        );
        for name in [
            b"CSTask\0".as_slice(),
            b"WorldChrMan\0",
            b"SoloParamRepository\0",
        ] {
            assert!(
                bytes.windows(name.len()).any(|window| window == name),
                "required DLRF singleton name {} was not found",
                String::from_utf8_lossy(&name[..name.len() - 1])
            );
        }

        let crash_pattern = crate::parse_aob(crate::DEFAULT_CRASH_PATCH_AOB)
            .expect("default crash patch AOB should parse");
        let crash_matches = crate::aob_match_offsets(&bytes, &crash_pattern);
        assert_eq!(
            crash_matches.len(),
            1,
            "crash patch AOB must have exactly one match"
        );

        println!(
            "compatible executable={} register_task=0x{register_task_rva:X} fd4_singleton_candidates={fd4_singleton_candidates} required_singleton_names=ok crash_patch_matches=1",
            path.display(),
        );
    }

    #[test]
    fn compatibility_missing_task_pattern_is_rejected() {
        let Some(path) = configured_game_executable() else {
            eprintln!("ER_GAME_EXE is not set; negative compatibility check skipped");
            return;
        };

        let mut bytes = fs::read(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        let raw_pattern =
            crate::parse_aob(REGISTER_TASK_RAW_AOB).expect("register_task raw AOB should parse");
        let matches = crate::aob_match_offsets(&bytes, &raw_pattern);
        assert_eq!(matches.len(), 1, "fixture requires one task pattern");
        bytes[matches[0]] = 0x90;

        let pe = PeFile::from_bytes(&bytes).expect("mutated fixture should remain a valid PE");
        let error = resolve_register_task_rva(pe).expect_err("missing pattern must be rejected");
        assert!(error.contains("missing or ambiguous"));
    }
}
