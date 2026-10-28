//! A damaged `.mzidx` is `error.badIndex` when it is opened: metadata whose sections reach outside the file or overflow, keys that name no section,
//! a file cut short. Never a panic or an allocation of the size the damaged numbers ask for. The metadata is rewritten at the end of a good index.
#![allow(clippy::indexing_slicing)]
#![allow(clippy::disallowed_methods)]

mod lettore_finto;

use std::io::Cursor;

use lettore_finto::Finto;
use mzlab_core::idxread::Index;
use mzlab_core::{mzidx, upload};

fn good() -> Vec<u8> {
    let mut l = Finto::vario(300, 200, 31337);
    let p = upload::prepare_lettore(&mut l, 0, &[], "x.mzML").unwrap();
    let mut out = Cursor::new(Vec::new());
    mzidx::build_lettore(&p.table, &mut l, 0, &mut out, mzidx::DEFAULT_BUDGET).unwrap();
    out.into_inner()
}

fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().unwrap())
}

/// The index with its metadata JSON changed by `f` and moved to the end of the file (the header points at the new place and size).
fn with_meta(b: &[u8], f: impl Fn(String) -> String) -> Vec<u8> {
    let (off, len) = (u64_at(b, 16) as usize, u64_at(b, 24) as usize);
    let meta = f(String::from_utf8(b[off..off + len].to_vec()).unwrap());
    let mut out = b.to_vec();
    let at = out.len() as u64;
    out.extend_from_slice(meta.as_bytes());
    out[16..24].copy_from_slice(&at.to_le_bytes());
    out[24..32].copy_from_slice(&(meta.len() as u64).to_le_bytes());
    let size = out.len() as u64;
    out[48..56].copy_from_slice(&size.to_le_bytes());
    out
}

/// Replaces the number after the first `"key":` with `value`.
fn set(meta: &str, key: &str, value: &str) -> String {
    let pat = format!("\"{key}\":");
    let at = meta.find(&pat).unwrap_or_else(|| panic!("no {key}")) + pat.len();
    let end = at + meta[at..].find(|c: char| !c.is_ascii_digit()).unwrap();
    format!("{}{value}{}", &meta[..at], &meta[end..])
}

fn open(b: Vec<u8>) -> mzlab_core::Result<Index<Vec<u8>>> {
    let size = b.len() as u64;
    Index::open(b, size)
}

#[test]
fn a_good_index_with_its_metadata_moved_opens() {
    let b = good();
    let moved = with_meta(&b, |m| m);
    let (a, z) = (open(b).unwrap(), open(moved).unwrap());
    assert_eq!(a.sections.len(), z.sections.len());
}

#[test]
fn sections_outside_the_file_are_a_bad_index() {
    let b = good();
    let size = b.len();
    for (key, value) in [
        ("n_peaks", "18446744073709551615"),
        ("n_peaks", "99999999999"),
        ("n_scans", "18446744073709551615"),
        ("n_scans", "99999999999"),
        ("n_fence", "18446744073709551615"),
        ("rt", "18446744073709551615"),
        ("rt", &(size * 2).to_string()),
        ("fence", &(size * 2).to_string()),
        ("mz", "18446744073709551615"),
        ("inten", &(size * 2).to_string()),
        ("pos", &(size * 2).to_string()),
    ] {
        let err = open(with_meta(&b, |m| set(&m, key, value)))
            .err()
            .unwrap_or_else(|| panic!("{key}={value} opened"));
        assert_eq!(err.key, "error.badIndex", "{key}={value}");
    }
}

#[test]
fn a_key_that_names_no_section_is_a_bad_index() {
    let b = good();
    let err = open(with_meta(&b, |m| {
        m.replacen("\"keys\":{", "\"keys\":{\"9/all\":99,", 1)
    }))
    .err()
    .unwrap();
    assert_eq!(err.key, "error.badIndex");
}

#[test]
fn a_cut_file_is_a_bad_index() {
    let b = good();
    for cut in [0, 10, 63, 64, 100, b.len() / 2, b.len() - 1] {
        let err = open(b[..cut].to_vec())
            .err()
            .unwrap_or_else(|| panic!("cut {cut} opened"));
        assert_eq!(err.key, "error.badIndex", "cut {cut}");
    }
}
