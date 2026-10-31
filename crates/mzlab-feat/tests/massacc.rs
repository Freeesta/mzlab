//! `qc.mass_accuracy` on synthetic indexes (8 ions of the lab list held steady over 20 scans): a constant error is recovered as the offset and flagged, a linear error is recovered by
//! `linear_mz`, one overlapped ion is rejected by the robust fits, three references are "not assessable", and a corrected table has its m/z back within 0.5 ppm with the same rows.

use mzlab_core::idxread::Index;
use mzlab_core::json::parse;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare;
use mzlab_feat::massacc::{self, FileSpec, Model, Params, RefIon, Robust, Role};
use mzlab_feat::store::{read_table, write_table, Origin, Slice};
use mzlab_feat::table::{Aligned, CELL_DETECTED};

const MZ: [f64; 8] = [
    149.0233, 279.1591, 391.2843, 445.1200, 579.1810, 683.4642, 797.5278, 881.2105,
];
const SCANS: usize = 20;

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

/// An index whose scans all hold `peaks` (m/z, intensity).
fn index_of(peaks: &[(f64, f64)]) -> Index<Vec<u8>> {
    let mut x = String::from(
        r#"<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.ms.info/ms/mzml"><run id="r"><spectrumList count=""#,
    );
    x += &format!("{SCANS}\">");
    let mut sorted = peaks.to_vec();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mz: Vec<f64> = sorted.iter().map(|p| p.0).collect();
    let y: Vec<f64> = sorted.iter().map(|p| p.1).collect();
    for i in 0..SCANS {
        let rt = 1.0 + i as f64 * 0.1;
        x += &format!(
            r#"<spectrum index="{i}" id="scan={}" defaultArrayLength="{}"><cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="1"/><cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum"/><cvParam cvRef="MS" accession="MS:1000130" name="positive scan"/><cvParam cvRef="MS" accession="MS:1000512" name="filter string" value="FTMS + p ESI Full ms [50.00-1000.00]"/><scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{rt}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList><binaryDataArrayList count="2">{}{}</binaryDataArrayList></spectrum>"#,
            i + 1,
            mz.len(),
            array("MS:1000514", &mz),
            array("MS:1000515", &y)
        );
    }
    x += "</spectrumList></run></mzML>";
    let mzml = x.into_bytes();
    let mut src = mzml.clone();
    let p = prepare(&mut src, mzml.len() as u64, "x.mzML").unwrap();
    let bytes = build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap().0;
    let n = bytes.len() as u64;
    Index::open(bytes, n).unwrap()
}

fn refs(n: usize) -> Vec<RefIon> {
    MZ.iter()
        .take(n)
        .enumerate()
        .map(|(i, &mz)| RefIon {
            name: format!("ion{i}"),
            mz,
            pol: 1,
            rt: None,
            window: None,
            role: Role::Reference,
            background: false,
        })
        .collect()
}

/// The check of one file whose ion `i` is seen at `mz * (1 + err(mz) ppm)`.
fn check(
    err: impl Fn(usize, f64) -> f64,
    n_refs: usize,
    p: &Params,
    table: Option<&[u8]>,
) -> (mzlab_core::json::Value, Option<Vec<u8>>) {
    let peaks: Vec<(f64, f64)> = MZ
        .iter()
        .enumerate()
        .map(|(i, &m)| (m * (1.0 + err(i, m) * 1e-6), 1e6))
        .collect();
    let mut ix = Some(index_of(&peaks));
    let mut slice = table.map(Slice);
    let out = massacc::run(
        &mut |_| Ok(ix.take().unwrap()),
        &[FileSpec {
            name: "a.mzML".into(),
            istd: false,
        }],
        refs(n_refs),
        slice.as_mut().zip(table).map(|(s, t)| (s, t.len() as u64)),
        p,
    )
    .unwrap();
    (parse(out.json.as_bytes()).unwrap(), out.table)
}

fn num(v: &mzlab_core::json::Value, path: &[&str]) -> f64 {
    path.iter().fold(v, |v, k| v.get(k).unwrap()).f64().unwrap()
}

fn warnings(f: &mzlab_core::json::Value) -> Vec<String> {
    f.get("warnings")
        .unwrap()
        .arr()
        .iter()
        .map(|w| w.str().unwrap().to_string())
        .collect()
}

#[test]
fn a_constant_error_is_the_offset_and_is_flagged() {
    let (doc, _) = check(|_, _| 4.0, 8, &Params::default(), None);
    let f = &doc.get("files").unwrap().arr()[0];
    assert!((num(f, &["offset_ppm"]) - 4.0).abs() < 0.2);
    assert!(warnings(f).contains(&"warn.qc.mass.offset".to_string()));
    assert_eq!(f.get("assessable").unwrap().bool(), Some(true));
}

