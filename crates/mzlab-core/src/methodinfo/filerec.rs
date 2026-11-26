//! The file-information strings of a method (the `.dam` method path, the date stamp, the software and the computer), read from the printable UTF-16
//! runs of `AcqMethodFileInfoStm` and `FileRec_Str`.

use super::ascii;
use crate::ole::Ole;

/// Runs of at least four printable ASCII characters in UTF-16 (`_strings`).
fn strings(d: &[u8]) -> Vec<String> {
    let (mut out, mut i) = (Vec::new(), 0);
    while i + 1 < d.len() {
        let mut j = i;
        while j + 1 < d.len() && (0x20..=0x7e).contains(&d[j]) && d[j + 1] == 0 {
            j += 2;
        }
        if (j - i) / 2 >= 4 {
            out.push(ascii(&d[i..j]));
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

const DAYS: [&str; 7] = [
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
];

/// `[A-Z]:\\.*?\.dam`
fn dam_path(t: &str) -> Option<&str> {
    let b = t.as_bytes();
    (0..b.len().saturating_sub(2))
        .find(|&i| b[i].is_ascii_uppercase() && b[i + 1] == b':' && b[i + 2] == b'\\')
        .and_then(|i| t[i + 3..].find(".dam").map(|j| &t[i..i + 3 + j + 4]))
}

/// `[A-Z][a-z]+day .*?[AP]M`
fn stamp(t: &str) -> Option<&str> {
    let b = t.as_bytes();
    for i in 0..b.len() {
        if !b[i].is_ascii_uppercase() {
            continue;
        }
        let run = b[i + 1..]
            .iter()
            .take_while(|c| c.is_ascii_lowercase())
            .count();
        let Some(k) = (1..=run)
            .rev()
            .find(|&k| b[i + 1 + k..].starts_with(b"day "))
        else {
            continue;
        };
        let from = i + 1 + k + 4;
        if let Some(j) = (from..b.len().saturating_sub(1))
            .find(|&j| (b[j] == b'A' || b[j] == b'P') && b[j + 1] == b'M')
        {
            return Some(&t[i..j + 2]);
        }
    }
    None
}

pub(super) fn file_info(ole: &Ole, base: &str) -> Vec<(&'static str, String)> {
    let mut f: Vec<(&'static str, String)> = Vec::new();
    for fname in [
        format!("{base}AcqMethodFileInfoStm"),
        "FileRec_Str".to_string(),
    ] {
        let Some(d) = ole.read(&fname) else { continue };
        for t in strings(&d) {
            let tl = t.to_lowercase();
            let has = |k: &str| f.iter().any(|x| x.0 == k);
            if tl.contains(".dam") && !has("metodo") {
                f.push(("metodo", dam_path(&t).unwrap_or(&t).to_string()));
            } else if DAYS.iter().any(|d| tl.contains(d)) {
                if !has("data") {
                    f.push(("data", stamp(&t).unwrap_or(&t).replace(": ", ":")));
                }
            } else if tl.starts_with("analyst") {
                if !has("software") {
                    f.push(("software", t.trim().to_string()));
                }
            } else if (t.starts_with("LAB_") || t.ends_with("-PC")) && !has("computer") {
                f.push(("computer", t.clone()));
            }
        }
        if !f.is_empty() {
            break;
        }
    }
    f
}
