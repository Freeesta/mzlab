#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)] // a test builds files by offsets and fails by panicking

use super::*;

const NONE: u32 = 0xFFFF_FFFF;

/// One directory entry: name, type (5 root, 1 storage, 2 stream), left, right and child.
fn entry(name: &str, typ: u8, left: u32, right: u32, child: u32) -> [u8; 128] {
    let mut e = [0u8; 128];
    let units: Vec<u16> = name.encode_utf16().collect();
    for (i, u) in units.iter().enumerate() {
        e[2 * i..2 * i + 2].copy_from_slice(&u.to_le_bytes());
    }
    e[0x40..0x42].copy_from_slice(&((units.len() as u16 + 1) * 2).to_le_bytes());
    e[0x42] = typ;
    e[0x44..0x48].copy_from_slice(&left.to_le_bytes());
    e[0x48..0x4C].copy_from_slice(&right.to_le_bytes());
    e[0x4C..0x50].copy_from_slice(&child.to_le_bytes());
    e[0x74..0x78].copy_from_slice(&END.to_le_bytes());
    e
}

/// A file with the FAT sectors first and then the directory, as many sectors as the entries need.
fn file(shift: u16, entries: &[[u8; 128]]) -> Vec<u8> {
    let ss = 1usize << shift;
    let per = ss / 128;
    let dirs = entries.len().div_ceil(per);
    let mut fats = 1;
    while fats * (ss / 4) < fats + dirs {
        fats += 1;
    }
    let mut h = vec![0u8; 512.max(ss)];
    h[..8].copy_from_slice(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
    h[0x1E..0x20].copy_from_slice(&shift.to_le_bytes());
    h[0x20..0x22].copy_from_slice(&6u16.to_le_bytes());
    h[0x2C..0x30].copy_from_slice(&(fats as u32).to_le_bytes());
    h[0x30..0x34].copy_from_slice(&(fats as u32).to_le_bytes());
    h[0x38..0x3C].copy_from_slice(&4096u32.to_le_bytes());
    h[0x3C..0x40].copy_from_slice(&END.to_le_bytes());
    h[0x44..0x48].copy_from_slice(&END.to_le_bytes());
    for i in 0..109 {
        let sid = if i < fats { i as u32 } else { NONE };
        h[0x4C + 4 * i..0x50 + 4 * i].copy_from_slice(&sid.to_le_bytes());
    }
    let mut fat = vec![NONE; fats * ss / 4];
    fat[..fats].fill(0xFFFF_FFFD);
    for d in 0..dirs {
        fat[fats + d] = if d + 1 < dirs {
            (fats + d + 1) as u32
        } else {
            END
        };
    }
    let mut out = h;
    out.extend(fat.iter().flat_map(|v| v.to_le_bytes()));
    let mut dir: Vec<u8> = entries.iter().flatten().copied().collect();
    dir.resize(dirs * ss, 0);
    out.extend(dir);
    out
}

#[test]
fn a_small_file_lists_its_streams_in_tree_order() {
    let f = file(
        9,
        &[
            entry("Root Entry", 5, NONE, NONE, 1),
            entry("Sub", 1, NONE, NONE, 2),
            entry("a", 2, NONE, 3, NONE),
            entry("b", 2, NONE, NONE, NONE),
        ],
    );
    let o = Ole::open(f, "x").unwrap();
    assert_eq!(o.paths_list().collect::<Vec<_>>(), ["Sub/a", "Sub/b"]);
    assert!(o.has("Sub/b") && !o.has("a"));
}

#[test]
fn a_sector_size_the_standard_does_not_allow_is_refused() {
    let mut f = file(9, &[entry("Root Entry", 5, NONE, NONE, NONE)]);
    f[0x1E..0x20].copy_from_slice(&1u16.to_le_bytes()); // 2-byte sectors: `ss / 4 - 1` would go below zero
    assert!(Ole::open(f.clone(), "x").is_err());
    f[0x1E..0x20].copy_from_slice(&24u16.to_le_bytes());
    assert!(Ole::open(f, "x").is_err());
}

#[test]
fn a_difat_chain_that_points_to_itself_ends() {
    let mut f = file(9, &[entry("Root Entry", 5, NONE, NONE, NONE)]);
    // the first DIFAT sector is sector 1 (the directory) and its last word points to itself: 4 billion turns of 127 entries before
    f[0x44..0x48].copy_from_slice(&1u32.to_le_bytes());
    f[0x48..0x4C].copy_from_slice(&u32::MAX.to_le_bytes());
    let at = 512 * 2 + 512 - 4;
    f[at..at + 4].copy_from_slice(&1u32.to_le_bytes());
    assert!(Ole::open(f, "x").is_ok());
}

#[test]
fn a_directory_that_is_one_long_chain_or_a_loop_is_walked_without_recursion() {
    // 40 000 entries, each the left child of the one before: a recursive walk needs 40 000 frames
    let n = 40_000u32;
    let mut e = vec![entry("Root Entry", 5, NONE, NONE, 1)];
    for i in 1..n {
        let left = if i + 1 < n { i + 1 } else { NONE };
        e.push(entry(&format!("s{i}"), 2, left, NONE, NONE));
    }
    let o = Ole::open(file(12, &e), "x").unwrap();
    assert_eq!(o.paths_list().count(), (n - 1) as usize);
    assert_eq!(o.paths_list().next(), Some(format!("s{}", n - 1).as_str())); // in order: the deepest left child first
                                                                             // a loop: entry 2 has entry 1 as its right child
    let looped = file(
        9,
        &[
            entry("Root Entry", 5, NONE, NONE, 1),
            entry("a", 2, NONE, 2, NONE),
            entry("b", 2, NONE, 1, NONE),
        ],
    );
    assert_eq!(
        Ole::open(looped, "x")
            .unwrap()
            .paths_list()
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
}
