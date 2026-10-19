//! Agilent LC/MS `.D` folders (QqQ MRM/dMRM, Q-TOF centroid/profile/MS2, both polarities) through the module reader, on the real runs of the private data repository
//! (`dati/formati/agilent/D/<run>/<name>.d`, next to the mzML of msconvert for the pairs; the tests are skipped without it).

#![cfg(feature = "agilent")]
#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

use std::path::{Path, PathBuf};

use mzlab_core::lettori::{FileSet, Lettore, Limiti, Membro};
use mzlab_core::mzml::{self, ChromKind};

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

fn root() -> Option<PathBuf> {
    dati_percorsi::percorso("dati/formati/agilent/D").filter(|p| p.is_dir())
}

/// The `.d` folders of the run directories, sorted.
fn runs(root: &Path) -> Vec<PathBuf> {
    let mut v = Vec::new();
    for dir in std::fs::read_dir(root).unwrap().flatten() {
        if !dir.path().is_dir() {
            continue;
        }
        for e in std::fs::read_dir(dir.path()).unwrap().flatten() {
            let p = e.path();
            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
            if p.is_dir() && ext.eq_ignore_ascii_case("d") {
                v.push(p);
            }
        }
    }
    v.sort();
    v
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else {
            out.push(p);
        }
    }
}

/// The files of a folder as a `FileSet`, read from disk by offset.
fn set_of(dir: &Path) -> FileSet {
    let name = dir.file_name().unwrap().to_string_lossy().into_owned();
    let mut files = Vec::new();
    walk(dir, &mut files);
    files.sort();
    FileSet {
        membri: files
            .into_iter()
            .map(|f| {
                let rel = f
                    .strip_prefix(dir)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                let file = std::fs::File::open(&f).unwrap();
                Membro {
                    nome: format!("{name}/{rel}"),
                    dim: file.metadata().unwrap().len(),
                    dati: Box::new(file),
                    sha1: None,
                }
            })
            .collect(),
    }
}

fn open(dir: &Path) -> Box<dyn Lettore> {
    mzlab_lettori::apri_nudo(set_of(dir), &Limiti::sito()).unwrap()
}

fn name(dir: &Path) -> String {
    dir.file_name().unwrap().to_string_lossy().into_owned()
}

#[test]
fn every_real_run_opens_and_gives_its_scans_and_traces() {
    let Some(root) = root() else {
        eprintln!("SKIP agilent_lcms: no data repository (MZLAB_DATI)");
        return;
    };
    let all = runs(&root);
    assert!(all.len() >= 10, "{} runs", all.len());
    for dir in all {
        let label = name(&dir);
        // a folder without the points (MSScan.bin alone) is refused with the file error
        if label.contains("MTBLS787") {
            let e = mzlab_lettori::apri_nudo(set_of(&dir), &Limiti::sito())
                .err()
                .unwrap();
            assert_eq!(e.key, "err.file.agilent", "{label}");
            continue;
        }
        let mut l = open(&dir);
        let n = l.campioni()[0].scansioni;
        let index = l.indice(0).unwrap().to_vec();
        assert_eq!(index.len(), n, "{label}");
        let list = l.cromatogrammi(0).unwrap();
        let srm = list.iter().filter(|c| c.tipo == ChromKind::Srm).count();
        // the first, a middle and the last spectrum, and every trace
        for k in [0, n / 2, n.saturating_sub(1)]
            .into_iter()
            .filter(|_| n > 0)
        {
            let s = l.scansione(0, k).unwrap();
            assert_eq!(s.mz.len(), s.intensity.len(), "{label} #{k}");
            assert!(
                s.mz.windows(2).all(|w| w[0] <= w[1]),
                "{label} #{k}: m/z order"
            );
            assert_eq!(s.level, index[k].livello, "{label} #{k}");
            assert_eq!(s.polarity, index[k].polarita, "{label} #{k}");
        }
        for c in &list {
            let h = l.cromatogramma(0, &c.id).unwrap();
            assert_eq!(h.time.len(), h.intensity.len(), "{label} {}", c.id);
        }
        let tic = l.cromatogramma(0, "TIC").unwrap();
        assert!(!tic.time.is_empty(), "{label}: TIC");
        let m = l.metodo(0).unwrap();
        assert!(m.contains("\"schema\":\"metodo/1\""), "{label}: method");
        eprintln!(
            "{label}: {n} spectra, {srm} SRM, TIC {} points",
            tic.time.len()
        );
        if label.contains("6460")
            || label.contains("6470")
            || label.contains("6495")
            || label.contains("BC102")
        {
            assert!(
                srm > 0 && n == 0,
                "{label}: a QqQ MRM run has SRM traces and no spectra"
            );
        }
    }
}

