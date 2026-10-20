#![cfg(feature = "thermo")]
//! The Thermo TSQ MRM run PXD043691 `201208_15.raw` (v66 file, one event per scan with Q1, collision energy and seven product ranges) read
//! against two external oracles: the spectra of ThermoRawFileParser (`trfp/201208_15.mzML`) and the 14 SRM chromatograms of msconvert.
//! Needs the real file and the oracles under `rosina-dati` (`MZLAB_ROSINA_DATI`, or a folder of that name above the repository);
//! a part whose file is missing is skipped. Nothing here is a synthetic file.
#![allow(clippy::indexing_slicing)]

use std::path::PathBuf;

use mzlab_core::lettori::{FileSet, Limiti, Membro};
use mzlab_core::mzml::{self, ChromKind, Spectrum};
use mzlab_core::mzmlhead::Mzml;

const RAW: &str = "thermo/raw/PXD043691_mrm/201208_15.raw";
const TRFP: &str = "_oracoli/trfp/201208_15.mzML";
const MSCONVERT: &str =
    "_oracoli/msconvert_windows/centroidi/thermo/raw/PXD043691_mrm/201208_15.mzML";

fn dati(rel: &str) -> Option<PathBuf> {
    let mut radici: Vec<PathBuf> = std::env::var_os("MZLAB_ROSINA_DATI")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    let qui = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    radici.extend(
        qui.ancestors()
            .skip(1)
            .take(8)
            .map(|a| a.join("rosina-dati")),
    );
    radici
        .into_iter()
        .map(|r| r.join(rel))
        .find(|p| p.is_file())
}

fn lettore(raw: &PathBuf) -> Box<dyn mzlab_core::lettori::Lettore> {
    let bytes = std::fs::read(raw).unwrap();
    let set = FileSet {
        membri: vec![Membro {
            nome: "201208_15.raw".into(),
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }],
    };
    mzlab_lettori::apri_nudo(set, &Limiti::sito()).unwrap()
}

fn close(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * b.abs().max(1e-12)
}

#[test]
fn the_scans_match_thermorawfileparser() {
    let (Some(raw), Some(oracle)) = (dati(RAW), dati(TRFP)) else {
        eprintln!("SKIP raw_srm_tsq spectra: real file or ThermoRawFileParser oracle absent");
        return;
    };
    let want = mzml::read(std::fs::File::open(oracle).unwrap())
        .unwrap()
        .spectra;
    let got = mzlab_core::raw::read(std::fs::File::open(raw).unwrap())
        .unwrap()
        .spectra;
    assert_eq!(got.len(), want.len());
    let seven = |s: &Spectrum| s.mz.len();
    for (g, w) in got.iter().zip(&want) {
        let at = format!("scan {}", w.index);
        assert_eq!(g.level, w.level, "{at}");
        assert_eq!(g.filter, w.filter, "{at}: filter");
        assert!(
            g.precursor
                .zip(w.precursor)
                .is_some_and(|(a, b)| (a - b).abs() < 1e-3),
            "{at}: Q1"
        );
        assert!((g.rt - w.rt).abs() < 1e-4, "{at}: rt");
        assert_eq!(seven(g), 7, "{at}: 7 transitions");
        assert_eq!(seven(g), seven(w), "{at}: points");
        for i in 0..seven(w) {
            assert!((g.mz[i] - w.mz[i]).abs() < 1e-3, "{at}: Q3 {i}");
            assert!(
                close(f64::from(g.intensity[i]), f64::from(w.intensity[i]), 1e-5),
                "{at}: intensity {i}"
            );
        }
    }
}

#[test]
fn the_srm_traces_match_msconvert() {
    let (Some(raw), Some(oracle)) = (dati(RAW), dati(MSCONVERT)) else {
        eprintln!("SKIP raw_srm_tsq traces: real file or msconvert oracle absent");
        return;
    };
    // msconvert names a transition `SRM SIC <Q1>,<Q3>`
    let size = std::fs::metadata(&oracle).unwrap().len();
    let mut m = Mzml::new(std::fs::File::open(&oracle).unwrap(), size);
    let mut want: Vec<(f64, f64, Vec<f64>, Vec<f64>)> = Vec::new();
    for c in m
        .chromatograms(0)
        .unwrap()
        .iter()
        .filter(|c| c.id.starts_with("SRM SIC "))
    {
        let (q1, q3) = c.id["SRM SIC ".len()..].split_once(',').unwrap();
        let (t, y) = m.chromatogram(c).unwrap();
        want.push((q1.parse().unwrap(), q3.parse().unwrap(), t, y));
    }
    assert_eq!(want.len(), 14);
    let mut l = lettore(&raw);
    let mut got = Vec::new();
    for v in l
        .cromatogrammi(0)
        .unwrap()
        .iter()
        .filter(|v| v.tipo == ChromKind::Srm)
    {
        got.push(l.cromatogramma(0, &v.id).unwrap());
    }
    assert_eq!(got.len(), want.len(), "number of transitions");
    for (q1, q3, t, y) in &want {
        let g = got
            .iter()
            .find(|g| {
                g.q1.is_some_and(|a| (a - q1).abs() < 1e-3)
                    && g.q3.is_some_and(|a| (a - q3).abs() < 1e-3)
            })
            .unwrap_or_else(|| panic!("no trace for Q1={q1} Q3={q3}"));
        assert_eq!(g.time.len(), 46);
        assert_eq!(g.time.len(), t.len());
        for i in 0..t.len() {
            assert!((g.time[i] - t[i]).abs() < 1e-4, "Q1={q1} Q3={q3}: time {i}");
            assert!(
                close(g.intensity[i], y[i], 1e-5),
                "Q1={q1} Q3={q3}: intensity {i}"
            );
        }
    }
}
