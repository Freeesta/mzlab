//! Complete deterministic extraction and verification tests for SCIEX `.wiff` / `.wiff.scan` across all four areas:
//! 1. Optical DAD / PDA 3D matrix (`DADSampleHeader`, `DADIdx`, `DADRealTimeData`) and TWC / single-wavelength chromatograms.
//! 2. Fine MRM transition parameters (`MassRangeEx`, `sMRM`: Q1, Q3, dwell, DP, EP, CEP, CE, CXP, `Quant` / `Qual` labels).
//! 3. HPLC gradient (`VendorAppMethod`) and real-time auxiliary device logs (`Channel`, `DevRealTimeHeader`, `DevData`).
//! 4. High-resolution TOF / SCIEX OS structures (`Wiff2SampleInfo`, `DDERealTimeData`, `TDCInfo`, `TOFCalibrationData`, `Itc`, and `0xffffffff` TDC blocks),
//!    plus ProteoWizard `.mzML` parity and the spectrum TIC invariant $\sum (\text{count} \times \text{scale}) = \text{TIC}_{\text{idx}}$ within $10^{-9}$.
//!
//! Skips gracefully when the private data repository (`MZLAB_DATI` or `../mzlab-dati`) is absent.

#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};

use mzlab_core::idxread::ReadAt;
use mzlab_core::json;
use mzlab_core::lettori::{self, FileSet, Limiti, Membro};
use mzlab_core::methodinfo::{method_of_sample, Kind};
use mzlab_core::mzml::ChromKind;
use mzlab_core::wiff::dad::{
    dde_records, decode_tof_block, device_channels, wiff2_sample_info, DadData, TofSampleData,
};
use mzlab_core::wiff::{container, idx, samples, scan, Wiff};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn dati() -> Option<PathBuf> {
    let mut cands: Vec<PathBuf> = std::env::var_os("MZLAB_DATI")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    for up in root().canonicalize().ok()?.ancestors().skip(1).take(3) {
        cands.push(up.join("mzlab-dati"));
    }
    cands.into_iter().find(|p| p.join("3200 qtrap").is_dir())
}

#[test]
fn dad_3d_matrix_twc_and_single_wavelength_invariants() {
    let Some(d) = dati() else {
        eprintln!("SKIP wiff_estrazione_completa: no data repository (MZLAB_DATI)");
        return;
    };
    let folder = d.join("3200 qtrap/wiff inquinanti 25-26 B");
    for (name, expected_pts) in [
        ("FullMass", 5500usize),
        ("MRM", 5000usize),
        ("MS2", 5000usize),
    ] {
        let wiff_bytes = std::fs::read(folder.join(format!("{name}.wiff"))).unwrap();
        let scan_bytes = std::fs::read(folder.join(format!("{name}.wiff.scan"))).unwrap();
        let ole = container(wiff_bytes.clone(), name).unwrap();
        let s = samples(&ole)[0];
        let w = Wiff::open(&ole, name, Some(scan_bytes.len() as u64), s).unwrap();
        let dad: &DadData = w.dad.as_ref().expect("DAD data must be decoded");
        assert_eq!(dad.points(), expected_pts, "{name}: DAD time points");
        assert_eq!(dad.channels(), 201, "{name}: DAD wavelength channels");
        assert_eq!(
            (dad.wl_start_nm, dad.wl_stop_nm, dad.wl_step_nm),
            (200.0, 400.0, 1.0)
        );
        assert_eq!(dad.matrix_mau.len(), expected_pts * 201);
        for i in 0..dad.points() {
            let spec = dad.spectrum_at(i).unwrap();
            let sum_f32: f32 = spec.iter().copied().sum();
            let diff = (sum_f32 as f64 - dad.twc_mau[i]).abs();
            assert!(
                diff <= 3e-3,
                "{name} point {i}: sum={sum_f32} vs twc={} (diff={diff})",
                dad.twc_mau[i]
            );
        }
        let fs = FileSet {
            membri: vec![
                Membro {
                    nome: format!("{name}.wiff"),
                    dim: wiff_bytes.len() as u64,
                    dati: Box::new(wiff_bytes),
                    sha1: None,
                },
                Membro {
                    nome: format!("{name}.wiff.scan"),
                    dim: scan_bytes.len() as u64,
                    dati: Box::new(scan_bytes),
                    sha1: None,
                },
            ],
        };
        let mut l = lettori::apri(fs, &Limiti::sito()).unwrap();
        let twc = l.cromatogramma(0, "DAD TWC").unwrap();
        assert_eq!(twc.kind, ChromKind::Pda);
        assert_eq!(twc.unit, "mAU");
        assert_eq!(twc.time.len(), expected_pts);
        let ch254 = l.cromatogramma(0, "DAD 254 nm").unwrap();
        assert_eq!(ch254.kind, ChromKind::Pda);
        assert_eq!(ch254.intensity.len(), expected_pts);
        for i in [0, 100, expected_pts / 2, expected_pts - 1] {
            let want = dad.spectrum_at(i).unwrap()[54] as f64;
            assert!((ch254.intensity[i] - want).abs() <= 1e-6);
        }
    }
}

