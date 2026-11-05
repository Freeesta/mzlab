//! Structured mutations of valid synthetic files, one base per reader: cuts at every structural boundary, flipped bytes, numbers swapped for hostile ones (NaN, inf,
//! negative, huge), duplicated and removed elements. Every mutant is read end to end (index, every scan, headers, chromatograms, method, the mzML writer) on a thread with a
//! deadline: a panic, a hang or a non-finite number in the output is a finding. Deterministic seeds; `MZLAB_MUTAZIONI=N` multiplies the random rounds for a longer search.
#![cfg(feature = "tutte")]

mod comune;
mod comune_sqlite;
mod comune_waters;

use comune::andi::{andi_cromato, andi_ms, PIATTA};
use comune::mzxml::{corsa, mzxml, PIATTO};
use comune::*;
use mzlab_core::lettori::{FileSet, Limiti, Membro};
use std::sync::mpsc;
use std::time::Duration;

/// xorshift: the same mutants on every run.
struct Caso(u64);
impl Caso {
    fn n(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn sotto(&mut self, n: usize) -> usize {
        (self.n() % n.max(1) as u64) as usize
    }
}

fn giri() -> usize {
    std::env::var("MZLAB_MUTAZIONI")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
}

fn insieme_di(membri: &[(String, Vec<u8>)]) -> FileSet {
    FileSet {
        membri: membri
            .iter()
            .map(|(nome, b)| Membro {
                nome: nome.clone(),
                dim: b.len() as u64,
                dati: Box::new(b.clone()),
                sha1: None,
            })
            .collect(),
    }
}

/// Reads everything a page and the CLI read; `Err` names the first non-finite number met.
fn leggi_tutto(membri: &[(String, Vec<u8>)]) -> Result<(), String> {
    let Ok(mut l) = mzlab_lettori::apri(insieme_di(membri), &Limiti::sito()) else {
        return Ok(());
    };
    for c in 0..l.campioni().len() {
        let _ = l.metodo(c);
        let _ = l.testata_mzml(c);
        let Ok(ix) = l.indice(c).map(<[_]>::to_vec) else {
            continue;
        };
        for (k, v) in ix.iter().enumerate() {
            if !v.rt.is_finite() {
                return Err(format!("index RT of scan {k} is {}", v.rt));
            }
            if let Ok(s) = l.scansione(c, k) {
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
            let _ = l.intestazione(c, k);
        }
        if let Ok(cr) = l.cromatogrammi(c) {
            for v in &cr {
                let Ok(t) = l.cromatogramma(c, &v.id) else {
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
        let _ = mzlab_core::lettori_mzml::scrivi(l.as_mut(), c, Vec::new(), &mut |_, _| {});
    }
    Ok(())
}

/// Runs `f` on a thread with a 5 s deadline; a panic, a hang or an `Err` is a finding.
fn sorveglia(
    cosa: &str,
    f: impl FnOnce() -> Result<(), String> + Send + 'static,
) -> Option<String> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        let _ = tx.send(r);
    });
    match rx.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(Ok(()))) => None,
        Ok(Ok(Err(e))) => Some(format!("{cosa}: {e}")),
        Ok(Err(p)) => Some(format!(
            "{cosa}: panic {}",
            p.downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default()
        )),
        Err(_) => Some(format!("{cosa}: no answer in 5 s")),
    }
}

fn gz(b: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    e.write_all(b).unwrap();
    e.finish().unwrap()
}