/// msconvert names an SRM trace `- SRM SIC Q1=73.0 Q3=45.196 start=… end=…`; its Q3 is the stored one less 0.004.
#[test]
fn srm_traces_match_msconvert() {
    let Some(root) = root() else {
        eprintln!("SKIP agilent_lcms: no data repository (MZLAB_DATI)");
        return;
    };
    let mut pairs = 0;
    for dir in runs(&root) {
        let mz = dir.with_extension("mzML");
        let Ok(f) = std::fs::File::open(&mz) else {
            continue;
        };
        let want = mzml::read(std::io::BufReader::new(f)).unwrap();
        let theirs: Vec<_> = want
            .chromatograms
            .iter()
            .filter(|c| c.id.contains("SRM SIC"))
            .collect();
        if theirs.is_empty() {
            continue;
        }
        let label = name(&dir);
        let mut l = open(&dir);
        let mut ours = Vec::new();
        for c in l.cromatogrammi(0).unwrap() {
            if c.tipo == ChromKind::Srm {
                ours.push(l.cromatogramma(0, &c.id).unwrap());
            }
        }
        let mut used = vec![false; ours.len()];
        let (mut same, mut counts, mut missing) = (0, 0, 0);
        for t in &theirs {
            let (q1, q3, _, _) = mzlab_core::mzmlhead::srm_id(&t.id);
            let (q1, q3) = (q1.unwrap(), q3.unwrap());
            // two rows of a dMRM table can share Q1 and Q3: the identical trace is the one that counts
            let cand = ours.iter().enumerate().find(|(i, o)| {
                !used[*i]
                    && (o.q1.unwrap() - q1).abs() < 0.05
                    && (o.q3.unwrap() - 0.004 - q3).abs() < 0.0015
                    && o.time.len() == t.time.len()
                    && o.time
                        .iter()
                        .zip(&t.time)
                        .all(|(a, b)| (a - b).abs() <= 1e-9)
                    && o.intensity
                        .iter()
                        .zip(&t.intensity)
                        .all(|(a, b)| (a - b).abs() <= 1e-6)
            });
            match cand {
                Some((i, _)) => {
                    used[i] = true;
                    same += 1;
                }
                None if ours.iter().any(|o| {
                    (o.q1.unwrap() - q1).abs() < 0.05 && (o.q3.unwrap() - 0.004 - q3).abs() < 0.0015
                }) =>
                {
                    counts += 1
                }
                None => missing += 1,
            }
        }
        eprintln!(
            "{label}: msconvert {} SRM, ours {}: identical {same}, other number of points {counts}, not found {missing}",
            theirs.len(),
            ours.len()
        );
        assert_eq!(
            missing, 0,
            "{label}: transitions of msconvert that we do not have"
        );
        assert!(
            same * 100 >= theirs.len() * 90,
            "{label}: only {same} of {} identical",
            theirs.len()
        );
        pairs += 1;
    }
    assert!(pairs >= 3, "{pairs} QqQ pairs");
}

fn field(h: &[mzlab_core::lettori::Campo], name: &str) -> f64 {
    h.iter()
        .find(|c| c.nome == name)
        .unwrap()
        .valore
        .parse()
        .unwrap()
}

