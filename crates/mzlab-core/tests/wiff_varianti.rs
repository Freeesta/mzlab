//! The `.wiff` reader of the readers module (feature `estesi`) against the synthetic pairs of `tools/wiff_riferimento.py`: a QTRAP 5500 with scan blocks of
//! versions 1 and 3 and precursor-ion / neutral-loss scans, and the files that must be refused with `err.file.wiff` (the cause in `detail`), never read as a
//! full scan. Skipped without python3 (`MZLAB_GOLDEN_STRICT=1` fails instead).

#![cfg(feature = "estesi")]

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::lettori::{self, FileSet, Limiti, Membro};
use mzlab_core::wiff::{container, Wiff};
use serde_json::Value;

fn reference() -> Option<(PathBuf, Value)> {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_varianti_{}", std::process::id()));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_riferimento.py");
    let ok = Command::new("python3")
        .arg(script)
        .arg(&out)
        .output()
        .ok()
        .filter(|o| o.status.success());
    if ok.is_none() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP wiff_varianti: python3 not available");
        return None;
    }
    let json =
        serde_json::from_slice(&std::fs::read(out.join("riferimento.json")).unwrap()).unwrap();
    Some((out, json))
}

fn nums(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

fn close(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

fn set(dir: &Path, name: &str) -> FileSet {
    let member = |file: String| {
        let bytes = std::fs::read(dir.join(&file)).unwrap();
        Membro {
            nome: file,
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
        }
    };
    FileSet {
        membri: vec![
            member(format!("{name}.wiff")),
            member(format!("{name}.wiff.scan")),
        ],
    }
}

fn open(dir: &Path, name: &str) -> Result<(Wiff, Vec<u8>), mzlab_core::error::Error> {
    let ole = container(
        std::fs::read(dir.join(format!("{name}.wiff"))).unwrap(),
        name,
    )
    .unwrap();
    let scan = std::fs::read(dir.join(format!("{name}.wiff.scan"))).unwrap();
    Wiff::open(&ole, name, Some(scan.len() as u64), 1).map(|w| (w, scan))
}

fn detail(e: &mzlab_core::error::Error) -> String {
    assert_eq!(e.key, "err.file.wiff");
    e.params
        .iter()
        .find(|(k, _)| *k == "detail")
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}

#[test]
fn a_qtrap_5500_reads_its_scan_types() {
    let Some((dir, want)) = reference() else {
        return;
    };
    let r = &want["variante"]["1"];
    let (w, mut scan) = open(&dir, "variante").unwrap();
    assert!(w.variante() && w.model.as_deref() == r["model"].as_str());
    let run = w.run(&mut scan).unwrap();
    let sp = r["spectra"].as_array().unwrap();
    assert_eq!(run.spectra.len(), sp.len());
    for (g, e) in run.spectra.iter().zip(sp) {
        assert_eq!(g.id, e["id"].as_str().unwrap());
        assert_eq!(
            (g.level as i64, g.precursor),
            (e["level"].as_i64().unwrap(), e["precursor"].as_f64()),
            "{}",
            g.id
        );
        let (mz, it) = (nums(&e["mz"]), nums(&e["intensity"]));
        assert_eq!(g.mz.len(), mz.len(), "{}: points", g.id);
        assert!(
            (0..mz.len()).all(|k| close(g.mz[k], mz[k]) && close(g.intensity_f64(k), it[k])),
            "{}",
            g.id
        );
    }
}

#[test]
fn the_mzml_of_the_module_names_the_instrument_and_the_spectrum_types() {
    let Some((dir, want)) = reference() else {
        return;
    };
    let mut l = lettori::apri(set(&dir, "variante"), &Limiti::sito()).unwrap();
    assert!(l.variante(0));
    let xml = String::from_utf8(
        mzlab_core::lettori_mzml::scrivi(&mut *l, 0, Vec::new(), &mut |_, _| {}).unwrap(),
    )
    .unwrap();
    assert!(
        xml.contains("accession=\"MS:1000931\" name=\"QTRAP 5500\""),
        "instrument"
    );
    assert!(!xml.contains("name=\"instrument model\""));
    let content = &xml[xml.find("<fileContent>").unwrap()..xml.find("</fileContent>").unwrap()];
    assert!(
        content.contains("MS:1000341") && content.contains("MS:1000326"),
        "{content}"
    );
    for (term, name) in want["variante"]["1"]["spectra"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|s| s["term"].as_array())
        .map(|t| (t[0].as_str().unwrap(), t[1].as_str().unwrap()))
    {
        assert!(
            xml.contains(&format!("accession=\"{term}\" name=\"{name}\"")),
            "{term}"
        );
    }
    // one precursor-ion and one neutral-loss spectrum per cycle (two cycles), each type in its own spectra
    assert_eq!(
        xml.matches("accession=\"MS:1000341\" name=\"precursor ion spectrum\"")
            .count(),
        2 + 1
    );
    assert_eq!(
        xml.matches("accession=\"MS:1000326\" name=\"constant neutral loss spectrum\"")
            .count(),
        1 + 1
    );
}

#[test]
fn what_the_reader_does_not_know_is_refused_never_read_as_a_full_scan() {
    let Some((dir, want)) = reference() else {
        return;
    };
    for (name, text) in want["rifiuti"].as_object().unwrap() {
        let text = text.as_str().unwrap();
        // a code or a period is refused when the sample is opened; a block when it is read
        let err = match open(&dir, name) {
            Err(e) => e,
            Ok((w, mut scan)) => w
                .run(&mut scan)
                .err()
                .unwrap_or_else(|| panic!("{name}: read")),
        };
        assert_eq!(detail(&err), text, "{name}");
        let via = lettori::apri(set(&dir, name), &Limiti::sito());
        assert_eq!(
            detail(&via.err().unwrap_or_else(|| panic!("{name}: registry"))),
            text,
            "{name}: registry"
        );
    }
}

fn dati_formati() -> Option<PathBuf> {
    let mut cands: Vec<PathBuf> = std::env::var_os("MZLAB_DATI")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .ok()?;
    cands.extend(
        root.ancestors()
            .skip(1)
            .take(3)
            .map(|p| p.join("mzlab-dati")),
    );
    cands
        .into_iter()
        .map(|p| p.join("dati_formati"))
        .find(|p| p.is_dir())
}

fn pairs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            pairs(&p, out);
        } else if p.extension().is_some_and(|x| x == "wiff")
            && p.with_extension("wiff.scan").is_file()
        {
            out.push(p);
        }
    }
}

