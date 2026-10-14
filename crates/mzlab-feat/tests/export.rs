//! The exports of a table: the CSV "full", the FBMN quantification table, the MGF and the SIRIUS `.ms`; and the MS2 chosen for a row from the index
//! of a synthetic series (skipped without python3 and numpy, `MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::idxread::Index;
use mzlab_feat::table::{RowIdent, RowQ, NO_FILE};
use mzlab_feat::export::{self, csv_full, quant, Ctx};
use mzlab_feat::export_ms::{mgf, mgf_unit, sirius, Ms2};
use mzlab_feat::filters::Rule;
use mzlab_feat::norm::Method;
use mzlab_feat::pipeline::{analyse, Steps};
use mzlab_feat::samples::{propose, Sample};
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED, CELL_FILLED, CELL_TARGETED};
use mzlab_feat::trace::Params;
use mzlab_feat::view;

fn table() -> Aligned {
    let rows: [(f64, f64, [f64; 3], [u8; 3]); 2] = [
        (200.123456789, 5.5, [1.23456789e6, 2e6, 3e6], [CELL_DETECTED, CELL_FILLED, CELL_TARGETED]),
        (300.2, 7.0, [0.0, 0.0, 4e5], [CELL_ABSENT, CELL_ABSENT, CELL_DETECTED]),
    ];
    let mut t = Aligned::with_capacity(3, 2);
    for (mz, rt, areas, states) in rows {
        t.mz.push(mz);
        t.rt.push(rt);
        for (a, s) in areas.iter().zip(states) {
            t.area.push(*a);
            t.height.push((*a / 10.0) as f32);
            t.apex_rt.push(rt as f32);
            t.width.push(0.1);
            t.state.push(s);
        }
    }
    t
}

fn samples() -> Vec<Sample> {
    let f: Vec<(String, String)> = ["a,b.mzML", "t10.mzML", "t20.mzML"].iter().map(|n| (n.to_string(), "00".repeat(32))).collect();
    propose(&f)
}

#[test]
fn csv_full_writes_the_exact_state_of_every_cell() {
    let (t, s) = (table(), samples());
    let c = Ctx { table: &t, samples: &s, view: None, table_sha256: "ab", polarity: 1 };
    let csv = csv_full(&c, false);
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines[0], "id,mz,rt,rt_range:min,rt_range:max,charge,\"datafile:a,b:area\",\"datafile:a,b:height\",\"datafile:a,b:feature_state\",datafile:t10:area,datafile:t10:height,datafile:t10:feature_state,datafile:t20:area,datafile:t20:height,datafile:t20:feature_state");
    assert_eq!(lines[1], "1,200.12346,5.5,,,,1234567.9,123456.79,DETECTED,2000000,200000,FILLED,3000000,300000,TARGETED");
    assert_eq!(lines[2], "2,300.2,7,,,,,,UNKNOWN,,,UNKNOWN,400000,40000,DETECTED");
    assert!(csv_full(&c, true).lines().nth(1).unwrap().ends_with("ESTIMATED,3000000,300000,ESTIMATED"));
    assert_eq!(csv_full(&c, false), csv_full(&c, false));
}

#[test]
fn quant_follows_the_view_and_names_the_ion_only_when_known() {
    let (mut t, s) = (table(), samples());
    t.ident = RowIdent { neutral: vec![199.1156, 0.0], ion: vec![1, 1], src: vec![3, 1] };
    let v = view::build(&t, &s, &Method::None, &[Rule::MinArea(1e6)]).unwrap();
    let c = Ctx { table: &t, samples: &s, view: Some(&v), table_sha256: "ab", polarity: 1 };
    let q = quant(&c);
    let lines: Vec<&str> = q.lines().collect();
    assert_eq!(lines.len(), 2, "the second row was rejected by the filter");
    assert!(lines[0].ends_with("\"a,b.mzML Peak area\",t10.mzML Peak area,t20.mzML Peak area"));
    assert_eq!(lines[1], "1,200.12346,5.5,-1,-1,[M+H]+,199.1156,1234567.9,2000000,3000000");
}

