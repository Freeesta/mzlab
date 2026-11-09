//! Structured mutations of valid synthetic files, one base per reader: cuts at every structural boundary, flipped bytes, numbers swapped for hostile ones (NaN, inf,
//! negative, huge), duplicated and removed elements. Every mutant is read end to end (index, every scan, headers, chromatograms, method, the mzML writer) on a thread with a
//! deadline: a panic, a hang or a non-finite number in the output is a finding. Deterministic seeds; `MZLAB_MUTAZIONI=N` multiplies the random rounds for a longer search.
#![cfg(feature = "tutte")]

mod common;
mod common_sqlite;
mod common_waters;

use common::andi::{andi_chromatogram, andi_ms, FLAT_RUN};
use common::mzxml::{mzxml, run, FLAT};
use common::*;
use mzlab_core::readers::{FileSet, Limits, Member};
use std::sync::mpsc;
use std::time::Duration;

/// xorshift: the same mutants on every run.
struct Case(u64);
impl Case {
    fn n(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.n() % n.max(1) as u64) as usize
    }
}

fn rounds() -> usize {
    std::env::var("MZLAB_MUTAZIONI")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
}

fn set_of(members: &[(String, Vec<u8>)]) -> FileSet {
    FileSet {
        membri: members
            .iter()
            .map(|(name, b)| Member {
                nome: name.clone(),
                dim: b.len() as u64,
                dati: Box::new(b.clone()),
                sha1: None,
            })
            .collect(),
    }
}

/// Reads everything a page and the CLI read; `Err` names the first non-finite number met.
fn read_all(members: &[(String, Vec<u8>)]) -> Result<(), String> {
    let Ok(mut l) = mzlab_lettori::apri(set_of(members), &Limits::sito()) else {
        return Ok(());
    };
    for c in 0..l.samples().len() {
        let _ = l.method(c);
        let _ = l.mzml_header(c);
        let Ok(ix) = l.index(c).map(<[_]>::to_vec) else {
            continue;
        };
        for (k, v) in ix.iter().enumerate() {
            if !v.rt.is_finite() {
                return Err(format!("index RT of scan {k} is {}", v.rt));
            }
            if let Ok(s) = l.scan(c, k) {
                if s.mz.len() != s.intensity.len() {
                    return Err(format!(
                        "scan {k}: {} m/z for {} intensities",
                        s.mz.len(),
                        s.intensity.len()
                    ));
                }
                if let Some(x) =
                    s.mz.iter()
                        .copied()
                        .chain(s.intensity.iter().map(|x| f64::from(*x)))
                        .find(|x| !x.is_finite())
                {
                    return Err(format!("scan {k} carries {x}"));
                }
            }
            let _ = l.scan_header(c, k);
        }
        if let Ok(cr) = l.chromatograms(c) {
            for v in &cr {
                let Ok(t) = l.chromatogram(c, &v.id) else {
                    continue;
                };
                if t.time.len() != t.intensity.len() {
                    return Err(format!(
                        "trace {}: {} times for {} values",
                        v.id,
                        t.time.len(),
                        t.intensity.len()
                    ));
                }
            }
        }
        let _ = mzlab_core::readers_mzml::scrivi(l.as_mut(), c, Vec::new(), &mut |_, _| {});
    }
    Ok(())
}

/// Runs `f` on a thread with a 5 s deadline; a panic, a hang or an `Err` is a finding.
fn watch(what: &str, f: impl FnOnce() -> Result<(), String> + Send + 'static) -> Option<String> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        let _ = tx.send(r);
    });
    match rx.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(Ok(()))) => None,
        Ok(Ok(Err(e))) => Some(format!("{what}: {e}")),
        Ok(Err(p)) => Some(format!(
            "{what}: panic {}",
            p.downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default()
        )),
        Err(_) => Some(format!("{what}: no answer in 5 s")),
    }
}

fn gz(b: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    e.write_all(b).unwrap();
    e.finish().unwrap()
}

const HOSTILE: &[&str] = &[
    "NaN",
    "-inf",
    "inf",
    "-1",
    "0",
    "1e308",
    "-1e308",
    "99999999999999999999",
    "4294967296",
    "",
    "x",
];