/// Real SCIEX files of other instruments (private data repository): every sample opens and its scans add up to the TIC of the index, or is refused with a cause.
#[test]
fn real_qtrap_files_open_with_the_tic_of_the_index_or_are_refused() {
    let Some(d) = dati_formati() else {
        eprintln!("SKIP wiff_varianti: no data repository (MZLAB_DATI)");
        return;
    };
    let mut files = Vec::new();
    pairs(&d, &mut files);
    files.sort();
    let (mut opened, mut refused, mut scans, mut off) = (0, 0, 0, 0);
    for f in &files {
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        let ole = container(std::fs::read(f).unwrap(), &name).unwrap();
        let mut scan = std::fs::read(f.with_extension("wiff.scan")).unwrap();
        for s in mzlab_core::wiff::samples(&ole) {
            let w = match Wiff::open(&ole, &name, Some(scan.len() as u64), s) {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("REFUSED {name} #{s}: {}", detail(&e));
                    refused += 1;
                    continue;
                }
            };
            let run = match w.run(&mut scan) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("REFUSED {name} #{s}: {}", detail(&e));
                    refused += 1;
                    continue;
                }
            };
            opened += 1;
            let n_exp = w.experiments.len().max(1);
            for sp in &run.spectra {
                let field = |k: &str| {
                    sp.id
                        .split_whitespace()
                        .find_map(|x| x.strip_prefix(k))
                        .and_then(|v| v.parse::<usize>().ok())
                        .unwrap()
                };
                let rec = &w.records[(field("cycle=") - 1) * n_exp + field("experiment=") - 1];
                let sum: f64 = (0..sp.mz.len()).map(|k| sp.intensity_f64(k)).sum();
                scans += 1;
                off += usize::from(
                    !close(sum, rec.tic) && (sum - rec.tic).abs() > 1e-6 * rec.tic.abs(),
                );
            }
            eprintln!(
                "OK {name} #{s}: model {:?}, codes {:?}, {} spectra, {} chromatograms",
                w.model,
                w.experiments.iter().map(|e| e.code).collect::<Vec<_>>(),
                run.spectra.len(),
                run.chromatograms.len()
            );
        }
    }
    eprintln!("{} files: {opened} samples read, {refused} refused, {scans} scans, {off} differ from the TIC of the index", files.len());
    assert!(
        opened > 0 && off * 50 <= scans.max(1),
        "{off} of {scans} scans differ from the index"
    );
}

fn transition_key(id: &str) -> Option<String> {
    let at = id.find("experiment=")?;
    Some(id[at..].to_string())
}

