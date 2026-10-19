//! Shared by the ADAP tests: the reference rows of mzmine 3.9.0, the mzML reader of the scans and the comparison of one chromatogram with its row.
#![allow(dead_code)]

use std::path::Path;

use mzlab_core::mzml;
use mzlab_feat::untargeted::adap::{AdapParams, Chrom, Ms1Scan};

pub fn scans_of(path: &Path, p: &AdapParams) -> Vec<Ms1Scan> {
    let run = mzml::read(std::fs::File::open(path).unwrap()).unwrap();
    run.spectra
        .iter()
        .filter(|s| s.level == 1)
        .enumerate()
        .map(|(k, s)| Ms1Scan::above_noise(k, s.rt, &s.mz, &s.intensity, p.noise))
        .collect()
}

pub struct Row {
    pub file: String,
    pub mz: f64,
    pub rt: f64,
    pub start: f64,
    pub end: f64,
    pub height: f64,
}

pub fn rows(path: &Path) -> Vec<Row> {
    let text = std::fs::read_to_string(path).unwrap();
    text.lines()
        .skip(1)
        .map(|l| {
            let c: Vec<&str> = l.split(',').collect();
            let n = |i: usize| c[i].parse::<f64>().unwrap();
            Row {
                file: c[0].to_string(),
                mz: n(2),
                rt: n(3),
                start: n(4),
                end: n(5),
                height: n(6),
            }
        })
        .collect()
}

/// (apex RT, first and last RT with the zeros, height) of a chromatogram.
pub fn shape(c: &Chrom, scans: &[Ms1Scan]) -> (f64, f64, f64, f64) {
    let rt = |k: usize| scans[c.scan[k] as usize].rt;
    (
        rt(c.apex()),
        rt(0),
        rt(c.scan.len() - 1),
        c.int[c.apex()] as f64,
    )
}

pub fn same(r: &Row, c: &Chrom, scans: &[Ms1Scan], mz_tol: f64, h_tol: f64) -> Result<(), String> {
    let (rt, a, b, h) = shape(c, scans);
    let mut why = Vec::new();
    if (c.mz - r.mz).abs() > mz_tol {
        why.push(format!("m/z {} vs {}", c.mz, r.mz));
    }
    if (rt - r.rt).abs() > 1e-3 || (a - r.start).abs() > 1e-3 || (b - r.end).abs() > 1e-3 {
        why.push(format!(
            "RT apex/start/end {rt:.2}/{a:.2}/{b:.2} vs {}/{}/{}",
            r.rt, r.start, r.end
        ));
    }
    if (h - r.height).abs() > h_tol * r.height {
        why.push(format!("height {h} vs {}", r.height));
    }
    if why.is_empty() {
        Ok(())
    } else {
        Err(why.join("; "))
    }
}

/// As [`same`] but without the RT range: apex, m/z and height only (mzmine 4.10 reports wider ranges than 3.9.0, see `mzmine410/LEGGIMI.md`).
pub fn same_core(
    r: &Row,
    c: &Chrom,
    scans: &[Ms1Scan],
    mz_tol: f64,
    h_tol: f64,
) -> Result<(), String> {
    let (rt, _, _, h) = shape(c, scans);
    let mut why = Vec::new();
    if (c.mz - r.mz).abs() > mz_tol {
        why.push(format!("m/z {} vs {}", c.mz, r.mz));
    }
    if (rt - r.rt).abs() > 1e-3 {
        why.push(format!("RT apex {rt:.2} vs {}", r.rt));
    }
    if (h - r.height).abs() > h_tol * r.height {
        why.push(format!("height {h} vs {}", r.height));
    }
    if why.is_empty() {
        Ok(())
    } else {
        Err(why.join("; "))
    }
}

/// How much wider (min) the range of the reference is than ours: `(ours.start - theirs.start, theirs.end - ours.end)`.
pub fn range_widening(r: &Row, c: &Chrom, scans: &[Ms1Scan]) -> (f64, f64) {
    let (_, a, b, _) = shape(c, scans);
    (a - r.start, r.end - b)
}
