use super::super::tests::CodePage;
use super::*;

fn store_pointer(buffer: &mut [u8], offset: usize, pointer: usize) {
    buffer[offset..offset + 8].copy_from_slice(&pointer.to_le_bytes());
}

fn activation_code(window: &[u8]) -> CodePage {
    // Preserve Win64 nonvolatile registers; original branch offsets stay intact.
    let mut code = vec![
        0x57, 0x41, 0x56, 0x48, 0x83, 0xec, 0x60, 0x48, 0x8b, 0xf9, 0x4c, 0x8b, 0xf2,
    ];
    code.resize(0x40, 0x90);
    code.extend_from_slice(window);
    code.extend_from_slice(&[0x8b, 0xc2]);
    let tail = [0x48, 0x83, 0xc4, 0x60, 0x41, 0x5e, 0x5f, 0xc3];
    code.extend_from_slice(&tail);
    code.resize(0x40 + 0x134, 0x90);
    code.extend_from_slice(&[0xb8, 0xff, 0xff, 0xff, 0xff]);
    code.extend_from_slice(&tail);
    CodePage::new(&code)
}

fn selection_code(window: &[u8]) -> CodePage {
    let mut code = vec![0x41, 0x55, 0x4c, 0x8b, 0xea];
    code.extend_from_slice(window);
    code.extend_from_slice(&[0x41, 0x5d, 0xc3]);
    CodePage::new(&code)
}

#[test]
fn native_original_reproduces_recorded_negative_index() {
    let page = activation_code(EDITS[0].original);
    let run: unsafe extern "system" fn(*const u8, *const u8) -> i32 =
        unsafe { std::mem::transmute(page.0) };
    let mut clip = [0u8; 0x160];
    clip[0xc6..0xc8].copy_from_slice(&35781u16.to_le_bytes());
    let graph = [0u8; 0x118];
    let mut context = [0u8; 0x20];
    store_pointer(&mut context, 0x10, graph.as_ptr() as usize);
    assert_eq!(unsafe { run(clip.as_ptr(), context.as_ptr()) }, -29755);
}

#[test]
fn native_corrected_boundary_regression() {
    let activation_page = activation_code(EDITS[0].replacement);
    let selection_page = selection_code(EDITS[1].replacement);
    let activate: unsafe extern "system" fn(*const u8, *const u8) -> i32 =
        unsafe { std::mem::transmute(activation_page.0) };
    let select: unsafe extern "system" fn(*const u8, *const u8) -> i32 =
        unsafe { std::mem::transmute(selection_page.0) };
    let mut clip = [0u8; 0x160];
    let mut graph = [0u8; 0x118];
    let mut context = [0u8; 0x20];
    let mut mapping = [0u8; 0x20];
    let mut indices: Vec<i32> = (0..65534).map(|i| i + 70000).collect();
    indices[12] = -1;
    indices[13] = -2;
    store_pointer(&mut mapping, 0x18, indices.as_ptr() as usize);
    store_pointer(&mut context, 0x10, graph.as_ptr() as usize);
    for remapped in [false, true] {
        let pointer = if remapped {
            mapping.as_ptr() as usize
        } else {
            0
        };
        store_pointer(&mut graph, 0x110, pointer);
        for word in 0..=u16::MAX {
            clip[0xc6..0xc8].copy_from_slice(&word.to_le_bytes());
            let expected = match word {
                65534 => -2,
                65535 => -1,
                _ if remapped => indices[word as usize],
                _ => i32::from(word),
            };
            assert_eq!(
                unsafe { activate(clip.as_ptr(), context.as_ptr()) },
                expected,
                "activation word={word} remapped={remapped}"
            );
            assert_eq!(
                unsafe { select(clip.as_ptr(), pointer as *const u8) },
                expected,
                "selection word={word} remapped={remapped}"
            );
        }
    }
}

#[test]
fn native_name_table_defaults_and_equality() {
    let mut default_code = vec![0x56, 0x48, 0x8b, 0xf1];
    default_code.extend_from_slice(EDITS[2].replacement);
    default_code.extend_from_slice(&[0x49, 0x8b, 0xc0, 0x5e, 0xc3]);
    let default_page = CodePage::new(&default_code);
    let default: unsafe extern "system" fn(*const u8) -> u64 =
        unsafe { std::mem::transmute(default_page.0) };
    let mut compare_code = vec![0x53, 0x56, 0x48, 0x8b, 0xd9, 0x48, 0x8b, 0xf2];
    compare_code.extend_from_slice(EDITS[3].replacement);
    compare_code.extend_from_slice(&[0x0f, 0x94, 0xc0, 0x0f, 0xb6, 0xc0, 0x5e, 0x5b, 0xc3]);
    let compare_page = CodePage::new(&compare_code);
    let equal: unsafe extern "system" fn(*const u8, *const u8) -> u32 =
        unsafe { std::mem::transmute(compare_page.0) };
    let mut table = [0u8; 0x68];
    let mut clip = [0u8; 0x160];
    for count in 0..65534u32 {
        table[0x60..0x64].copy_from_slice(&count.to_le_bytes());
        assert_eq!(unsafe { default(table.as_ptr()) }, u64::from(count));
        for raw in [count as u16, 65534, 65535] {
            clip[0xc6..0xc8].copy_from_slice(&raw.to_le_bytes());
            assert_eq!(
                unsafe { equal(clip.as_ptr(), table.as_ptr()) },
                u32::from(u32::from(raw) == count)
            );
        }
    }
}

#[test]
fn native_unchanged_teardown_distinguishes_both_sentinels() {
    let mut code = vec![
        0x53, 0x57, 0x48, 0x83, 0xec, 0x28, 0x48, 0x8b, 0xd9, 0x33, 0xff,
    ];
    let start = code.len();
    code.extend_from_slice(TEARDOWN);
    code.extend_from_slice(&[0x48, 0x83, 0xc4, 0x28, 0x5f, 0x5b, 0xc3]);
    // Only replace external calls with counters in this isolated harness.
    for (rva, stub) in [(0x146184c, 0x200usize), (0x146186f, 0x210)] {
        let offset = start + rva - 0x1461836;
        assert_eq!(code[offset], 0xe8);
        let displacement = (stub as i32 - offset as i32 - 5).to_le_bytes();
        code[offset + 1..offset + 5].copy_from_slice(&displacement);
    }
    code.resize(0x200, 0x90);
    code.extend_from_slice(&[0xff, 1, 0xc3]);
    code.resize(0x210, 0x90);
    code.extend_from_slice(&[0xff, 0x41, 4, 0xc3]);
    let page = CodePage::new(&code);
    let run: unsafe extern "system" fn(*mut u8) = unsafe { std::mem::transmute(page.0) };
    for raw in [0u16, 32767, 32768, 35781, 65533, 65534, 65535] {
        let mut clip = [0u8; 0x160];
        let mut calls = [0u32; 2];
        clip[0xc6..0xc8].copy_from_slice(&raw.to_le_bytes());
        store_pointer(&mut clip, 0xd8, calls.as_mut_ptr() as usize);
        unsafe { run(clip.as_mut_ptr()) };
        let actual_index = u16::from_le_bytes(clip[0xc6..0xc8].try_into().unwrap());
        assert_eq!(actual_index, if raw == 65534 { 65535 } else { raw });
        assert_eq!(
            calls,
            match raw {
                65534 => [1, 0],
                65535 => [0, 0],
                _ => [0, 1],
            }
        );
        let control = usize::from_le_bytes(clip[0xd8..0xe0].try_into().unwrap());
        assert_eq!(control == 0, raw < 65534);
    }
}
