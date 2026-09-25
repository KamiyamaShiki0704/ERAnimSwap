use super::*;
use windows::Win32::System::Memory::{
    MEM_RELEASE, MEM_RESERVE, PAGE_EXECUTE_READ, PAGE_READWRITE, VirtualAlloc, VirtualFree,
};

fn fixture() -> Vec<u8> {
    let mut image = vec![0; IMAGE_LENGTH];
    image[..2].copy_from_slice(b"MZ");
    image[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
    image[0x80..0x84].copy_from_slice(b"PE\0\0");
    image[0x84..0x86].copy_from_slice(&0x8664u16.to_le_bytes());
    image[0x86..0x88].copy_from_slice(&11u16.to_le_bytes());
    image[0x88..0x8c].copy_from_slice(&0x6A96B418u32.to_le_bytes());
    image[0x94..0x96].copy_from_slice(&240u16.to_le_bytes());
    image[0x98..0x9a].copy_from_slice(&0x20bu16.to_le_bytes());
    image[0xd0..0xd4].copy_from_slice(&(IMAGE_LENGTH as u32).to_le_bytes());
    image[0x188..0x190].copy_from_slice(b".text\0\0\0");
    image[0x190..0x194].copy_from_slice(&(TEXT_LENGTH as u32).to_le_bytes());
    image[0x194..0x198].copy_from_slice(&(TEXT_RVA as u32).to_le_bytes());
    for &(rva, expected) in GUARDS {
        image[rva..rva + expected.len()].copy_from_slice(expected);
    }
    for edit in EDITS {
        image[edit.rva..edit.rva + edit.original.len()].copy_from_slice(edit.original);
    }
    image
}

fn check(image: &[u8]) -> Result<(), &'static str> {
    validate(|rva, length| image.get(rva..rva + length))
}

#[test]
fn valid_fixture_and_build_mutations() {
    let mut image = fixture();
    assert_eq!(check(&image), Ok(()));
    for offset in [
        0, 0x3c, 0x80, 0x84, 0x86, 0x88, 0x94, 0x98, 0xd0, 0x188, 0x190, 0x194,
    ] {
        image[offset] ^= 1;
        assert!(check(&image).is_err(), "accepted mutation at {offset:X}");
        image[offset] ^= 1;
    }
    assert!(check(&image[..4095]).is_err());
    assert!(check(&image[..TEXT_RVA + TEXT_LENGTH - 1]).is_err());
}

#[test]
fn downstream_guards_and_duplicate_refused_without_mutation() {
    let mut image = fixture();
    for &(rva, expected) in GUARDS {
        for offset in 0..expected.len() {
            image[rva + offset] ^= 1;
            assert!(check(&image).is_err());
            image[rva + offset] ^= 1;
        }
    }
    image[0x2000..0x2000 + WINDOW.len()].copy_from_slice(WINDOW);
    assert_eq!(
        check(&image),
        Err("constructor signature missing or ambiguous")
    );
    assert_eq!(image[PATCH_RVA], 2);
}

#[test]
fn larger_and_already_modified_budgets_are_never_lowered() {
    let mut image = fixture();
    for value in [0, 8, 0x10, 0x20, 0x80, 0xff] {
        image[PATCH_RVA] = value;
        assert!(check(&image).is_err());
        assert_eq!(image[PATCH_RVA], value);
    }
}

#[test]
fn clip_guards_mixed_patches_and_late_world_refuse_before_writes() {
    let mut image = fixture();
    for edit in EDITS {
        for i in 0..edit.original.len() {
            image[edit.rva + i] ^= 1;
            assert!(check(&image).is_err());
            image[edit.rva + i] ^= 1;
        }
        image[edit.rva..edit.rva + edit.replacement.len()].copy_from_slice(edit.replacement);
        assert!(check(&image).is_err());
        image[edit.rva..edit.rva + edit.original.len()].copy_from_slice(edit.original);
    }
    image[WORLD_RVA] = 1;
    assert!(
        check(&image)
            .unwrap_err()
            .contains("startup loading required")
    );
}