const OSTILI: &[&str] = &[
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
fn mutanti_testo(b: &[u8], caso: &mut Caso, giri: usize) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for (i, _) in b
        .iter()
        .enumerate()
        .filter(|(_, c)| **c == b'<' || **c == b'>')
    {
        out.push((format!("cut@{i}"), b[..i].to_vec()));
    }
    // attribute values: every `="…"` gets each hostile value in turn (bounded by seeded picks)
    let virgolette: Vec<usize> = b
        .windows(2)
        .enumerate()
        .filter(|(_, w)| w == b"=\"")
        .map(|(i, _)| i + 2)
        .collect();
    for _ in 0..200 * giri {
        let Some(&a) = virgolette.get(caso.sotto(virgolette.len())) else {
            break;
        };
        let Some(z) = b[a..].iter().position(|c| *c == b'"') else {
            continue;
        };
        let v = OSTILI[caso.sotto(OSTILI.len())];
        let mut m = b[..a].to_vec();
        m.extend_from_slice(v.as_bytes());
        m.extend_from_slice(&b[a + z..]);
        out.push((format!("attr@{a}={v}"), m));
    }
    let righe: Vec<&[u8]> = b.split_inclusive(|c| *c == b'\n').collect();
    for k in 0..righe.len() {
        let senza: Vec<u8> = righe
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != k)
            .flat_map(|(_, r)| r.iter().copied())
            .collect();
        out.push((format!("drop-line{k}"), senza));
        let doppia: Vec<u8> = righe
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
        out.push((format!("dup-line{k}"), doppia));
    }
    for _ in 0..200 * giri {
        let mut m = b.to_vec();
        for _ in 0..1 + caso.sotto(4) {
            let i = caso.sotto(m.len());
            m[i] = caso.n() as u8;
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
    let mut coda = b.to_vec();
    coda.extend_from_slice(b"\0\0garbage<<<");
    out.push(("trailing".into(), coda));
    out
}

/// Binary mutants: cuts on a grid, flipped bytes, u32 words set to 0, 0xFFFFFFFF or 0x7FFFFFFF, f64 words set to NaN or -1.
fn mutanti_binari(b: &[u8], caso: &mut Caso, giri: usize) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let passo = (b.len() / 300).max(1);
    for i in (0..b.len()).step_by(passo) {
        out.push((format!("cut@{i}"), b[..i].to_vec()));
    }
    for _ in 0..300 * giri {
        let mut m = b.to_vec();
        let i = caso.sotto(m.len().saturating_sub(8)) & !3;
        let quale = caso.sotto(6);
        let parola: Vec<u8> = match quale {
            0 => 0u32.to_le_bytes().to_vec(),
            1 => u32::MAX.to_le_bytes().to_vec(),
            2 => 0x7FFF_FFFFu32.to_le_bytes().to_vec(),
            3 => f64::NAN.to_le_bytes().to_vec(),
            4 => (-1.0f64).to_le_bytes().to_vec(),
            _ => vec![caso.n() as u8],
        };
        if i + parola.len() <= m.len() {
            m[i..i + parola.len()].copy_from_slice(&parola);
        }
        out.push((format!("word{quale}@{i}"), m));
    }
    out
}

fn esplora(
    etichetta: &str,
    membri: Vec<(String, Vec<u8>)>,
    quale: usize,
    testo: bool,
) -> Vec<String> {
    let mut caso = Caso(0x9E37_79B9_7F4A_7C15 ^ etichetta.len() as u64);
    let base = membri[quale].1.clone();
    let mutanti = if testo {
        mutanti_testo(&base, &mut caso, giri())
    } else {
        mutanti_binari(&base, &mut caso, giri())
    };
    let mut trovati = Vec::new();
    for (nome, m) in mutanti {
        let mut ms = membri.clone();
        ms[quale].1 = m;
        if let Some(e) = sorveglia(&format!("{etichetta} {nome}"), move || leggi_tutto(&ms)) {
            trovati.push(e);
        }
    }
    trovati
}

fn esigi_nessuno(trovati: Vec<String>) {
    for t in trovati.iter().take(40) {
        eprintln!("{t}");
    }
    assert!(trovati.is_empty(), "{} findings", trovati.len());
}

#[test]
fn mzml_numpress_mutato() {
    let np = mzml_di(
        &scansioni(),
        Cod::Np {
            intensita: mzlab_core::numpress::Np::Pic,
            zlib: true,
        },
        None,
    );
    esigi_nessuno(esplora("mzml-np", vec![("x.mzML".into(), np)], 0, true));
}

#[test]
fn mzxml_mutato() {
    esigi_nessuno(esplora(
        "mzxml",
        vec![("x.mzXML".into(), mzxml(&corsa(), PIATTO))],
        0,
        true,
    ));
}