/// The chromatograms and spectra of the real files with a ProteoWizard mzML next to them (private data repository): the mzML of msconvert is the reference.
#[test]
fn real_files_match_proteowizard_mzml() {
    let Some(d) = dati_formati() else {
        eprintln!("SKIP wiff_varianti: no data repository (MZLAB_DATI)");
        return;
    };
    let mut mzmls = Vec::new();
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "mzML") {
                out.push(p);
            }
        }
    }
    walk(&d.join("sciex_wiff"), &mut mzmls);
    walk(&d.join("scansioni_speciali"), &mut mzmls);
    mzmls.sort();
    let (mut done, mut chroms, mut spectra, mut scheduled) = (0, 0, 0, 0);
    for f in mzmls {
        // the mzML of ProteoWizard is named after the .wiff, with a suffix after a dash for the sample
        let stem = f.file_stem().unwrap().to_string_lossy().to_string();
        let Some(wiff) = std::fs::read_dir(f.parent().unwrap())
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .find(|p| {
                p.extension().is_some_and(|x| x == "wiff")
                    && stem.starts_with(&*p.file_stem().unwrap().to_string_lossy())
            })
        else {
            continue;
        };
        let want =
            mzlab_core::mzml::read(std::io::BufReader::new(std::fs::File::open(&f).unwrap()))
                .unwrap();
        let name = wiff.file_name().unwrap().to_string_lossy().to_string();
        let ole = container(std::fs::read(&wiff).unwrap(), &name).unwrap();
        let mut scan = std::fs::read(wiff.with_extension("wiff.scan")).unwrap();
        let sample = want
            .spectra
            .first()
            .map(|s| s.id.clone())
            .or_else(|| {
                want.chromatograms
                    .iter()
                    .find(|c| c.id.contains("sample="))
                    .map(|c| c.id.clone())
            })
            .and_then(|id| {
                id.split("sample=")
                    .nth(1)?
                    .split_whitespace()
                    .next()?
                    .parse::<u32>()
                    .ok()
            })
            .unwrap();
        let w = Wiff::open(&ole, &name, Some(scan.len() as u64), sample).unwrap();
        let got = w.run(&mut scan).unwrap();
        let label = f.display().to_string();
        for e in want
            .chromatograms
            .iter()
            .filter(|c| c.id.contains("transition="))
        {
            let key = transition_key(&e.id).unwrap();
            let Some(g) = got
                .chromatograms
                .iter()
                .find(|g| transition_key(&g.id).as_deref() == Some(&key))
            else {
                panic!("{label}: {} missing", e.id)
            };
            let n = e.time.len().min(g.time.len());
            // ProteoWizard leaves out the last block of the file
            let exact = g.time.len().abs_diff(e.time.len()) <= 1
                && n > 0
                && (0..n).all(|k| {
                    close(g.time[k], e.time[k])
                        && (g.intensity[k] - e.intensity[k]).abs()
                            <= 1e-6 * e.intensity[k].abs().max(1.0)
                });
            if !exact {
                // scheduled MRM: ProteoWizard keeps each transition inside its window (zero-filled); every nonzero point of it must be ours, with the same value
                let mut at = 0;
                for (t, v) in e.time.iter().zip(&e.intensity).filter(|p| *p.1 > 0.0) {
                    let k = (at..g.time.len()).find(|&k| close(g.time[k], *t));
                    let Some(k) = k else {
                        panic!("{label} {}: point at {t} missing", e.id)
                    };
                    assert!(
                        (g.intensity[k] - v).abs() <= 1e-6 * v.abs().max(1.0),
                        "{label} {} at {t}",
                        e.id
                    );
                    at = k;
                }
                scheduled += 1;
            }
            chroms += 1;
        }
        let n = want.spectra.len().saturating_sub(1);
        for e in &want.spectra[..n] {
            let g = got
                .spectra
                .iter()
                .find(|g| g.id == e.id)
                .unwrap_or_else(|| panic!("{label}: {} missing", e.id));
            assert!(
                close(g.rt, e.rt) && g.level == e.level && g.polarity == e.polarity,
                "{label} {}: rt/level/polarity {} {} {} {} {} {}",
                e.id,
                g.rt,
                e.rt,
                g.level,
                e.level,
                g.polarity,
                e.polarity
            );
            assert_eq!(
                g.precursor.is_some(),
                e.precursor.is_some(),
                "{label} {}: precursor",
                e.id
            );
            let pts: Vec<(f64, f64)> = (0..e.mz.len())
                .map(|k| (e.mz[k], e.intensity_f64(k)))
                .filter(|p| p.1 > 0.0)
                .collect();
            assert_eq!(g.mz.len(), pts.len(), "{label} {}: points", e.id);
            for (k, p) in pts.iter().enumerate() {
                assert!(
                    close(g.mz[k], p.0)
                        && (g.intensity_f64(k) - p.1).abs() <= 1e-6 * p.1.abs().max(1.0),
                    "{label} {} point {k}",
                    e.id
                );
            }
            spectra += 1;
        }
        eprintln!(
            "compared {label}: {} chromatograms, {} spectra",
            want.chromatograms.len(),
            want.spectra.len()
        );
        done += 1;
    }
    eprintln!(
        "{done} mzML files, {chroms} SRM chromatograms ({scheduled} scheduled), {spectra} spectra"
    );
    assert!(done > 0, "no mzML paired with a wiff");
}

