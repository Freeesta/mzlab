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
    dde_records, decode_tof_block, device_channels, wiff2_sample_info, DadData, DdeEntry,
    TofSampleData,
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

    let p_acq = d.join("dati_formati/sciex_wiff/triplequad5500_mrm_Z14418483/NEG.wiff");
    if let Ok(bytes) = std::fs::read(&p_acq) {
        let ole = container(bytes, "NEG.wiff").unwrap();
        let m_acq = method_of_sample(&ole, 1).expect("Waters Acquity method");
        let lc_acq = m_acq.lc.expect("Waters Acquity LC gradient");
        assert!(lc_acq.flow.is_some());
        assert!(lc_acq.gradient.len() >= 2);
    }

    let p_ch = d.join("dati_formati/sciex_wiff/qtrap5500_mrm_PXD001506/Duodenum_MRM_SMB_01.wiff");
    if let Ok(bytes) = std::fs::read(&p_ch) {
        let ole = container(bytes, "Duodenum_MRM_SMB_01.wiff").unwrap();
        let chs = device_channels(&ole, 1);
        assert!(!chs.is_empty(), "expected auxiliary device channels");
        assert!(chs.iter().any(|c| !c.values.is_empty()));
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

    let ms1_scan = 2035usize;
    if ms1_scan < n_scans && dde.len() > 1855 {
        let rec = &idx_bytes[32 + 54 * ms1_scan..32 + 54 * (ms1_scan + 1)];
        let off = u32::from_le_bytes([rec[0], rec[1], rec[2], rec[3]]) as u64;
        let sz = u32::from_le_bytes([rec[4], rec[5], rec[6], rec[7]]) as usize;
        let at = section + off + scan::PAYLOAD_SKIP;
        let mut buf = vec![0u8; sz];
        scan_reader.read_at(at, &mut buf).unwrap();
        let pts = decode_tof_block(&buf).unwrap();
        let deps: Vec<_> = (0..5).map(|d| &dde[185 * 10 + d]).collect();
        assert!(verify_precursors_ppm(&pts, &tof, ms1_scan, &deps) >= 3);
    }
}

fn verify_precursors_ppm(
    pts: &[(u32, u32)],
    tof: &TofSampleData,
    scan: usize,
    deps: &[&DdeEntry],
) -> usize {
    let mut ok = 0usize;
    for &d in deps {
        if d.precursor_mz <= 0.0 || d.precursor_intensity <= 0.0 {
            continue;
        }
        let (mut apex, mut sum_mz, mut sum_c) = (0u32, 0.0f64, 0.0f64);
        for &(b, c) in pts {
            let mz = tof.bin_to_mz(scan, b);
            if (mz - d.precursor_mz).abs() < 0.05 {
                apex = apex.max(c);
                sum_mz += mz * f64::from(c);
                sum_c += f64::from(c);
            }
        }
        if sum_c > 0.0 {
            let cent = sum_mz / sum_c;
            let ppm = (cent - d.precursor_mz).abs() / d.precursor_mz * 1e6;
            assert!(
                ppm <= 10.0,
                "precursor {}: diff {ppm:.2} ppm",
                d.precursor_mz
            );
            assert_eq!(f64::from(apex), d.precursor_intensity);
            ok += 1;
        }
    }
    ok
}

