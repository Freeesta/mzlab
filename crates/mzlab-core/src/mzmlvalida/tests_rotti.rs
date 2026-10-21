//! Files broken on purpose: each one has its own error and no other.
use super::tests::{errors, report, show_all, written};
use crate::mzmlwrite::Opzioni;
use sha1::{Digest as _, Sha1};

fn indexed() -> Vec<u8> {
    written(true, Opzioni::default(), true)
}

/// Replaces the first `from` with `to` of the same length.
fn swap(mut b: Vec<u8>, from: &str, to: &str) -> Vec<u8> {
    assert_eq!(from.len(), to.len(), "a same-length edit keeps the offsets");
    let at = b
        .windows(from.len())
        .position(|w| w == from.as_bytes())
        .unwrap_or_else(|| panic!("{from} is not in the file"));
    b[at..at + from.len()].copy_from_slice(to.as_bytes());
    b
}

/// Writes the SHA-1 of the file into `fileChecksum`, so that an edit leaves the checksum right.
fn rehash(mut b: Vec<u8>) -> Vec<u8> {
    let tag = b"<fileChecksum>";
    let at = b.windows(tag.len()).position(|w| w == tag).unwrap() + tag.len();
    let hex: String = Sha1::digest(&b[..at])
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect();
    b[at..at + 40].copy_from_slice(hex.as_bytes());
    b
}

/// The text after `key` with its last digit changed (`10` -> `11`, `19` -> `10`).
fn bump(b: Vec<u8>, key: &str) -> Vec<u8> {
    let at = b
        .windows(key.len())
        .position(|w| w == key.as_bytes())
        .unwrap()
        + key.len();
    let end = at + b[at..].iter().position(|c| !c.is_ascii_digit()).unwrap();
    let mut out = b;
    out[end - 1] = b'0' + (out[end - 1] - b'0' + 1) % 10;
    out
}

/// The first digit of the stored checksum changed.
fn flip_checksum(mut b: Vec<u8>) -> Vec<u8> {
    let tag = b"<fileChecksum>";
    let at = b.windows(tag.len()).position(|w| w == tag).unwrap() + tag.len();
    b[at] = if b[at] == b'0' { b'1' } else { b'0' };
    b
}

/// `from..=to` (inclusive of `to`) replaced by spaces.
fn blank(mut b: Vec<u8>, from: &str, to: &str) -> Vec<u8> {
    let a = b
        .windows(from.len())
        .position(|w| w == from.as_bytes())
        .unwrap();
    let z = a
        + b[a..]
            .windows(to.len())
            .position(|w| w == to.as_bytes())
            .unwrap()
        + to.len();
    b[a..z].fill(b' ');
    b
}

#[test]
fn twelve_broken_files_each_with_its_own_error() {
    let cases: Vec<(&str, Vec<u8>, &str)> = vec![
        (
            "offset of a spectrum",
            rehash(bump(indexed(), "<offset idRef=\"scan=2\">")),
            "index.offset",
        ),
        (
            "SHA-1 of the file",
            flip_checksum(indexed()),
            "index.checksum",
        ),
        (
            "indexListOffset",
            rehash(bump(indexed(), "<indexListOffset>")),
            "index.listOffset",
        ),
        (
            "invented accession",
            rehash(swap(indexed(), "MS:1000285", "MS:1999285")),
            "accession",
        ),
        (
            "name that does not match",
            rehash(swap(
                indexed(),
                "name=\"total ion current\"",
                "name=\"total ion curreNt\"",
            )),
            "name",
        ),
        (
            "false count",
            rehash(swap(
                indexed(),
                "<spectrumList count=\"3\"",
                "<spectrumList count=\"4\"",
            )),
            "count",
        ),
        (
            "false defaultArrayLength",
            rehash(swap(
                indexed(),
                "defaultArrayLength=\"3\"",
                "defaultArrayLength=\"4\"",
            )),
            "arrayLength",
        ),
        (
            "m/z not in order",
            written(true, Opzioni::default(), false),
            "mz.order",
        ),
        (
            "no profile or centroid term",
            rehash(blank(
                indexed(),
                "<cvParam cvRef=\"MS\" accession=\"MS:1000127\"",
                "/>",
            )),
            "required.term",
        ),
        (
            "unit the term does not take",
            rehash(swap(
                indexed(),
                "unitCvRef=\"UO\" unitAccession=\"UO:0000031\" unitName=\"minute\"",
                "unitCvRef=\"MS\" unitAccession=\"MS:1000040\" unitName=\"m/z\"   ",
            )),
            "unit.notAllowed",
        ),
        (
            "cvRef not in the cvList",
            rehash(swap(
                indexed(),
                "cvRef=\"MS\" accession=\"MS:1000285\"",
                "cvRef=\"XS\" accession=\"MS:1000285\"",
            )),
            "cvRef",
        ),
        (
            "a required element missing",
            rehash(blank(
                indexed(),
                "<dataProcessingList",
                "</dataProcessingList>",
            )),
            "required",
        ),
        (
            "encodedLength",
            rehash(bump(indexed(), "encodedLength=\"")),
            "encodedLength",
        ),
        (
            "index that does not count",
            rehash(swap(
                indexed(),
                "<spectrum index=\"1\"",
                "<spectrum index=\"2\"",
            )),
            "index",
        ),
        ("file cut short", indexed()[..4000].to_vec(), "xml"),
    ];
    assert!(cases.len() >= 12);
    for (what, bytes, code) in cases {
        let r = report(&bytes);
        assert_eq!(errors(&r), [code], "{what}:\n{}", show_all(&r));
    }
}