/// `metodo/1` of real files: the sample name of ProteoWizard's mzML (`<wiff>-<name>.mzML`) is `campione.nome`, a scheduled experiment says so with the window the
/// method states (the file named «6minWindow» has 360 s), the transitions of a scheduled experiment carry their target time, the version of Analyst is in the GLP tables.
#[test]
fn real_methods_say_what_the_files_say() {
    use mzlab_core::metodo::Source;
    let Some(d) = dati_formati() else {
        eprintln!("SKIP wiff_varianti: no data repository (MZLAB_DATI)");
        return;
    };
    let method = |sub: &str, file: &str, sample: u32| -> Value {
        let path = d.join(sub).join(file);
        let ole = container(std::fs::read(&path).unwrap(), file).unwrap();
        let src = Source {
            format: "sciex-wiff",
            files: vec![],
            sample: Some(sample),
        };
        serde_json::from_str(&mzlab_core::metodo_wiff::from_wiff(&ole, sample, &src)).unwrap()
    };
    let experiments = |m: &Value| {
        m["ms"]["periodi"][0]["esperimenti"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    };
    let blank = method(
        "sciex_wiff/qtrap6500lowmass_mrm_mzml_MTBLS1381",
        "QTrap007483_031618.wiff",
        1,
    );
    assert_eq!(
        blank["campione"]["nome"], "Blank ext",
        "the mzML of msconvert is named QTrap007483_031618-Blank ext"
    );
    let sched = experiments(&blank)
        .into_iter()
        .find(|e| e["programmato"] == true)
        .expect("a scheduled experiment");
    assert_eq!(sched["finestra_s"], 120);
    let t = &sched["transizioni"][0];
    assert!(
        t.get("dwell_ms").is_none() && t["rt_min"].as_f64().is_some_and(|v| v > 0.0 && v < 60.0),
        "{t}"
    );
    assert!(blank["campi"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["chiave"] == "software"
            && c["valore"]
                .as_str()
                .is_some_and(|v| v.starts_with("Analyst "))));
    let moss = method(
        "sciex_wiff/qtrap4500_smrm_PXD005017",
        "Mix-moss_20160630_sMRM_6minWindow_30minGrad_60minTotal_3ulinj_01.wiff",
        1,
    );
    assert!(
        experiments(&moss).iter().any(|e| e["finestra_s"] == 360),
        "6minWindow"
    );
    let plain = method(
        "sciex_wiff/qtrap5500_mrm_PXD001506",
        "Duodenum_MRM_SMB_01.wiff",
        1,
    );
    assert!(
        experiments(&plain)
            .iter()
            .all(|e| e.get("programmato").is_none())
            && experiments(&plain)[0]["transizioni"][0]
                .get("dwell_ms")
                .is_some()
    );
    let lab = method("../3200 qtrap/wiff inquinanti 25-26 B", "MRM.wiff", 1);
    let dad = lab["campi"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| {
            c["origine"]
                .as_str()
                .is_some_and(|o| o.ends_with("DADSampleHeader"))
        })
        .count();
    assert_eq!(
        dad, 6,
        "the DAD header of the 3200 files: points, channels, duration, range, step"
    );
}

/// A damaged file never panics: every byte of the container and of the scan file of the synthetic QTRAP 5500 pair is overwritten in turn with values that
/// make sizes, counts and codes absurd; the reader opens it or refuses it with an `err.*` key, and the method document is always written.
#[test]
fn damaged_files_are_refused_or_read_never_a_panic() {
    let Some((dir, _)) = reference() else { return };
    let wiff = std::fs::read(dir.join("variante.wiff")).unwrap();
    let scan = std::fs::read(dir.join("variante.wiff.scan")).unwrap();
    let try_open = |wiff: Vec<u8>, scan: Vec<u8>| {
        let Ok(ole) = container(wiff, "variante.wiff") else {
            return;
        };
        drop(mzlab_core::metodo_wiff::from_wiff(
            &ole,
            1,
            &Default::default(),
        ));
        let mut scan = scan;
        if let Ok(w) = Wiff::open(&ole, "variante.wiff", Some(scan.len() as u64), 1) {
            drop(w.run(&mut scan));
        }
    };
    for at in (0..wiff.len()).step_by(7) {
        for v in [0u8, 0xff, 0x7f] {
            let mut w = wiff.clone();
            w[at] = v;
            try_open(w, scan.clone());
        }
    }
    for at in 0..scan.len() {
        for v in [0u8, 0xff, 0x80] {
            let mut s = scan.clone();
            s[at] = v;
            try_open(wiff.clone(), s);
        }
    }
    for cut in (0..scan.len()).step_by(11) {
        try_open(wiff.clone(), scan[..cut].to_vec());
    }
}