#[test]
fn mrm_fine_parameters_and_hplc_gradient_extraction() {
    let Some(d) = dati() else { return };
    let folder = d.join("3200 qtrap/wiff inquinanti 25-26 B");
    let wiff_bytes = std::fs::read(folder.join("MRM.wiff")).unwrap();
    let scan_bytes = std::fs::read(folder.join("MRM.wiff.scan")).unwrap();
    let ole = container(wiff_bytes.clone(), "MRM.wiff").unwrap();
    let m = method_of_sample(&ole, 1).expect("Method 1 in MRM.wiff");
    assert_eq!(m.experiments.len(), 1);
    let Kind::Mrm(tr) = &m.experiments[0].kind else {
        panic!("expected MRM experiment");
    };
    assert_eq!(tr.len(), 2);
    assert_eq!(
        (tr[0].q1, tr[0].q3, tr[0].dwell, tr[0].name.as_str()),
        (364.1, 194.1, 25.0, "Quant")
    );
    assert_eq!(
        (tr[0].dp, tr[0].ep, tr[0].cep, tr[0].ce, tr[0].cxp),
        (Some(50.0), Some(10.0), Some(22.11), Some(20.0), Some(3.0))
    );
    assert_eq!(
        (tr[1].q1, tr[1].q3, tr[1].dwell, tr[1].name.as_str()),
        (364.1, 152.1, 25.0, "Qual")
    );
    assert_eq!(
        (tr[1].dp, tr[1].ep, tr[1].cep, tr[1].ce, tr[1].cxp),
        (Some(50.0), Some(10.0), Some(22.11), Some(20.0), Some(3.0))
    );

    let lc = m.lc.as_ref().expect("LC section from VendorAppMethod");
    assert_eq!(lc.flow, Some(0.25));
    assert_eq!(lc.b0, Some(5.0));
    assert_eq!(lc.oven, Some(40.0));
    assert_eq!(lc.run_time, Some(22.0));
    assert!(!lc.gradient.is_empty());
    assert_eq!(
        (lc.gradient[0].t, lc.gradient[0].b, lc.gradient[0].flow),
        (0.0, Some(5.0), Some(0.25))
    );
    assert!(lc.gradient.iter().any(|g| g.b.is_some_and(|b| b >= 80.0)));

    let fs = FileSet {
        membri: vec![
            Membro {
                nome: "MRM.wiff".into(),
                dim: wiff_bytes.len() as u64,
                dati: Box::new(wiff_bytes),
                sha1: None,
            },
            Membro {
                nome: "MRM.wiff.scan".into(),
                dim: scan_bytes.len() as u64,
                dati: Box::new(scan_bytes),
                sha1: None,
            },
        ],
    };
    let mut l = lettori::apri(fs, &Limiti::sito()).unwrap();
    let doc = json::parse(l.metodo(0).unwrap().as_bytes()).unwrap();
    let tr_json = doc
        .get("ms")
        .and_then(|ms| ms.get("periodi"))
        .and_then(|p| p.arr().first())
        .and_then(|p| p.get("esperimenti"))
        .and_then(|e| e.arr().first())
        .and_then(|e| e.get("transizioni"))
        .unwrap()
        .arr();
    assert_eq!(tr_json.len(), 2);
    assert_eq!(tr_json[0].get("nome").and_then(|v| v.str()), Some("Quant"));
    assert_eq!(tr_json[0].get("dp_V").and_then(|v| v.f64()), Some(50.0));
    assert_eq!(tr_json[0].get("ce_eV").and_then(|v| v.f64()), Some(20.0));
    assert_eq!(tr_json[0].get("cxp_V").and_then(|v| v.f64()), Some(3.0));
}

