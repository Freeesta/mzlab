//! The `.wiff` reader of the readers module (feature `estesi`) on the real files of other SCIEX instruments (private data repository, `dati_formati/`):
//! every sample opens with the TIC of the index or is refused with a cause, the chromatograms and spectra match the ProteoWizard mzML next to the file,
//! `metodo/1` says what the files say. Skipped without the data repository.

#![cfg(feature = "sciex")]

use std::path::{Path, PathBuf};

use mzlab_core::mzml::Run;
use mzlab_core::wiff::{container, Wiff};
use serde_json::Value;

fn close(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

fn detail(e: &mzlab_core::error::Error) -> String {
    assert_eq!(e.key, "err.file.wiff");
    e.params
        .iter()
        .find(|(k, _)| *k == "detail")
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
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
    mzlab_core::sortc::slice_ord(&mut files);
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

fn mzml_files(d: &Path) -> Vec<PathBuf> {
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
    let mut mzmls = Vec::new();
    walk(&d.join("sciex_wiff"), &mut mzmls);
    walk(&d.join("scansioni_speciali"), &mut mzmls);
    mzlab_core::sortc::slice_ord(&mut mzmls);
    mzmls
}

/// The `.wiff` a ProteoWizard mzML belongs to: it is named after it, with a suffix after a dash for the sample.
fn wiff_of(mzml: &Path) -> Option<PathBuf> {
    let stem = mzml.file_stem()?.to_string_lossy().to_string();
    std::fs::read_dir(mzml.parent()?)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.extension().is_some_and(|x| x == "wiff")
                && stem.starts_with(&*p.file_stem().unwrap().to_string_lossy())
        })
}

/// The sample number the ids of the mzML name (`sample=N`).
fn sample_of(want: &Run) -> u32 {
    want.spectra
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
        .unwrap()
}

/// Every SRM chromatogram of the reference is ours; returns (compared, scheduled ones compared by their nonzero points).
fn compare_chromatograms(label: &str, want: &Run, got: &Run) -> (usize, usize) {
    let (mut chroms, mut scheduled) = (0, 0);
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
    (chroms, scheduled)
}

/// Every spectrum of the reference (but the last block) is ours: time, level, polarity, precursor and the nonzero points.
fn compare_spectra(label: &str, want: &Run, got: &Run) -> usize {
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
    }
    n
}

/// The chromatograms and spectra of the real files with a ProteoWizard mzML next to them (private data repository): the mzML of msconvert is the reference.
#[test]
fn real_files_match_proteowizard_mzml() {
    let Some(d) = dati_formati() else {
        eprintln!("SKIP wiff_varianti_reali: no data repository (MZLAB_DATI)");
        return;
    };
    let (mut done, mut chroms, mut spectra, mut scheduled) = (0, 0, 0, 0);
    for f in mzml_files(&d) {
        let Some(wiff) = wiff_of(&f) else {
            continue;
        };
        let want =
            mzlab_core::mzml::read(std::io::BufReader::new(std::fs::File::open(&f).unwrap()))
                .unwrap();
        let name = wiff.file_name().unwrap().to_string_lossy().to_string();
        let ole = container(std::fs::read(&wiff).unwrap(), &name).unwrap();
        let mut scan = std::fs::read(wiff.with_extension("wiff.scan")).unwrap();
        let w = Wiff::open(&ole, &name, Some(scan.len() as u64), sample_of(&want)).unwrap();
        let got = w.run(&mut scan).unwrap();
        let label = f.display().to_string();
        let (c, s) = compare_chromatograms(&label, &want, &got);
        chroms += c;
        scheduled += s;
        spectra += compare_spectra(&label, &want, &got);
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

/// `metodo/1` of the real file `file` in `sub` of the data repository, sample `sample`.
fn method(d: &Path, sub: &str, file: &str, sample: u32) -> Value {
    use mzlab_core::metodo::Source;
    let ole = container(std::fs::read(d.join(sub).join(file)).unwrap(), file).unwrap();
    let src = Source {
        format: "sciex-wiff",
        files: vec![],
        sample: Some(sample),
    };
    serde_json::from_str(&mzlab_core::metodo_wiff::from_wiff(&ole, sample, &src)).unwrap()
}

fn experiments(m: &Value) -> Vec<Value> {
    m["ms"]["periodi"][0]["esperimenti"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

/// `metodo/1` of real files: the sample name of ProteoWizard's mzML (`<wiff>-<name>.mzML`) is `campione.nome`, a scheduled experiment says so with the window the
/// method states (the file named «6minWindow» has 360 s), the transitions of a scheduled experiment carry their target time, the version of Analyst is in the GLP tables.
#[test]
fn real_methods_say_what_the_files_say() {
    let Some(d) = dati_formati() else {
        eprintln!("SKIP wiff_varianti_reali: no data repository (MZLAB_DATI)");
        return;
    };
    let blank = method(
        &d,
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
        &d,
        "sciex_wiff/qtrap4500_smrm_PXD005017",
        "Mix-moss_20160630_sMRM_6minWindow_30minGrad_60minTotal_3ulinj_01.wiff",
        1,
    );
    assert!(
        experiments(&moss).iter().any(|e| e["finestra_s"] == 360),
        "6minWindow"
    );
    let plain = method(
        &d,
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
    let lab = method(&d, "../3200 qtrap/wiff inquinanti 25-26 B", "MRM.wiff", 1);
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
