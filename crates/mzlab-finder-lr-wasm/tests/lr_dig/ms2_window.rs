//! The MS2 scans of a row of the low-resolution Dig (`lr::ms2::pool`): the window is centred on the reference RT of the row and a trace
//! without signal gives no scans. Included by `lr_dig.rs` (`#[path]`), which has the generators of the series.

use super::*;
use mzlab_feat::legacy::lr::labels::Cell;
use mzlab_feat::legacy::lr::ms2::pool;

/// A trace file: product-ion scans of `precursor` every 0.05 min over `0..20`, a flat TIC of ~100 plus a peak of 20 000 at `peak`.
fn trace_index(precursor: f64, peak: Option<f64>) -> Vec<u8> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "lr_trace".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let n = 400;
    w.open_spectra(n).unwrap();
    for i in 0..n {
        let rt = 0.05 * i as f64;
        let top = peak.map_or(0.0, |p| 20_000.0 * (-((rt - p) / 0.1).powi(2)).exp());
        w.spectrum(&Spectrum {
            index: i,
            id: format!("sample=1 period=1 cycle={} experiment=2", i + 1),
            level: 2,
            rt,
            polarity: 1,
            profile: false,
            filter: "+ EPI".into(),
            precursor: Some(precursor),
            mz: vec![precursor - 17.0, precursor - 44.0],
            intensity: vec![(100.0 + top) as f32, 50.0],
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    let mzml = w.finish().unwrap();
    let size = mzml.len() as u64;
    let mut src = mzml.clone();
    let p = prepare(&mut src, size, "lr_trace.mzML").unwrap();
    build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap().0
}

fn cell(apex: f64, half: f64) -> Cell {
    Cell {
        area: 1.0,
        height: 1.0,
        apex_rt: Some(apex),
        detected: true,
        lo: apex - half,
        hi: apex + half,
    }
}

/// The scans the pool of the parent takes from a trace file with the given peak, its cells being `cells`.
fn scans(peak: Option<f64>, cells: impl FnOnce(f64) -> Vec<Cell>) -> usize {
    let (files, idxs) = series();
    let with = files.replace(
        ']',
        r#",{"name": "lr_EPI.mzML", "time": null, "type": "sample", "kind": "ms2"}]"#,
    );
    let mut d = dig(&with, &idxs);
    let rt = d.entries[0].ref_rt.expect("RT of the parent");
    d.entries[0].cells = cells(rt);
    let epi = trace_index(d.entries[0].mz_x, peak.map(|p| p + rt));
    let n = idxs.len();
    let mut op = |k: usize| Ok(if k < n { open(&idxs[k]) } else { open(&epi) });
    pool(&d, &d.entries[0], 6, &mut op).unwrap().scans
}

fn narrow(rt: f64) -> Vec<Cell> {
    (0..6).map(|i| cell(rt + 0.01 * i as f64, 0.1)).collect()
}

#[test]
fn a_wide_cell_in_another_file_does_not_open_the_window() {
    let wide = |rt: f64| {
        let mut c = narrow(rt);
        c.push(cell(rt + 3.0, 5.0));
        c
    };
    // a flat trace: nothing to take, however wide the cell
    assert_eq!(scans(None, wide), 0);
    // a peak at the RT of the row: its scans only, none from the rest of the run
    let n = scans(Some(0.0), wide);
    assert!((10..=20).contains(&n), "{n}");
    assert_eq!(n, scans(Some(0.0), narrow));
}

#[test]
fn a_cell_one_minute_away_does_not_move_the_window() {
    let off = |rt: f64| {
        let mut c = narrow(rt);
        c.push(cell(rt + 1.0, 0.1));
        c
    };
    assert_eq!(scans(Some(0.0), off), scans(Some(0.0), narrow));
    assert_eq!(scans(Some(1.0), off), 0);
    assert_eq!(scans(Some(0.0), |_| vec![cell(1.0e3, 0.1)]), 0);
}