#[test]
fn transaction_api_failures_restore_all_windows_and_page_protections() {
    let pointer =
        unsafe { VirtualAlloc(None, IMAGE_LENGTH, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE) }
            .cast::<u8>();
    assert!(!pointer.is_null());
    let page = CodePage(pointer);
    let base = pointer as usize;
    for edit in EDITS {
        unsafe {
            std::ptr::copy_nonoverlapping(
                edit.original.as_ptr(),
                pointer.add(edit.rva),
                edit.original.len(),
            )
        };
    }
    let mut old = PAGE_PROTECTION_FLAGS::default();
    unsafe { VirtualProtect(pointer.cast(), IMAGE_LENGTH, PAGE_EXECUTE_READ, &mut old) }.unwrap();
    // Four unique pages, flush, then four restores. Name edits share one page.
    for failure in 0..9 {
        let mut call = 0;
        let result = unsafe {
            write_edits(base, || {
                let allow = call != failure;
                call += 1;
                allow
            })
        };
        assert!(
            result.is_err(),
            "injected operation {failure} was not exercised"
        );
        for edit in EDITS {
            let actual =
                unsafe { std::slice::from_raw_parts(pointer.add(edit.rva), edit.original.len()) };
            assert_eq!(actual, edit.original, "rollback operation {failure}");
            let mut info = MEMORY_BASIC_INFORMATION::default();
            unsafe {
                VirtualQuery(
                    Some(pointer.add(edit.rva).cast()),
                    &mut info,
                    size_of::<MEMORY_BASIC_INFORMATION>(),
                )
            };
            assert_eq!(info.Protect, PAGE_EXECUTE_READ);
        }
    }
    assert_eq!(unsafe { write_edits(base, || true) }, Ok(()));
    for edit in EDITS {
        let actual =
            unsafe { std::slice::from_raw_parts(pointer.add(edit.rva), edit.replacement.len()) };
        assert_eq!(actual, edit.replacement);
    }
    assert!(unsafe { write_edits(base, || true) }.is_err());
    for edit in EDITS {
        let actual =
            unsafe { std::slice::from_raw_parts(pointer.add(edit.rva), edit.replacement.len()) };
        assert_eq!(
            actual, edit.replacement,
            "a repeated call must not undo prior patches"
        );
    }
    drop(page);
}

pub(super) struct CodePage(pub(super) *mut u8);
impl CodePage {
    pub(super) fn new(code: &[u8]) -> Self {
        let pointer = unsafe { VirtualAlloc(None, 4096, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE) }
            .cast::<u8>();
        assert!(!pointer.is_null());
        unsafe { std::ptr::copy_nonoverlapping(code.as_ptr(), pointer, code.len()) };
        let mut previous = PAGE_PROTECTION_FLAGS::default();
        unsafe { VirtualProtect(pointer.cast(), 4096, PAGE_EXECUTE_READ, &mut previous) }.unwrap();
        unsafe { FlushInstructionCache(GetCurrentProcess(), Some(pointer.cast()), code.len()) }
            .unwrap();
        Self(pointer)
    }
}
impl Drop for CodePage {
    fn drop(&mut self) {
        unsafe { VirtualFree(self.0.cast(), 0, MEM_RELEASE) }.unwrap();
    }
}

#[test]
fn actual_constructor_instruction_executes_new_budget_and_only_one_byte_changes() {
    // Stack prologue, exact game budget store, read result, restore stack, ret.
    let mut code = vec![0x48, 0x83, 0xec, 0x38];
    code.extend_from_slice(&WINDOW[5..14]);
    code.extend_from_slice(&[0x48, 0x8b, 0x44, 0x24, 0x28, 0x48, 0x83, 0xc4, 0x38, 0xc3]);
    let page = CodePage::new(&code);
    let run: unsafe extern "system" fn() -> u64 = unsafe { std::mem::transmute(page.0) };
    assert_eq!(unsafe { run() }, OLD_BUDGET as u64);
    let address = unsafe { page.0.add(4 + 7) };
    unsafe { write_budget_byte(address) }.unwrap();
    assert_eq!(unsafe { run() }, NEW_BUDGET as u64);
    let actual = unsafe { std::slice::from_raw_parts(page.0, code.len()) };
    let differences: Vec<_> = actual
        .iter()
        .zip(&code)
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(differences, [11]);
    assert_eq!(actual[11], 0x10);
    let mut memory = MEMORY_BASIC_INFORMATION::default();
    unsafe {
        VirtualQuery(
            Some(page.0.cast()),
            &mut memory,
            size_of::<MEMORY_BASIC_INFORMATION>(),
        )
    };
    assert_eq!(memory.Protect, PAGE_EXECUTE_READ);
    assert!(unsafe { write_budget_byte(address) }.is_err());
    assert_eq!(unsafe { run() }, NEW_BUDGET as u64);
}

#[test]
fn private_or_unmapped_memory_is_not_accepted_as_game_image() {
    let page = CodePage::new(&[0xc3]);
    assert!(unsafe { image_bytes(page.0 as usize, 0, 1) }.is_none());
    assert!(unsafe { image_bytes(0, 0, 4096) }.is_none());
    assert!(unsafe { image_bytes(usize::MAX, 1, 4096) }.is_none());
    assert!(unsafe { image_bytes(page.0 as usize, IMAGE_LENGTH, 1) }.is_none());
}

