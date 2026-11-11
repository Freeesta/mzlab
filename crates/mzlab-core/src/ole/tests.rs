#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)] // a test builds files by offsets and fails by panicking

use super::*;
use crate::readers::Limits;

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
    let o = Ole::open(f, "x", &Limits::held()).unwrap();
    assert_eq!(o.paths_list().collect::<Vec<_>>(), ["Sub/a", "Sub/b"]);
    assert!(o.has("Sub/b") && !o.has("a"));
}

#[test]
fn a_sector_size_the_standard_does_not_allow_is_refused() {
    let mut f = file(9, &[entry("Root Entry", 5, NONE, NONE, NONE)]);
    f[0x1E..0x20].copy_from_slice(&1u16.to_le_bytes()); // 2-byte sectors: `ss / 4 - 1` would go below zero
    assert!(Ole::open(f.clone(), "x", &Limits::held()).is_err());
    f[0x1E..0x20].copy_from_slice(&24u16.to_le_bytes());
    assert!(Ole::open(f, "x", &Limits::held()).is_err());
}

#[test]
fn a_difat_chain_that_points_to_itself_ends() {
    let mut f = file(9, &[entry("Root Entry", 5, NONE, NONE, NONE)]);
    // the first DIFAT sector is sector 1 (the directory) and its last word points to itself: 4 billion turns of 127 entries before
    f[0x44..0x48].copy_from_slice(&1u32.to_le_bytes());
    f[0x48..0x4C].copy_from_slice(&u32::MAX.to_le_bytes());
    let at = 512 * 2 + 512 - 4;
    f[at..at + 4].copy_from_slice(&1u32.to_le_bytes());
    assert!(Ole::open(f, "x", &Limits::held()).is_ok());
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
    let o = Ole::open(file(12, &e), "x", &Limits::held()).unwrap();
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
        Ole::open(looped, "x", &Limits::held())
            .unwrap()
            .paths_list()
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
}

#[test]
fn sector_numbers_at_the_top_of_the_range_read_nothing() {
    let mut big = entry("big", 2, NONE, NONE, NONE);
    big[0x74..0x78].copy_from_slice(&0xFFFF_FFFDu32.to_le_bytes());
    big[0x78..0x7C].copy_from_slice(&0x10000u32.to_le_bytes());
    let o = Ole::open(
        file(9, &[entry("Root Entry", 5, NONE, NONE, 1), big]),
        "x",
        &Limits::held(),
    )
    .unwrap();
    assert_eq!(o.read("big"), Some(vec![]));
    // `sector` alone, whatever its caller checks: `sid + 1` must not wrap to sector 0 on a 32-bit target
    assert!(o.sector(u32::MAX).is_empty() && o.sector(0xFFFF_FFFE).is_empty());
}

/// Storages nested `n` deep, each the only child of the one above, and a stream at the bottom.
fn nested(n: u32) -> Vec<u8> {
    let mut e = vec![entry("Root Entry", 5, NONE, NONE, 1)];
    for i in 1..n {
        e.push(entry(&format!("d{i}"), 1, NONE, NONE, i + 1));
    }
    e.push(entry("s", 2, NONE, NONE, NONE));
    file(12, &e)
}

#[test]
fn storages_nested_past_the_depth_limit_are_refused_not_walked() {
    // each level copied the path above it: 30 000 levels (a 3.8 MB file) cost 450 million copied names and the memory of the machine
    let t = std::time::Instant::now();
    let e = Ole::open(nested(30_000), "x", &Limits::held())
        .err()
        .unwrap();
    assert_eq!(e.key, "err.dam.unreadable");
    assert!(t.elapsed().as_secs() < 5, "{:?}", t.elapsed());
    // the limit itself is a valid file
    let o = Ole::open(
        nested(Limits::held().profondita as u32),
        "x",
        &Limits::held(),
    )
    .unwrap();
    assert_eq!(o.paths_list().count(), 1);
}

#[test]
fn a_file_past_the_byte_limit_is_a_fixed_limit_and_a_directory_past_the_entries_a_memory_error() {
    let f = file(9, &[entry("Root Entry", 5, NONE, NONE, NONE)]);
    let small = Limits {
        byte: 100,
        ..Limits::held()
    };
    assert_eq!(
        Ole::open(f.clone(), "x", &small).err().unwrap().key,
        "error.fixedLimit"
    );
    let few = Limits {
        entries: 1,
        ..Limits::held()
    };
    assert_eq!(
        Ole::open(f, "x", &few).err().unwrap().key,
        "error.memoryBudget"
    );
}

#[test]
fn a_mini_fat_that_loops_reads_no_more_than_the_stream_size() {
    // root entry with a ministream of one sector (sector `fats + 1`), a mini FAT (sector `fats + 2`) of 128 entries that all point to mini sector 0,
    // and a stream of 100 bytes in the ministream: the chain loops on mini sector 0 and used to copy 64 bytes per mini FAT entry
    let mut root = entry("Root Entry", 5, NONE, NONE, 1);
    let mut s = entry("s", 2, NONE, NONE, NONE);
    s[0x74..0x78].copy_from_slice(&0u32.to_le_bytes());
    s[0x78..0x7C].copy_from_slice(&100u32.to_le_bytes());
    let mut f = file(9, &[root, s]);
    let fats = u32::from_le_bytes(f[0x2C..0x30].try_into().unwrap()) as usize;
    let (ms, mf) = (fats as u32 + 1, fats as u32 + 2);
    root[0x74..0x78].copy_from_slice(&ms.to_le_bytes());
    root[0x78..0x7C].copy_from_slice(&512u32.to_le_bytes());
    let dir_at = 512 * (1 + fats);
    f[dir_at..dir_at + 128].copy_from_slice(&root);
    f[0x3C..0x40].copy_from_slice(&mf.to_le_bytes());
    let fat_at = |sid: u32| 512 + 4 * sid as usize;
    f[fat_at(ms)..fat_at(ms) + 4].copy_from_slice(&END.to_le_bytes());
    f[fat_at(mf)..fat_at(mf) + 4].copy_from_slice(&END.to_le_bytes());
    f.extend(vec![7u8; 512]);
    f.extend(vec![0u8; 512]);
    let o = Ole::open(f, "x", &Limits::held()).unwrap();
    let b = o.read("s").unwrap();
    assert_eq!(b, vec![7u8; 100]);
    assert!(
        b.capacity() < 100 + 2 * 64,
        "the chain was followed past the size: {}",
        b.capacity()
    );
}