#[test]
fn mzml_gz_mutato() {
    let gz = gz(&mzml_di(&scansioni(), Cod::Piano, None));
    esigi_nessuno(esplora("mzml-gz", vec![("x.mzML.gz".into(), gz)], 0, false));
}

#[test]
fn andi_mutato() {
    esigi_nessuno(esplora(
        "andi-ms",
        vec![("x.cdf".into(), andi_ms(&PIATTA))],
        0,
        false,
    ));
    esigi_nessuno(esplora(
        "andi-cromato",
        vec![("x.cdf".into(), andi_cromato(40))],
        0,
        false,
    ));
}

#[test]
fn shimadzu_mutato() {
    esigi_nessuno(esplora(
        "qgd",
        vec![("x.qgd".into(), comune::shimadzu::qgd(8))],
        0,
        false,
    ));
    esigi_nessuno(esplora(
        "lcd",
        vec![("x.lcd".into(), comune::shimadzu::lcd2020(8))],
        0,
        false,
    ));
}

#[test]
fn thermo_mutato() {
    let raw = comune::thermo::Raw::tsq(63, 6).bytes();
    esigi_nessuno(esplora("thermo", vec![("x.raw".into(), raw)], 0, false));
}

#[test]
fn mzxml_annidato_32_mutato() {
    let o = comune::mzxml::Opz {
        a64: false,
        zlib: false,
        indice: false,
        annidato: true,
        attributi: false,
    };
    let b = mzxml(&comune::mzxml::annida(&corsa()), o);
    esigi_nessuno(esplora(
        "mzxml-annidato",
        vec![("x.mzXML".into(), b)],
        0,
        true,
    ));
}

#[test]
fn mzml_gz_multimembro_mutato() {
    let piano = mzml_di(&scansioni(), Cod::Piano, None);
    let meta = piano.len() / 2;
    let mut b = gz(&piano[..meta]);
    b.extend(gz(&piano[meta..]));
    esigi_nessuno(esplora("mzml-gz2", vec![("x.mzML.gz".into(), b)], 0, false));
}

#[test]
fn waters_mutato() {
    let file = comune_waters::file_cartella();
    for q in 0..file.len() {
        let e = format!("waters {}", file[q].0);
        esigi_nessuno(esplora(
            &e,
            file.clone(),
            q,
            file[q].0.ends_with(".TXT") || file[q].0.ends_with(".INF"),
        ));
    }
}

#[test]
fn bruker_mutato() {
    let f = vec![
        ("x.d/analysis.tdf".to_string(), comune_sqlite::tdf_minimo(3)),
        ("x.d/analysis.tdf_bin".to_string(), comune_sqlite::blocco2()),
    ];
    esigi_nessuno(esplora("bruker tdf", f.clone(), 0, false));
    esigi_nessuno(esplora("bruker bin", f, 1, false));
}

/// The site's own path for a plain mzML (scan table, `.mzidx`, chromatograms, parser): no panic, no hang, no NaN or infinite number passed on.
fn motore_mzml(b: &[u8]) -> Result<(), String> {
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
            let fuori = !s.rt.is_finite()
                || s.mz.iter().any(|x| !x.is_finite())
                || s.intensity.iter().any(|x| !x.is_finite());
            if bad.is_none() && fuori {
                bad = Some(format!("parse: spectrum {} not finite", s.index));
            }
        }
    });
    bad.map_or(Ok(()), Err)
}

#[test]
fn mzml_del_motore_mutato() {
    let piano = mzml_di(&scansioni(), Cod::Nudo, None);
    let mut caso = Caso(0x9E37_79B9_7F4A_7C15 ^ 0x6d6f746f7265);
    let mut trovati = Vec::new();
    for (nome, m) in mutanti_testo(&piano, &mut caso, giri()) {
        if let Some(e) = sorveglia(&format!("engine {nome}"), move || motore_mzml(&m)) {
            trovati.push(e);
        }
    }
    esigi_nessuno(trovati);
}