#[test]
fn mgf_and_sirius_hold_only_the_rows_with_a_spectrum() {
    let (t, s) = (table(), samples());
    let c = Ctx { table: &t, samples: &s, view: None, table_sha256: "ab", polarity: -1 };
    let sp = vec![None, Some(Ms2 { prec: 300.2001, mz: vec![100.0, 150.123456], it: vec![10.0, 55.56], files: 1 })];
    let m = mgf(&c, &sp);
    assert!(m.starts_with("# Tabui feature table"));
    assert!(m.contains("BEGIN IONS\nFEATURE_ID=2\nPEPMASS=300.2001\nSCANS=2\nRTINSECONDS=420.00\nCHARGE=1-\nMSLEVEL=2\nIONMODE=Negative\nMERGED_ACROSS_N_SAMPLES=1\n100 10\n150.12346 55.6\nEND IONS\n"));
    assert!(!m.contains("FEATURE_ID=1"));
    assert!(!m.contains("unit resolution"));
    let u = mgf_unit(&c, &sp);
    assert!(u.contains("# unit resolution, tolerance 0.5 Da\n") && u.replace("# unit resolution, tolerance 0.5 Da\n", "") == m);
    let ms = sirius(&c, &sp);
    assert!(ms.contains("\n>compound feature_2\n>parentmass 300.2001\n>ionization [M-H]-\n>rt 420s\n>ms2\n100 10\n"));
    assert!(!ms.contains("feature_1"));
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_ms2_of_a_row_comes_from_its_file() {
    let out = std::env::temp_dir().join(format!("mzlab_feat_export_{}", std::process::id()));
    let ok = Command::new("python3").arg(root().join("tools/feat_riferimento.py")).arg(&out).args(["--solo", "identita"]).output().map(|o| o.status.success()).unwrap_or(false);
    if !ok {
        assert!(std::env::var("MZLAB_GOLDEN_STRICT").is_err(), "python3 or numpy missing");
        eprintln!("skipped: python3/numpy not available");
        return;
    }
    let open = || {
        let f = std::fs::File::open(out.join("identita.mzidx")).unwrap();
        let size = f.metadata().unwrap().len();
        Index::open(f, size).unwrap()
    };
    let (f, _) = analyse(&mut open(), &Params::default(), &Steps { split: false, ..Steps::default() }).unwrap();
    let i = (0..f.len()).find(|&i| f.ms2_n[i] > 0).expect("a feature with an MS2");
    let mut t = Aligned::with_capacity(1, 1);
    t.mz.push(f.mz[i]);
    t.rt.push(f64::from(f.rt[i]));
    t.area.push(f.area[i]);
    t.height.push(f.height[i]);
    t.apex_rt.push(f.rt[i]);
    t.width.push(f.right[i] - f.left[i]);
    t.state.push(CELL_DETECTED);
    t.meta.push(&RowQ { rt_lo: f.left[i], rt_hi: f.right[i], ms2_file: 0, ms2_purity: f.purity[i], ..RowQ::NONE });
    let s = samples();
    let sp = {
        let c = Ctx { table: &t, samples: &s[..1], view: None, table_sha256: "ab", polarity: 1 };
        mzlab_feat::ms2spec::spectra(&c, |_| Ok(open())).unwrap()
    };
    let m = sp[0].as_ref().expect("the spectrum of the row");
    assert!(!m.mz.is_empty() && m.mz.len() == m.it.len() && m.mz.windows(2).all(|w| w[0] <= w[1]));
    assert!((m.prec - t.mz[0]).abs() < 1.0, "the isolation window holds the ion");
    // a row with no MS2 file has no spectrum
    t.meta.ms2_file[0] = NO_FILE;
    let c = Ctx { table: &t, samples: &s[..1], view: None, table_sha256: "ab", polarity: 1 };
    assert!(mzlab_feat::ms2spec::spectra(&c, |_| Ok(open())).unwrap()[0].is_none());
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn the_sheet_is_the_same_in_the_rows_for_xlsx_and_in_the_csv_with_semicolons() {
    let (t, s) = (table(), samples());
    let c = Ctx { table: &t, samples: &s, view: None, table_sha256: "ab", polarity: 1 };
    let csv = export::csv_semicolon(&c);
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines[0], "id;mz;rt;rt_range:min;rt_range:max;charge;a,b area;a,b state;t10 area;t10 state;t20 area;t20 state");
    assert_eq!(lines[1], "1;200.12346;5.5;;;;1234567.9;DETECTED;2000000;FILLED;3000000;TARGETED");
    let rows = export::xlsx_rows(&c);
    let v = mzlab_core::json::parse(rows.as_bytes()).unwrap();
    let sheet = v.get("hojas").unwrap().at(0).unwrap().get("filas").unwrap();
    assert_eq!(sheet.arr().len(), 3);
    assert_eq!(sheet.at(1).unwrap().at(1).unwrap().f64(), Some(200.123456789));
    assert!(sheet.at(2).unwrap().at(6).unwrap().is_null());
    assert_eq!(export::xlsx_rows(&c), rows);
}
