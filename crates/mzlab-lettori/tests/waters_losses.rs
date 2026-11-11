#![cfg(feature = "waters")]
//! `Reader::losses` of the Waters reader on the folder of `flags_folder` (two scanning functions), changed one member at a time: the scans the index lists and the
//! reader does not read are counted, with the key of the cause (a DAT cut short, a function without its DAT, a record width that is not decoded).
mod common_waters;
use common_waters::*;
use mzlab_core::readers::{Limits, Losses, TRUNCATED};

fn losses(files: Vec<(String, Vec<u8>)>) -> Losses {
    let mut l = mzlab_lettori::open_bare(file_set(files), &Limits::sito())
        .unwrap_or_else(|e| panic!("{}", e.key));
    let n = l.index(0).unwrap().len();
    let x = l.losses(0).unwrap();
    assert_eq!(x.read, n, "read is the index");
    x
}

fn member(files: &[(String, Vec<u8>)], name: &str) -> Vec<u8> {
    files.iter().find(|f| f.0 == name).unwrap().1.clone()
}

fn scans(files: &[(String, Vec<u8>)], k: u32) -> usize {
    member(files, &format!("_FUNC{k:03}.IDX")).len() / 22
}

#[test]
fn a_whole_folder_loses_nothing() {
    let f = flags_folder(0x4C);
    let n = scans(&f, 1) + scans(&f, 2);
    assert_eq!(losses(f), Losses::complete(n));
}

#[test]
fn a_scanning_function_without_its_dat_is_counted() {
    let f = flags_folder(0x4C);
    let (n1, n2) = (scans(&f, 1), scans(&f, 2));
    let f: Vec<_> = f.into_iter().filter(|x| x.0 != "_FUNC002.DAT").collect();
    let x = losses(f);
    assert_eq!(
        (x.declared, x.read, x.cause),
        (n1 + n2, n1, "warn.file.memberMissing")
    );
}

#[test]
fn a_record_width_not_decoded_is_counted() {
    // function 2 rewritten with 10-byte records (none of 6, 8, 12): the first 10 bytes of each 12-byte record, offsets moved to match
    let mut f = flags_folder(0x4C);
    let (n1, n2) = (scans(&f, 1), scans(&f, 2));
    let (dat, mut idx) = (member(&f, "_FUNC002.DAT"), member(&f, "_FUNC002.IDX"));
    let per = dat.len() / n2;
    let mut d10 = Vec::new();
    for k in 0..n2 {
        for r in dat[k * per..(k + 1) * per].chunks(12) {
            d10.extend_from_slice(&r[..10]);
        }
        let off = (k * per / 12 * 10) as u32;
        idx[k * 22..k * 22 + 4].copy_from_slice(&off.to_le_bytes());
    }
    for (name, b) in [("_FUNC002.DAT", d10), ("_FUNC002.IDX", idx)] {
        f.iter_mut().find(|x| x.0 == name).unwrap().1 = b;
    }
    let x = losses(f);
    assert_eq!(
        (x.declared, x.read, x.cause),
        (n1 + n2, n1, "warn.file.recordWidth")
    );
}

#[test]
fn scans_past_a_cut_dat_keep_the_truncated_key() {
    let mut f = flags_folder(0x4C);
    let n = scans(&f, 1) + scans(&f, 2);
    let dat = member(&f, "_FUNC001.DAT");
    f.iter_mut().find(|x| x.0 == "_FUNC001.DAT").unwrap().1 = dat[..dat.len() - 8].to_vec();
    let x = losses(f);
    assert_eq!((x.declared, x.read, x.cause), (n, n - 1, TRUNCATED));
}