/// Text mutants: cuts at every `<` and `>`, hostile numbers in attribute values, duplicated and removed lines, BOM/CRLF, junk appended.
fn text_mutants(b: &[u8], case: &mut Case, rounds: usize) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for (i, _) in b
        .iter()
        .enumerate()
        .filter(|(_, c)| **c == b'<' || **c == b'>')
    {
        out.push((format!("cut@{i}"), b[..i].to_vec()));
    }
    // attribute values: every `="…"` gets each hostile value in turn (bounded by seeded picks)
    let quotes: Vec<usize> = b
        .windows(2)
        .enumerate()
        .filter(|(_, w)| w == b"=\"")
        .map(|(i, _)| i + 2)
        .collect();
    for _ in 0..200 * rounds {
        let Some(&a) = quotes.get(case.below(quotes.len())) else {
            break;
        };
        let Some(z) = b[a..].iter().position(|c| *c == b'"') else {
            continue;
        };
        let v = HOSTILE[case.below(HOSTILE.len())];
        let mut m = b[..a].to_vec();
        m.extend_from_slice(v.as_bytes());
        m.extend_from_slice(&b[a + z..]);
        out.push((format!("attr@{a}={v}"), m));
    }
    let rows: Vec<&[u8]> = b.split_inclusive(|c| *c == b'\n').collect();
    for k in 0..rows.len() {
        let without: Vec<u8> = rows
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != k)
            .flat_map(|(_, r)| r.iter().copied())
            .collect();
        out.push((format!("drop-line{k}"), without));
        let double: Vec<u8> = rows
            .iter()
            .enumerate()
            .flat_map(|(j, r)| {
                if j == k {
                    [*r, *r].concat()
                } else {
                    r.to_vec()
                }
            })
            .collect();
        out.push((format!("dup-line{k}"), double));
    }
    for _ in 0..200 * rounds {
        let mut m = b.to_vec();
        for _ in 0..1 + case.below(4) {
            let i = case.below(m.len());
            m[i] = case.n() as u8;
        }
        out.push(("flip".into(), m));
    }
    let mut bom = b"\xEF\xBB\xBF".to_vec();
    bom.extend_from_slice(b);
    out.push(("bom".into(), bom));
    let crlf: Vec<u8> = b
        .iter()
        .flat_map(|c| {
            if *c == b'\n' {
                vec![b'\r', b'\n']
            } else {
                vec![*c]
            }
        })
        .collect();
    out.push(("crlf".into(), crlf));
    let mut tail = b.to_vec();
    tail.extend_from_slice(b"\0\0garbage<<<");
    out.push(("trailing".into(), tail));
    out
}

/// Binary mutants: cuts on a grid, flipped bytes, u32 words set to 0, 0xFFFFFFFF or 0x7FFFFFFF, f64 words set to NaN or -1.
fn binary_mutants(b: &[u8], case: &mut Case, rounds: usize) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let step = (b.len() / 300).max(1);
    for i in (0..b.len()).step_by(step) {
        out.push((format!("cut@{i}"), b[..i].to_vec()));
    }
    for _ in 0..300 * rounds {
        let mut m = b.to_vec();
        let i = case.below(m.len().saturating_sub(8)) & !3;
        let which = case.below(6);
        let word: Vec<u8> = match which {
            0 => 0u32.to_le_bytes().to_vec(),
            1 => u32::MAX.to_le_bytes().to_vec(),
            2 => 0x7FFF_FFFFu32.to_le_bytes().to_vec(),
            3 => f64::NAN.to_le_bytes().to_vec(),
            4 => (-1.0f64).to_le_bytes().to_vec(),
            _ => vec![case.n() as u8],
        };
        if i + word.len() <= m.len() {
            m[i..i + word.len()].copy_from_slice(&word);
        }
        out.push((format!("word{which}@{i}"), m));
    }
    out
}

fn explore(label: &str, members: Vec<(String, Vec<u8>)>, which: usize, text: bool) -> Vec<String> {
    let mut case = Case(0x9E37_79B9_7F4A_7C15 ^ label.len() as u64);
    let base = members[which].1.clone();
    let mutants = if text {
        text_mutants(&base, &mut case, rounds())
    } else {
        binary_mutants(&base, &mut case, rounds())
    };
    let mut found = Vec::new();
    for (name, m) in mutants {
        let mut ms = members.clone();
        ms[which].1 = m;
        if let Some(e) = watch(&format!("{label} {name}"), move || read_all(&ms)) {
            found.push(e);
        }
    }
    found
}

fn require_none(found: Vec<String>) {
    for t in found.iter().take(40) {
        eprintln!("{t}");
    }
    assert!(found.is_empty(), "{} findings", found.len());
}

#[test]
fn mzml_numpress_mutated() {
    let np = mzml_di(
        &scans(),
        Cod::Np {
            intensities: mzlab_core::numpress::Np::Pic,
            zlib: true,
        },
        None,
    );
    require_none(explore("mzml-np", vec![("x.mzML".into(), np)], 0, true));
}

#[test]
fn mzxml_mutated() {
    require_none(explore(
        "mzxml",
        vec![("x.mzXML".into(), mzxml(&run(), FLAT))],
        0,
        true,
    ));
}

