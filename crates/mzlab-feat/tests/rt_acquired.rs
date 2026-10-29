//! Cells outside the RT a file acquired: files cut at 1.8 and 15.5 min (valve to the MS) give `CELL_NA` (not measured, NaN area) where the expected apex lies outside, never a false
//! zero; `rt_range` keeps only the rows inside. The mzML are synthetic (one Gaussian trace per ion), built here.

use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare;
use mzlab_feat::facts::file_facts;
use mzlab_feat::gapfill::{fill_file, FillParams};
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED, CELL_FILLED, CELL_NA};
use mzlab_feat::trace::Params;

const IONS: [(f64, f64); 3] = [(200.1, 1.2), (300.2, 8.0), (400.3, 17.0)];
const STEP: f64 = 0.05;

fn b64(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for c in data.chunks(3) {
        let n = (u32::from(c[0]) << 16)
            | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
            | u32::from(*c.get(2).unwrap_or(&0));
        for i in 0..4 {
            s.push(if i <= c.len() {
                T[(n >> (18 - 6 * i) & 63) as usize] as char
            } else {
                '='
            });
        }
    }
    s
}

fn array(acc: &str, v: &[f64]) -> String {
    let raw: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
    format!(
        r#"<binaryDataArray encodedLength="0"><cvParam cvRef="MS" accession="MS:1000523" name="64-bit float"/><cvParam cvRef="MS" accession="MS:1000576" name="no compression"/><cvParam cvRef="MS" accession="{acc}" name=""/><binary>{}</binary></binaryDataArray>"#,
        b64(&raw)
    )
}

/// A file acquired from `lo` to `hi` minutes: each ion is a Gaussian of 0.1 min sigma at its RT.
fn index_bytes(lo: f64, hi: f64) -> Vec<u8> {
    let n = ((hi - lo) / STEP).round() as usize + 1;
    let mut x = String::from(
        r#"<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.ms.info/ms/mzml"><run id="r"><spectrumList count=""#,
    );
    x += &format!("{n}\">");
    for i in 0..n {
        let rt = lo + i as f64 * STEP;
        let mz: Vec<f64> = IONS.iter().map(|p| p.0).collect();
        let y: Vec<f64> = IONS
            .iter()
            .map(|p| 1e6 * (-((rt - p.1) / 0.1).powi(2) / 2.0).exp() + 30_000.0)
            .collect();
        x += &format!(
            r#"<spectrum index="{i}" id="scan={}" defaultArrayLength="3"><cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="1"/><cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum"/><cvParam cvRef="MS" accession="MS:1000130" name="positive scan"/><scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{rt}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList><binaryDataArrayList count="2">{}{}</binaryDataArrayList></spectrum>"#,
            i + 1,
            array("MS:1000514", &mz),
            array("MS:1000515", &y)
        );
    }
    x += "</spectrumList></run></mzML>";
    let mzml = x.into_bytes();
    let mut src = mzml.clone();
    let p = prepare(&mut src, mzml.len() as u64, "x.mzML").unwrap();
    build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap().0
}

fn open(bytes: &[u8]) -> Index<Vec<u8>> {
    Index::open(bytes.to_vec(), bytes.len() as u64).unwrap()
}

/// 6 whole files (0-20 min), then 6 cut at 1.8-15.5 min. The ions are detected in the whole files and absent in the cut ones, except the middle one.
fn table() -> Aligned {
    let nf = 12;
    let mut t = Aligned {
        n_files: nf,
        ..Aligned::default()
    };
    for (mz, rt) in IONS {
        t.mz.push(mz);
        t.rt.push(rt);
        for k in 0..nf {
            let seen = k < 6 || (rt > 1.8 && rt < 15.5);
            t.area.push(if seen { 1e6 } else { 0.0 });
            t.height.push(0.0);
            t.apex_rt.push(if seen { rt as f32 } else { f32::NAN });
            t.width.push(if seen { 0.3 } else { f32::NAN });
            t.state.push(if seen { CELL_DETECTED } else { CELL_ABSENT });
        }
    }
    t
}

#[test]
fn cut_files_give_na_not_zero() {
    let (whole, cut) = (index_bytes(0.0, 20.0), index_bytes(1.8, 15.5));
    let (a, b) = file_facts(&open(&cut)).rt_range.unwrap();
    assert!((a - 1.8).abs() < 1e-9 && (b - 15.5).abs() < 1e-9);
    let mut t = table();
    let (p, f) = (Params::default(), FillParams::default());
    for k in 0..12 {
        let mut ix = open(if k < 6 { &whole } else { &cut });
        fill_file(&mut ix, &mut t, k, None, &p, &f).unwrap();
    }
    for k in 0..6 {
        for r in 0..3 {
            assert_eq!(t.state[r * 12 + k], CELL_DETECTED);
        }
    }
    for k in 6..12 {
        for r in [0, 2] {
            let c = r * 12 + k;
            assert_eq!(t.state[c], CELL_NA, "ion {r} file {k}");
            assert!(t.area[c].is_nan());
        }
        assert_eq!(t.state[12 + k], CELL_DETECTED);
    }
    assert!(!t.state.contains(&CELL_ABSENT) && !t.state.contains(&CELL_FILLED));
}

#[test]
fn rt_range_drops_rows_outside() {
    let mut t = table();
    t.retain_rt(1.8, 15.5);
    assert_eq!(t.mz, vec![300.2]);
    assert_eq!(t.state.len(), 12);
}