/// The Q-TOF scans agree with their own records: the m/z of the most intense point is the base peak of the record (the record is computed with the default
/// calibration, the points with the one of the scan: 200 ppm apart at most), and a profile adds up to the TIC of the record.
#[test]
fn qtof_points_agree_with_their_records() {
    let Some(root) = root() else {
        eprintln!("SKIP agilent_lcms: no data repository (MZLAB_DATI)");
        return;
    };
    let mut checked = 0;
    for dir in runs(&root) {
        let label = name(&dir);
        if label.contains("MTBLS787") {
            continue;
        }
        let mut l = open(&dir);
        let n = l.campioni()[0].scansioni;
        if n == 0 {
            continue;
        }
        for k in (0..n).step_by((n / 40).max(1)) {
            let s = l.scansione(0, k).unwrap();
            let h = l.intestazione(0, k).unwrap();
            let (bp_mz, bp, tic) = (
                field(&h, "BasePeakMZ"),
                field(&h, "BasePeakValue"),
                field(&h, "TIC"),
            );
            if s.mz.is_empty() || bp <= 0.0 {
                continue;
            }
            let top = (0..s.mz.len())
                .max_by(|&a, &b| s.intensity_f64(a).total_cmp(&s.intensity_f64(b)))
                .unwrap();
            let ppm = (s.mz[top] - bp_mz).abs() / bp_mz * 1e6;
            // GC runs keep their own tests; here every run is LC/MS
            assert!(
                ppm < 200.0,
                "{label} #{k}: top at {} against base peak {bp_mz} ({ppm} ppm)",
                s.mz[top]
            );
            if s.profile {
                let sum: f64 = (0..s.mz.len()).map(|i| s.intensity_f64(i)).sum();
                assert!(
                    (sum - tic).abs() <= 1e-6 * tic.max(1.0) + 1.0,
                    "{label} #{k}: sum {sum}, TIC {tic}"
                );
                assert!(
                    (s.intensity_f64(top) - bp).abs() <= 1e-6 * bp + 1.0,
                    "{label} #{k}: top {} base {bp}",
                    s.intensity_f64(top)
                );
            }
            checked += 1;
        }
    }
    assert!(checked > 100, "{checked} scans checked");
}

/// The probe says `variante` for every LC/MS run (QqQ MRM, Q-TOF centroid and profile): the engine's reader cannot read their points.
#[test]
fn probe_says_variant_for_every_lc_run() {
    let Some(root) = root() else {
        eprintln!("SKIP agilent_lcms: no data repository (MZLAB_DATI)");
        return;
    };
    for dir in runs(&root) {
        let label = name(&dir);
        let motivi = mzlab_lettori::motivi_insieme(&mut set_of(&dir));
        assert!(motivi.contains(&"variante"), "{label}: {motivi:?}");
    }
}

/// The method of an LC/MS run: the transitions with their dMRM windows, the precursor lists of a Q-TOF, the gradient, the modules and the diode-array detector.
#[test]
fn the_method_carries_the_lc_ms_sections() {
    let Some(root) = root() else {
        eprintln!("SKIP agilent_lcms: no data repository (MZLAB_DATI)");
        return;
    };
    let (mut qqq, mut qtof, mut with_lc) = (0, 0, 0);
    for dir in runs(&root) {
        let label = name(&dir);
        let whole = dir.to_string_lossy().into_owned();
        if label.contains("MTBLS787") {
            continue;
        }
        let m = open(&dir).metodo(0).unwrap();
        eprintln!(
            "{label}: lc {} gradient {} modules {} sample {}",
            m.contains("\"lc\":{"),
            m.contains("\"gradiente\":["),
            m.contains("\"moduli\":["),
            m.contains("\"campione\":{\"nome\"")
        );
        with_lc += m.contains("\"gradiente\":[") as usize;
        if whole.contains("qqq_6470") {
            // a dMRM method: the retention windows of each transition
            assert!(m.contains("\"tipo\":\"MRM\""), "{label}");
            assert!(m.contains("\"rt_min\":") && m.contains("\"finestra_rt_min\":"));
            assert!(m.contains("\"frammentatore_V\":") && m.contains("\"polarita\":\"Negative\""));
            qqq += 1;
        }
        if whole.contains("MTBLS599") {
            assert!(m.contains("\"rivelatori\":[{\"tipo\":\"DAD\""), "{label}");
            assert!(m.contains("\"nm\":254"), "{label}");
            assert!(m.contains("\"liste_precursori\":["), "{label}");
            assert!(
                m.contains("\"dove\":\"AcqData/DAD1.sd\""),
                "{label}: DAD not decoded"
            );
            qtof += 1;
        }
    }
    assert_eq!((qqq, qtof), (1, 1));
    assert!(with_lc >= 12, "{with_lc} methods with a gradient");
}