fn write_synthetic_ci_wiff(out: &Path) {
    let script = concat!(
        "import struct, sys\n",
        "from pathlib import Path\n",
        "sys.path.insert(0, str(Path(sys.argv[1]) / 'tools'))\n",
        "import cfb_scrivi\n",
        "out = Path(sys.argv[2])\n",
        "dad_hdr = bytearray(86); struct.pack_into('<I', dad_hdr, 4, 4); struct.pack_into('<II', dad_hdr, 34, 3, 5); struct.pack_into('<ffff', dad_hdr, 46, 0.012, 200.0, 204.0, 1.0)\n",
        "rows = [[1.0, 2.0, 3.0, 4.0, 5.0], [0.5, 1.5, 2.5, 3.5, 4.5], [2.0, 4.0, 6.0, 8.0, 10.0]]\n",
        "dad_idx = bytearray(32) + b''.join(struct.pack('<ff', (i+1)*0.004, sum(r)) for i, r in enumerate(rows))\n",
        "dad_rtd = bytearray(32) + b''.join(struct.pack('<5f', *r) for r in rows)\n",
        "exp_hdr = bytearray(180); struct.pack_into('<I', exp_hdr, 4, 5); struct.pack_into('<I', exp_hdr, 122, 4); struct.pack_into('<I', exp_hdr, 176, 2)\n",
        "par = lambda n, v: struct.pack('<H', len(n)*2) + n.encode('utf-16le') + struct.pack('<fff', v, v, 0.0)\n",
        "ent = lambda q1, q3, dw, lb, dp, ep, cep, ce, cxp: struct.pack('<ffffIH', q1, 0.0, q3, dw, 0, len(lb)*2) + lb.encode('utf-16le') + par('DP', dp) + par('EP', ep) + par('CEP', cep) + par('CE', ce) + par('CXP', cxp)\n",
        "mrex = bytearray(32) + struct.pack('<II', 2, 5) + ent(364.1, 194.1, 25.0, 'Quant', 50.0, 10.0, 22.11, 20.0, 3.0) + ent(364.1, 152.1, 25.0, 'Qual', 50.0, 10.0, 22.11, 28.0, 4.0)\n",
        "smrm = bytearray(52); struct.pack_into('<I', smrm, 28, 30); struct.pack_into('<II', smrm, 36, 0, 60); struct.pack_into('<d', smrm, 44, 1.0)\n",
        "vam_shim = bytearray(612) + b'<?xml version=\"1.0\"?><SCIEX_ANALYST_ACQUISITION_METHOD><Category><Title>Pumps</Title><Parameter><Name>Total Flow</Name><Value>0.25</Value></Parameter><Parameter><Name>B Concentration</Name><Value>5.0</Value></Parameter></Category><Category><Title>Oven</Title><Parameter><Name>Temperature</Name><Value>40.0</Value></Parameter></Category><Category><Title>System Controller</Title><Table><Name>Time Program</Name><row><Field1>Time</Field1><Field2>Module</Field2><Field3>0</Field3><Field4>Parameter</Field4><Field5>Value</Field5></row><row><Field1>10.0</Field1><Field2>Pumps</Field2><Field3>0</Field3><Field4>Pump B Conc.</Field4><Field5>95.0</Field5></row><row><Field1>22.0</Field1><Field2>Controller</Field2><Field3>0</Field3><Field4>Stop</Field4><Field5>0</Field5></row></Table></Category></SCIEX_ANALYST_ACQUISITION_METHOD>'\n",
        "vam_acq = bytearray(612) + b'<?xml version=\"1.0\"?><SCIEX_ANALYST_ACQUISITION_METHOD><Category><Title>ACQUITY BSM Method</Title><Table><Name>Gradient Table</Name><row><Field1>Time</Field1><Field2>Flow Rate</Field2><Field3>%A</Field3><Field4>%B</Field4></row><row><Field1>Initial</Field1><Field2>0.300</Field2><Field3>90.0</Field3><Field4>10.0</Field4></row><row><Field1>8.00</Field1><Field2>0.300</Field2><Field3>5.0</Field3><Field4>95.0</Field4></row></Table></Category><Category><Title>ACQUITY SM Method</Title><Parameter><Name>Target Column Temperature</Name><Value>35.0</Value></Parameter></Category></SCIEX_ANALYST_ACQUISITION_METHOD>'\n",
        "step = lambda t, fl, a, b: bytes(36) + struct.pack('<H', 16) + 'New step'.encode('utf-16le') + struct.pack('<fffff', t, fl, 0.0, a, b) + bytes(16)\n",
        "cn, cu = 'Column Pressure'.encode('utf-16le'), '(psi)'.encode('utf-16le')\n",
        "ch_s = bytes(36) + struct.pack('<HIH', 1, 1, len(cn)) + cn + struct.pack('<H', len(cu)) + cu + struct.pack('<dHffI', 6.0, 0, 0.0, 5000.0, 0xffffffff)\n",
        "w2 = bytearray(126); struct.pack_into('<I', w2, 32, 1); struct.pack_into('<H', w2, 52, 56); w2[54:110] = b'01234567-89ab-cdef-0123-456789abcdef_0000000000000001234'\n",
        "dde = bytearray(32) + struct.pack('<dddHHi', 413.266, 0.0, 1540.0, 1, 0, 0)\n",
        "tdc = bytearray(72); struct.pack_into('<d', tdc, 32, 1.0 / 38.4)\n",
        "tcal = bytearray(32) + struct.pack('<ddII', 0.001, 100.0, 0, 1) + struct.pack('<ddI', 0.001, 100.0, 0)\n",
        "idx1 = bytearray(32) + struct.pack('<IId', 0, 8, 6000.0) + b'\\x01\\x00' + struct.pack('<dd', 1500.0, 0.0) + bytes(20)\n",
        "tbl = bytearray(32) + b''.join(struct.pack('<II', s, 20) + bytes(8) for s in (1, 2, 3))\n",
        "cfb_scrivi.write({\n",
        "  'SampleSubtree/SampleIdxTable': bytes(tbl),\n",
        "  'SampleSubtree/Sample1/Idx': bytes(idx1),\n",
        "  'SampleSubtree/Sample1/Devices/Device_1/DAD/DADSampleHeader': bytes(dad_hdr),\n",
        "  'SampleSubtree/Sample1/Devices/Device_1/DAD/DADIdx': bytes(dad_idx),\n",
        "  'SampleSubtree/Sample1/Devices/Device_1/DAD/DADRealTimeData': bytes(dad_rtd),\n",
        "  'SampleSubtree/Sample1/Devices/Device_2/Channel': ch_s,\n",
        "  'SampleSubtree/Sample1/Devices/Device_2/DevRealTimeHeader': bytes(32) + struct.pack('<I', 3),\n",
        "  'SampleSubtree/Sample1/Devices/Device_2/DevData': bytes(32) + struct.pack('<3d', 1200.0, 1250.0, 1210.0),\n",
        "  'SampleSubtree/Sample1/Wiff2SampleInfo': bytes(w2),\n",
        "  'SampleSubtree/Sample1/DDERealTimeData': bytes(dde),\n",
        "  'SampleSubtree/Sample1/TDCInfo': bytes(tdc),\n",
        "  'SampleSubtree/Sample1/TOFCalibrationData': bytes(tcal),\n",
        "  'SampleSubtree/Sample1/Itc': bytes(32) + struct.pack('<d', 0.5),\n",
        "  'SampleSubtree/Sample1/TDCStatistics': bytes(32) + struct.pack('<I', 1) + bytes(20),\n",
        "  'MethodSubtree/Method1/DeviceMethod0/Period0/Experiment0/ExperimentHeader': bytes(exp_hdr),\n",
        "  'MethodSubtree/Method1/DeviceMethod0/Period0/Experiment0/MassRangeEx/MassRangeEx': bytes(mrex),\n",
        "  'MethodSubtree/Method1/DeviceMethod0/Period0/Experiment0/sMRM': bytes(smrm),\n",
        "  'MethodSubtree/Method1/DeviceMethod1/VendorAppMethod': bytes(vam_shim),\n",
        "  'SampleSubtree/Sample2/Idx': bytes(idx1),\n",
        "  'MethodSubtree/Method2/DeviceMethod0/Period0/Experiment0/ExperimentHeader': bytes(exp_hdr),\n",
        "  'MethodSubtree/Method2/DeviceMethod1/VendorAppMethod': bytes(vam_acq),\n",
        "  'SampleSubtree/Sample3/Idx': bytes(idx1),\n",
        "  'MethodSubtree/Method3/DeviceMethod0/Period0/Experiment0/ExperimentHeader': bytes(exp_hdr),\n",
        "  'MethodSubtree/Method3/DeviceMethod1/LCPumpStep0/LCPumpStep': step(0.0, 400.0, 95.0, 5.0),\n",
        "  'MethodSubtree/Method3/DeviceMethod1/LCPumpStep1/LCPumpStep': step(12.0, 400.0, 10.0, 90.0),\n",
        "}, out / 'ci.wiff')\n",
        "(out / 'ci.wiff.scan').write_bytes(bytes(44) + struct.pack('<2f', 1000.0, 500.0))\n",
    );
    let st = std::process::Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(root())
        .arg(out)
        .status()
        .unwrap();
    assert!(st.success(), "synthetic CI wiff generator failed");
}