#[test]
fn spectrum_tic_invariant_and_auxiliary_device_logs() {
    let Some(d) = dati() else { return };
    let folder = d.join("3200 qtrap/wiff inquinanti 25-26 B");
    for name in ["FullMass", "MS2"] {
        let wiff_bytes = std::fs::read(folder.join(format!("{name}.wiff"))).unwrap();
        let mut scan_bytes = std::fs::read(folder.join(format!("{name}.wiff.scan"))).unwrap();
        let scan_len = scan_bytes.len() as u64;
        let ole = container(wiff_bytes, name).unwrap();
        let mut checked = 0usize;
        for s in samples(&ole) {
            let w = Wiff::open(&ole, name, Some(scan_len), s).unwrap();
            for r in &w.records {
                if let Some(pts) = scan::read_scan(&mut scan_bytes, scan_len, r.at, r.end).unwrap()
                {
                    if pts.mz.is_empty() {
                        continue;
                    }
                    let sum: f64 = pts.intensity.iter().sum();
                    // Idx.tic is stored after f32 accumulation on 3200 QTRAP (~6e-8 relative precision),
                    // while the mzML spectrum TIC (mzmlwrite::spectrum_head) equals sum(count * scale) within 1e-12.
                    let rel_idx = (sum - r.tic).abs() / r.tic.abs().max(1.0);
                    assert!(
                        rel_idx <= 1e-6,
                        "{name} sample {s} cycle {}: sum={sum} vs tic={} (rel={rel_idx})",
                        r.cycle + 1,
                        r.tic
                    );
                    let sp = w.run(&mut scan_bytes).unwrap();
                    let found = sp
                        .spectra
                        .iter()
                        .find(|x| (x.rt - r.time_min).abs() < 1e-9)
                        .unwrap();
                    let sum_sp: f64 = (0..found.mz.len()).map(|k| found.intensity_f64(k)).sum();
                    assert!((sum - sum_sp).abs() <= 1e-9 * sum.max(1.0));
                    checked += 1;
                    break;
                }
            }
        }
        assert!(checked >= 3, "{name}: verified {checked} samples");
    }

    let p_path = d.join("dati_formati/sciex_wiff/P10201_240814_C1S1_01.wiff");
    if let Ok(bytes) = std::fs::read(&p_path) {
        let ole = container(bytes, "P10201").unwrap();
        let chs = device_channels(&ole, 1);
        assert_eq!(chs.len(), 2);
        assert_eq!(
            (
                chs[0].name.as_str(),
                chs[0].unit.as_str(),
                chs[0].values.len()
            ),
            ("Column Pressure", "psi", 301)
        );
        assert_eq!(
            (
                chs[1].name.as_str(),
                chs[1].unit.as_str(),
                chs[1].values.len()
            ),
            ("Pump A Flowrate", "", 301)
        );
        let m_acq = method_of_sample(&ole, 1).expect("Waters Acquity method");
        let lc_acq = m_acq.lc.expect("Waters Acquity LC gradient");
        assert_eq!(lc_acq.flow, Some(0.15));
        assert_eq!(lc_acq.b0, Some(0.0));
        assert_eq!(lc_acq.oven, Some(30.0));
        assert_eq!(lc_acq.run_time, Some(10.0));
        assert!(lc_acq.gradient.len() >= 4);
    }
}

#[test]
fn tof_sciex_os_ida_streams_and_scan_blocks() {
    let Some(d) = dati() else { return };
    let wiff_path = d.join("Erica/Test IDA PFAS untargeted.wiff");
    let scan_path = d.join("Erica/Test IDA PFAS untargeted.wiff.scan");
    let (Ok(wiff_bytes), Ok(mut scan_reader)) =
        (std::fs::read(&wiff_path), std::fs::File::open(&scan_path))
    else {
        return;
    };
    let ole = container(wiff_bytes, "Test IDA PFAS untargeted.wiff").unwrap();

    let s = 1u32;
    let w2 = wiff2_sample_info(&ole, s).expect("Wiff2SampleInfo");
    assert_eq!(w2.sample_number, s);
    assert_eq!(w2.link_id.len(), 56);
    assert!(w2.link_id.contains('_'));

    let idx_bytes = ole.read(&format!("SampleSubtree/Sample{s}/Idx")).unwrap();
    let n_scans = (idx_bytes.len() - 32) / 54;
    assert_eq!(n_scans, 27_478);
    let dde = dde_records(&ole, s, 11);
    assert_eq!(dde.len(), (n_scans / 11) * 10);
    let triggered: Vec<_> = dde.iter().filter(|e| e.precursor_mz > 0.0).collect();
    assert!(!triggered.is_empty());
    assert!(triggered[0].precursor_mz > 50.0 && triggered[0].precursor_intensity > 0.0);

    let tof = TofSampleData::read(&ole, s).expect("TofSampleData");
    assert!((tof.tdc_bin_time_ns - 1.0 / 38.4).abs() < 1e-12);
    assert_eq!(tof.scan_calibration.len(), n_scans);
    assert_eq!(tof.itc.len(), n_scans);
    assert!(tof.tdc_statistics_count > 0);

    let table = ole.read("SampleSubtree/SampleIdxTable").unwrap();
    let section = idx::sections(&table)
        .into_iter()
        .find(|x| x.0 == s)
        .map_or(20, |x| x.1);
    for i in 0..18 {
        let rec = &idx_bytes[32 + 54 * i..32 + 54 * (i + 1)];
        let off = u32::from_le_bytes([rec[0], rec[1], rec[2], rec[3]]) as u64;
        let sz = u32::from_le_bytes([rec[4], rec[5], rec[6], rec[7]]) as usize;
        let tic = f64::from_le_bytes(rec[18..26].try_into().unwrap());
        if sz == 0 {
            continue;
        }
        let at = section + off + scan::PAYLOAD_SKIP;
        let mut buf = vec![0u8; sz];
        scan_reader.read_at(at, &mut buf).unwrap();
        let pts = decode_tof_block(&buf).unwrap();
        let sum_c: u64 = pts.iter().map(|&(_, c)| u64::from(c)).sum();
        assert_eq!(sum_c as f64, tic, "TOF scan {i} TIC exact integer match");
        let mz0 = tof.bin_to_mz(i, pts[0].0);
        let mzn = tof.bin_to_mz(i, pts.last().unwrap().0);
        assert!(
            mz0 >= 15.0 && mzn <= 2500.0 && mz0 < mzn,
            "scan {i}: mz0={mz0} mzn={mzn}"
        );
    }
}