#[test]
fn mzml_gz_mutated() {
    let gz = gz(&mzml_di(&scans(), Cod::Piano, None));
    require_none(explore("mzml-gz", vec![("x.mzML.gz".into(), gz)], 0, false));
}

#[test]
fn andi_mutated() {
    require_none(explore(
        "andi-ms",
        vec![("x.cdf".into(), andi_ms(&FLAT_RUN))],
        0,
        false,
    ));
    require_none(explore(
        "andi-cromato",
        vec![("x.cdf".into(), andi_chromatogram(40))],
        0,
        false,
    ));
}

#[test]
fn shimadzu_mutated() {
    require_none(explore(
        "qgd",
        vec![("x.qgd".into(), common::shimadzu::qgd(8))],
        0,
        false,
    ));
    require_none(explore(
        "lcd",
        vec![("x.lcd".into(), common::shimadzu::lcd2020(8))],
        0,
        false,
    ));
}

#[test]
fn thermo_mutated() {
    let raw = common::thermo::Raw::tsq(63, 6).bytes();
    require_none(explore("thermo", vec![("x.raw".into(), raw)], 0, false));
}

#[test]
fn mzxml_nested_32_mutated() {
    let o = common::mzxml::Opz {
        a64: false,
        zlib: false,
        index: false,
        nested: true,
        attributes: false,
    };
    let b = mzxml(&common::mzxml::nest(&run()), o);
    require_none(explore(
        "mzxml-annidato",
        vec![("x.mzXML".into(), b)],
        0,
        true,
    ));
}

#[test]
fn mzml_gz_multimember_mutated() {
    let flat = mzml_di(&scans(), Cod::Piano, None);
    let meta = flat.len() / 2;
    let mut b = gz(&flat[..meta]);
    b.extend(gz(&flat[meta..]));
    require_none(explore("mzml-gz2", vec![("x.mzML.gz".into(), b)], 0, false));
}

#[test]
fn waters_mutated() {
    let file = common_waters::folder_file();
    for q in 0..file.len() {
        let e = format!("waters {}", file[q].0);
        require_none(explore(
            &e,
            file.clone(),
            q,
            file[q].0.ends_with(".TXT") || file[q].0.ends_with(".INF"),
        ));
    }
}

#[test]
fn bruker_mutated() {
    let f = vec![
        (
            "x.d/analysis.tdf".to_string(),
            common_sqlite::minimal_tdf(3),
        ),
        ("x.d/analysis.tdf_bin".to_string(), common_sqlite::blocco2()),
    ];
    require_none(explore("bruker tdf", f.clone(), 0, false));
    require_none(explore("bruker bin", f, 1, false));
}

/// The site's own path for a plain mzML (scan table, `.mzidx`, chromatograms, parser): no panic, no hang, no NaN or infinite number passed on.
fn engine_mzml(b: &[u8]) -> Result<(), String> {
    let mut src = b.to_vec();
    let Ok(p) = mzlab_core::upload::prepare(&mut src, b.len() as u64, "x.mzML") else {
        return Ok(());
    };
    if let Some(r) = p
        .table
        .rt
        .iter()
        .chain(&p.table.tic)
        .find(|r| !r.is_finite())
    {
        return Err(format!("scan table RT/TIC {r}"));
    }
    let _ = mzlab_core::mzidx::build_mzml_to_vec(&p.table, b, 64 << 20);
    let mut m = mzlab_core::mzmlhead::Mzml::new(b.to_vec(), b.len() as u64);
    let _ = m.metadata();
    if let Ok(v) = m.chromatograms(0) {
        for c in &v {
            let Ok((t, y)) = m.chromatogram(c) else {
                continue;
            };
            if t.iter().chain(&y).any(|x| !x.is_finite()) {
                return Err(format!("chromatogram {} not finite", c.id));
            }
        }
    }
    let mut bad = None;
    let _ = mzlab_core::mzml::parse(std::io::Cursor::new(b), |it| {
        if let mzlab_core::mzml::Item::Spectrum(s) = it {
            let outside = !s.rt.is_finite()
                || s.mz.iter().any(|x| !x.is_finite())
                || s.intensity.iter().any(|x| !x.is_finite());
            if bad.is_none() && outside {
                bad = Some(format!("parse: spectrum {} not finite", s.index));
            }
        }
    });
    bad.map_or(Ok(()), Err)
}

#[test]
fn mutated_engine_mzml() {
    let flat = mzml_di(&scans(), Cod::Nudo, None);
    let mut case = Case(0x9E37_79B9_7F4A_7C15 ^ 0x6d6f746f7265);
    let mut found = Vec::new();
    for (name, m) in text_mutants(&flat, &mut case, rounds()) {
        if let Some(e) = watch(&format!("engine {name}"), move || engine_mzml(&m)) {
            found.push(e);
        }
    }
    require_none(found);
}
