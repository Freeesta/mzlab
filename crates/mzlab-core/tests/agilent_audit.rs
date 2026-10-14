//! Audit of loss for the Agilent `.D` reader (provisional form of the audit of every reader): an inventory of the raw folder made WITHOUT the reader's code
//! (a plain scan of the text of the XML and INI files, the names of the files, the fields of the record layout), and a check that every item of it
//! is somewhere in what the reader gives (`AgilentD::metadata`, `ScanRec`, `extra_names`), is declared undecoded with its SHA-1, or is listed in
//! `agilent_audit/scarti.txt` with the reason. `agilent_audit/noti.txt` lists, per file, how many items the reader gives: if a known one disappears the
//! test fails (write it again with `AUDIT_WRITE=1`). Real folders need the data repository (`MZLAB_DATI`); the synthetic test always runs.

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mzlab_core::agilent::scan::{mapped_fields, Layout};
use mzlab_core::agilent::{meta, AgilentD, Files};

fn dati() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .ok()?;
    let mut cands: Vec<PathBuf> = std::env::var_os("MZLAB_DATI")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    cands.extend(
        root.ancestors()
            .skip(1)
            .take(3)
            .map(|up| up.join("mzlab-dati")),
    );
    cands.into_iter().find(|p| p.join("GC-MS").is_dir())
}

/// Every file of the folder, as a path relative to it.
fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, base, out);
        } else {
            out.push(
                p.strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
}

/// The values of an XML text by a plain scan: the text between `>` and `<`, and the values of the attributes. No parser.
fn xml_values(text: &str) -> BTreeSet<String> {
    let mut v = BTreeSet::new();
    for part in text.trim_start_matches('\u{feff}').split('<') {
        let (tag, after) = part.split_once('>').unwrap_or((part, ""));
        if !tag.starts_with('?') && !tag.starts_with('!') && !tag.starts_with('/') {
            for q in tag.split('"').skip(1).step_by(2) {
                v.insert(q.trim().to_string());
            }
        }
        v.insert(after.trim().to_string());
    }
    v.remove("");
    v.into_iter().map(|x| unescape(&x)).collect()
}

/// Text without its white space: only the characters are compared, not how the line ends were written.
fn squeeze(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// The five entities and the line ends of XML, as a parser gives the text.
fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find("&#") {
        out.push_str(&rest[..i]);
        let tail = &rest[i + 2..];
        let end = tail.find(';').unwrap_or(0);
        let num = &tail[..end];
        let code = num.strip_prefix('x').map_or_else(
            || num.parse::<u32>().ok(),
            |h| u32::from_str_radix(h, 16).ok(),
        );
        match code.and_then(char::from_u32) {
            Some(c) if end > 0 => out.push(c),
            _ => out.push_str("&#"),
        }
        rest = if end > 0 && code.is_some() {
            &tail[end + 1..]
        } else {
            tail
        };
    }
    out.push_str(rest);
    out.replace("\r\n", "\n")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// The text of a file: UTF-16 with a byte order mark is turned into characters, anything else is read as UTF-8.
fn plain_text(b: &[u8]) -> String {
    match b {
        [0xff, 0xfe, r @ ..] => String::from_utf16_lossy(
            &r.chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect::<Vec<_>>(),
        ),
        _ => String::from_utf8_lossy(b).to_string(),
    }
}

fn ini_values(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|l| l.split_once('='))
        .map(|(_, v)| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect()
}

/// The path with the name of the method folder (`<name>.M`) replaced by `*.M`, so that the list has no names of methods.
fn generic(path: &str) -> String {
    path.split('/')
        .map(|p| if p.ends_with(".M") { "*.M" } else { p })
        .collect::<Vec<_>>()
        .join("/")
}

fn listed(file: &str) -> Vec<(String, String)> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/agilent_audit")
        .join(file);
    std::fs::read_to_string(p)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.starts_with('#') && l.contains('|'))
        .filter_map(|l| l.split_once('|'))
        .map(|(a, b)| (a.trim().into(), b.trim().into()))
        .collect()
}

struct Loaded {
    files: Vec<(String, Vec<u8>)>,
}

impl Loaded {
    fn new(folder: &Path) -> Loaded {
        let mut names = Vec::new();
        walk(folder, folder, &mut names);
        names.sort();
        Loaded {
            files: names
                .into_iter()
                .map(|n| {
                    let b = std::fs::read(folder.join(&n)).unwrap();
                    (n, b)
                })
                .collect(),
        }
    }

    fn get(&self, n: &str) -> Option<&[u8]> {
        self.files.iter().find(|f| f.0 == n).map(|f| f.1.as_slice())
    }