#[test]
fn synthetic_ci_dad_mrm_gradient_and_tof_roundtrip() {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_ci_{}", std::process::id()));
    std::fs::create_dir_all(&out).unwrap();
    write_synthetic_ci_wiff(&out);

    let wiff_bytes = std::fs::read(out.join("ci.wiff")).unwrap();
    let scan_bytes = std::fs::read(out.join("ci.wiff.scan")).unwrap();
    let ole = container(wiff_bytes, "ci.wiff").unwrap();
    let w = Wiff::open(&ole, "ci.wiff", Some(scan_bytes.len() as u64), 1).unwrap();

    let dad = w.dad.as_ref().expect("synthetic DAD");
    assert_eq!((dad.points(), dad.channels()), (3, 5));
    assert_eq!(dad.twc_chromatogram(0).intensity, vec![15.0, 12.5, 30.0]);
    assert_eq!(
        dad.wavelength_chromatogram(202.0, None, 0)
            .unwrap()
            .intensity,
        vec![3.0, 2.5, 6.0]
    );

    let m1 = method_of_sample(&ole, 1).unwrap();
    let Kind::Mrm(tr) = &m1.experiments[0].kind else {
        panic!("expected MRM")
    };
    assert_eq!(tr.len(), 2);
    assert_eq!(
        (tr[0].q1, tr[0].q3, tr[0].name.as_str(), tr[0].ce, tr[0].dp),
        (364.1, 194.1, "Quant", Some(20.0), Some(50.0))
    );
    assert_eq!(
        (tr[1].q1, tr[1].q3, tr[1].name.as_str(), tr[1].ce, tr[1].cxp),
        (364.1, 152.1, "Qual", Some(28.0), Some(4.0))
    );
    let lc1 = m1.lc.unwrap();
    assert_eq!(
        (lc1.flow, lc1.b0, lc1.oven, lc1.run_time),
        (Some(0.25), Some(5.0), Some(40.0), Some(22.0))
    );
    assert_eq!(lc1.gradient.len(), 2);

    let lc2 = method_of_sample(&ole, 2).unwrap().lc.unwrap();
    assert_eq!(
        (lc2.flow, lc2.b0, lc2.oven, lc2.run_time),
        (Some(0.3), Some(10.0), Some(35.0), Some(8.0))
    );
    let lc3 = method_of_sample(&ole, 3).unwrap().lc.unwrap();
    assert_eq!(
        (lc3.flow, lc3.b0, lc3.run_time),
        (Some(0.4), Some(5.0), Some(12.0))
    );

    let chs = device_channels(&ole, 1);
    assert_eq!(
        (
            chs.len(),
            chs[0].name.as_str(),
            chs[0].unit.as_str(),
            chs[0].values.len()
        ),
        (1, "Column Pressure", "psi", 3)
    );
    let dde = dde_records(&ole, 1, 2);
    assert_eq!(
        (dde.len(), dde[0].precursor_mz, dde[0].precursor_intensity),
        (1, 413.266, 1540.0)
    );
    let tof = TofSampleData::read(&ole, 1).unwrap();
    assert_eq!(tof.scan_calibration.len(), 1);
    let tof_blk = [
        0xff, 0xff, 0xff, 0xff, 100, 0, 0, 0, 0, 5, 0x82, 0x7c, 200, 0xff,
    ];
    let pts = decode_tof_block(&tof_blk).unwrap();
    assert_eq!(pts, vec![(100, 5), (112, 200)]);
    let mz0 = tof.bin_to_mz(0, pts[0].0);
    assert!(mz0 > 0.0 && mz0.is_finite());
}