#[test]
fn each_point_says_in_how_many_scans_the_ion_was_seen() {
    let (doc, _) = check(|_, _| 3.0, 8, &Params::default(), None);
    let pts = doc.get("files").unwrap().arr()[0]
        .get("points")
        .unwrap()
        .arr()
        .to_vec();
    assert_eq!(pts.len(), 8);
    assert!(pts.iter().all(|p| num(p, &["scans"]) == SCANS as f64));
}

#[test]
fn a_linear_error_is_recovered() {
    let p = Params {
        model: Model::LinearMz,
        ..Params::default()
    };
    let (doc, _) = check(|_, m| 1.0 + 0.01 * (m - 400.0), 8, &p, None);
    let c = doc.get("files").unwrap().arr()[0].get("coef").unwrap();
    // intercept at 400 and 1 ppm per 100 m/z
    assert!((num(c, &["offset_ppm"]) - 1.0).abs() < 0.2, "{c:?}");
    assert!(
        (num(c, &["slope_ppm_per_100_mz"]) - 1.0).abs() < 0.1,
        "{c:?}"
    );
}

#[test]
fn an_overlapped_reference_is_rejected_by_the_robust_fits() {
    let err = |i: usize, _: f64| if i == 4 { 15.0 } else { 4.0 };
    for robust in [Robust::Huber, Robust::Ransac] {
        let (doc, _) = check(
            err,
            8,
            &Params {
                robust,
                ..Params::default()
            },
            None,
        );
        let f = &doc.get("files").unwrap().arr()[0];
        assert!(
            (num(f, &["offset_ppm"]) - 4.0).abs() < 0.3,
            "{robust:?}: {}",
            num(f, &["offset_ppm"])
        );
        let outliers = f
            .get("points")
            .unwrap()
            .arr()
            .iter()
            .filter(|p| p.get("outlier").unwrap().bool() == Some(true))
            .count();
        assert_eq!(outliers, 1, "{robust:?}");
    }
    let (doc, _) = check(
        err,
        8,
        &Params {
            robust: Robust::Plain,
            ..Params::default()
        },
        None,
    );
    assert!(num(&doc.get("files").unwrap().arr()[0], &["offset_ppm"]) > 5.0);
}

#[test]
fn three_references_are_not_assessable() {
    let (doc, _) = check(|_, _| 4.0, 3, &Params::default(), None);
    let f = &doc.get("files").unwrap().arr()[0];
    assert_eq!(f.get("assessable").unwrap().bool(), Some(false));
    assert!(warnings(f).contains(&"warn.qc.mass.few_refs".to_string()));
}

fn table_of(true_mz: &[f64], seen_ppm: f64) -> Vec<u8> {
    let mut t = Aligned::with_capacity(1, true_mz.len());
    for (r, &m) in true_mz.iter().enumerate() {
        t.mz.push(m * (1.0 + seen_ppm * 1e-6));
        t.rt.push(2.0 + r as f64);
        t.area.push(1e6);
        t.height.push(1e5);
        t.apex_rt.push(2.0 + r as f32);
        t.width.push(0.2);
        t.state.push(CELL_DETECTED);
    }
    let o = Origin {
        inputs: vec![("a.mzML".into(), String::new())],
        ..Origin::default()
    };
    write_table(&t, &Default::default(), &o)
}

#[test]
fn the_offset_model_corrects_the_table_and_keeps_the_rows() {
    let truth = [210.1234, 405.5678, 700.9012];
    let bytes = table_of(&truth, 4.0);
    let p = Params {
        model: Model::Offset,
        ..Params::default()
    };
    let (doc, out) = check(|_, _| 4.0, 8, &p, Some(&bytes));
    let out = out.expect("a corrected table");
    assert_eq!(out.len(), bytes.len());
    let (before, _) = read_table(&bytes).unwrap();
    let (after, _) = read_table(&out).unwrap();
    assert_eq!(
        (after.len(), after.rt.clone(), after.area.clone()),
        (before.len(), before.rt.clone(), before.area.clone())
    );
    for (m, t) in after.mz.iter().zip(truth) {
        assert!(((m - t) / t * 1e6).abs() < 0.5, "{m} against {t}");
    }
    assert_eq!(
        doc.get("correction")
            .unwrap()
            .get("applied")
            .unwrap()
            .bool(),
        Some(true)
    );
    // no model, no table
    let (_, none) = check(|_, _| 4.0, 8, &Params::default(), Some(&bytes));
    assert!(none.is_none());
}