    /// The reader's input: the small files by their place, the others as `others` (all but the data files and the folders of the analysis methods).
    fn open(&self, name: &str) -> AgilentD {
        let main = [
            "AcqData/MSScan.bin",
            "AcqData/MSPeak.bin",
            "AcqData/MSScan.xsd",
            "AcqData/MSTS.xml",
            "AcqData/Devices.xml",
            "AcqData/Contents.xml",
            "AcqData/MSTree2.bin",
            "acqmeth.txt",
        ];
        let others: Vec<(&str, &[u8])> = self
            .files
            .iter()
            .filter(|(n, _)| {
                !main.contains(&n.as_str()) && n != "TEMPBASE" && n != "AcqData/MSIndex.bin"
            })
            .map(|(n, b)| (n.as_str(), b.as_slice()))
            .collect();
        let f = Files {
            ms_scan: self.get(main[0]).unwrap(),
            data_ms: self.get("data.ms"),
            ms_peak_len: self.get(main[1]).unwrap().len() as u64,
            ms_scan_xsd: self.get(main[2]),
            ms_ts: self.get(main[3]),
            devices: self.get(main[4]),
            contents: self.get(main[5]),
            ms_tree: self.get(main[6]),
            acqmeth: self.get(main[7]),
            others: &others,
        };
        AgilentD::open(name, &f).unwrap()
    }
}

/// What is in the folder and not in the output.
fn lost(l: &Loaded, d: &AgilentD) -> Vec<String> {
    let scarti = listed("scarti.txt");
    let mut lost = Vec::new();
    for (name, bytes) in &l.files {
        if scarti.iter().any(|(p, _)| name.starts_with(p.as_str())) {
            continue;
        }
        let items: Vec<&meta::Item> = d.metadata.iter().filter(|i| i.origin == *name).collect();
        if items.is_empty() {
            lost.push(format!("{name}: no item at all"));
            continue;
        }
        let text = plain_text(bytes);
        let values = if name.ends_with(".xml") {
            xml_values(&text)
        } else if name.to_lowercase().ends_with(".ini") {
            ini_values(&text)
        } else {
            BTreeSet::new()
        };
        let have: BTreeSet<String> = items
            .iter()
            .flat_map(|i| [squeeze(&i.value), squeeze(&i.name)])
            .collect();
        lost.extend(
            values
                .iter()
                .filter(|v| !have.contains(&squeeze(v)))
                .map(|v| format!("{name}: value of {} characters", v.chars().count())),
        );
    }
    lost
}

#[test]
fn nothing_of_the_real_folders_is_lost() {
    let Some(data) = dati() else {
        eprintln!("SKIP agilent_audit: no data repository (MZLAB_DATI)");
        return;
    };
    let mut folders: Vec<PathBuf> = std::fs::read_dir(data.join("GC-MS"))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "D"))
        .collect();
    folders.sort();
    let known = listed("noti.txt");
    let mut now: Vec<std::collections::BTreeMap<String, usize>> = Vec::new();
    for f in &folders {
        let l = Loaded::new(f);
        let d = l.open("x.D");
        let gone = lost(&l, &d);
        assert!(gone.is_empty(), "lost: {gone:?}");
        // the fields of the record layout: mapped or kept in extra
        let xsd = Layout::from_xsd(&String::from_utf8_lossy(
            l.get("AcqData/MSScan.xsd").unwrap(),
        ))
        .unwrap();
        for n in xsd.field_names() {
            assert!(
                mapped_fields().contains(&n.as_str()) || d.extra_names.contains(&n),
                "record field {n} is neither mapped nor kept"
            );
        }
        // what is not decoded is declared with its size and SHA-1
        assert!(
            d.metadata
                .iter()
                .any(|i| i.origin == "AcqData/MSTree2.bin" && i.name == "undecoded sha1")
                && d.metadata
                    .iter()
                    .any(|i| i.origin == "AcqData/MSScan.bin#header" && i.name == "undecoded sha1")
        );
        let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
        for i in &d.metadata {
            *counts.entry(generic(&i.origin)).or_default() += 1;
        }
        for (o, n) in &known {
            let present = l.files.iter().any(|f| generic(&f.0) == *o);
            let c = counts.get(o).copied().unwrap_or(0);
            assert!(
                !present || c >= n.parse::<usize>().unwrap_or(0),
                "known file {o} gives {c} items, it gave {n}"
            );
        }
        now.push(counts);
    }
    if std::env::var_os("AUDIT_WRITE").is_some() {
        let mut best: std::collections::BTreeMap<String, usize> = Default::default();
        for counts in &now {
            for (o, n) in counts {
                let e = best.entry(o.clone()).or_insert(*n);
                *e = (*e).min(*n);
            }
        }
        let text: String = best.iter().map(|(o, n)| format!("{o}|{n}\n")).collect();
        std::fs::write(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/agilent_audit/noti.txt"), format!("# file | items the reader gives at least (names only, no values); rewritten with AUDIT_WRITE=1\n{text}")).unwrap();
    }
}

#[test]
fn an_unknown_field_is_kept_not_dropped() {
    let xml = br#"<Root><Known>1</Known><NeverSeenBefore a="x">value</NeverSeenBefore><Field><Name>N</Name><Value>V</Value></Field></Root>"#;
    let items = meta::collect(&[("some.xml", xml), ("odd.dat", b"\x01\x02\x03")]);
    let has = |v: &str| items.iter().any(|i| i.value == v);
    assert!(has("1") && has("value") && has("x") && has("V"));
    assert!(
        items
            .iter()
            .any(|i| i.origin == "odd.dat" && i.name == "undecoded sha1" && i.value.len() == 40),
        "a binary file is declared with its SHA-1"
    );
}