#[test]
fn audited_executable_hash_headers_and_all_instruction_guards() {
    use pelite::pe64::{Pe, PeFile};
    use sha2::{Digest, Sha256};
    let Some(path) = std::env::var_os("ER_GAME_EXE") else {
        eprintln!("ER_GAME_EXE not set: actual executable check SKIPPED");
        return;
    };
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        "1a3547101327f65d0c76da2f9190ac0aa66871ea42bae2aecc61e11a8b597891"
    );
    let pe = PeFile::from_bytes(&bytes).unwrap();
    let mut mapped = vec![0; IMAGE_LENGTH];
    mapped[..4096].copy_from_slice(&bytes[..4096]);
    for section in pe.section_headers() {
        let raw = pe.get_section_bytes(section).unwrap();
        let start = section.VirtualAddress as usize;
        mapped[start..start + raw.len()].copy_from_slice(raw);
    }
    assert_eq!(check(&mapped), Ok(()));
    assert_eq!(u32_at(&mapped, 0x3B445F8), Some(512 * 1024));
}

#[test]
fn full_patch_on_private_image_mapping_preserves_disk_executable() {
    use sha2::{Digest, Sha256};
    use std::{fs::File, os::windows::io::AsRawHandle};
    use windows::{
        Win32::{
            Foundation::{CloseHandle, HANDLE},
            System::Memory::{
                CreateFileMappingW, FILE_MAP_READ, MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile,
                PAGE_READONLY, SEC_IMAGE, UnmapViewOfFile,
            },
        },
        core::PCWSTR,
    };
    let Some(path) = std::env::var_os("ER_GAME_EXE") else {
        eprintln!("ER_GAME_EXE not set: real image mapping check SKIPPED");
        return;
    };
    let before = Sha256::digest(std::fs::read(&path).unwrap());
    assert_eq!(
        format!("{before:x}"),
        "1a3547101327f65d0c76da2f9190ac0aa66871ea42bae2aecc61e11a8b597891"
    );
    // SEC_IMAGE gives private copy-on-write image pages. No entry point or
    // game thread is executed; the backing file is opened read-only.
    let file = File::open(&path).unwrap();
    let handle = unsafe {
        CreateFileMappingW(
            HANDLE(file.as_raw_handle()),
            None,
            PAGE_PROTECTION_FLAGS(PAGE_READONLY.0 | SEC_IMAGE.0),
            0,
            0,
            PCWSTR::null(),
        )
    }
    .unwrap();
    struct ImageMapping(HANDLE, MEMORY_MAPPED_VIEW_ADDRESS);
    impl Drop for ImageMapping {
        fn drop(&mut self) {
            if !self.1.Value.is_null() {
                unsafe { UnmapViewOfFile(self.1) }.unwrap();
            }
            unsafe { CloseHandle(self.0) }.unwrap();
        }
    }
    let mapping = ImageMapping(handle, unsafe {
        MapViewOfFile(handle, FILE_MAP_READ, 0, 0, 0)
    });
    assert!(!mapping.1.Value.is_null());
    let base = mapping.1.Value as usize;
    let original_text = unsafe { image_bytes(base, TEXT_RVA, TEXT_LENGTH) }
        .unwrap()
        .to_vec();
    assert_eq!(unsafe { apply(base) }, Ok(()));
    for edit in EDITS {
        assert_eq!(
            unsafe { image_bytes(base, edit.rva, edit.replacement.len()) }.unwrap(),
            edit.replacement
        );
    }
    let modified_text = unsafe { image_bytes(base, TEXT_RVA, TEXT_LENGTH) }.unwrap();
    let actual_changes: Vec<_> = original_text
        .iter()
        .zip(modified_text)
        .enumerate()
        .filter(|(_, (before, after))| before != after)
        .map(|(offset, _)| offset + TEXT_RVA)
        .collect();
    let mut expected_changes: Vec<_> = EDITS
        .iter()
        .flat_map(|edit| {
            edit.original
                .iter()
                .zip(edit.replacement)
                .enumerate()
                .filter(|(_, (before, after))| before != after)
                .map(move |(offset, _)| offset + edit.rva)
        })
        .collect();
    expected_changes.sort_unstable();
    assert_eq!(
        actual_changes, expected_changes,
        "no unrelated executable bytes may change"
    );
    let after = unsafe { image_bytes(base, WINDOW_RVA, WINDOW.len()) }.unwrap();
    let differences: Vec<_> = after
        .iter()
        .zip(WINDOW)
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(differences, [PATCH_RVA - WINDOW_RVA]);
    assert_eq!(after[PATCH_RVA - WINDOW_RVA], 0x10);
    assert!(unsafe { apply(base) }.is_err());
    drop(mapping);
    assert_eq!(Sha256::digest(std::fs::read(&path).unwrap()), before);
}
