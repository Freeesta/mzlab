//! The synthetic series of the kinetic LR chain (`workflow_kinetic_tests.rs`): eight full-scan mzXML files of a triple quad, unit resolution,
//! with a parent that decays at a known first-order rate per condition and two products, one formed only under one condition.

use std::path::{Path, PathBuf};

use super::s;
use crate::convert_fixtures::mzxml::{mzxml, Opz, Sc, FLAT};

/// The instrument's m/z offset of the series (observed - true), which `offset=auto` must find on the parent.
pub const OFF: f64 = 0.3;

/// The files of the plan template: (name, condition, minutes); the blank has no analyte, the t0 is shared by both conditions.
pub const FILES: [(&str, &str, f64); 8] = [
    ("Blank_01", "", -1.0),
    ("t0", "", 0.0),
    ("Fe_t5", "Fe", 5.0),
    ("Fe_t15", "Fe", 15.0),
    ("Fe_t30", "Fe", 30.0),
    ("Cu_t5", "Cu", 5.0),
    ("Cu_t15", "Cu", 15.0),
    ("Cu_t30", "Cu", 30.0),
];

/// A small deterministic generator (xorshift64) with uniform and normal draws.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }

    fn normal(&mut self) -> f64 {
        let (a, b) = (self.next().max(1e-12), self.next());
        (-2.0 * a.ln()).sqrt() * (std::f64::consts::TAU * b).cos()
    }
}

/// A slightly tailed Gaussian of height 1 (the shape of `tools/dati_sintetici.py`).
fn g(rt: f64, c: f64, sd: f64) -> f64 {
    let x = rt - c;
    (-0.5 * (x / sd).powi(2)).exp()
        * if x > 0.0 {
            (-x / 0.04 * 0.15).exp()
        } else {
            1.0
        }
}

/// The ions of one file, (observed m/z, RT, sigma, height): the parent decays faster under Fe; P287 forms in both conditions, P201 only under Fe.
fn ions(cond: &str, t: f64) -> Vec<(f64, f64, f64, f64)> {
    let mut v = vec![(391.3, 18.3, 0.08, 6e5), (279.2, 2.1, 0.1, 3e5)];
    if t < 0.0 {
        return v;
    }
    let k = if cond == "Cu" { 0.015 } else { 0.035 };
    let p = 2.0e7 * (-k * t).exp();
    for (mz, rel) in [
        (343.09, 1.0),
        (344.09, 0.18),
        (345.09, 0.063),
        (211.0, 0.8),
        (167.0, 0.95),
    ] {
        v.push((mz + OFF, 14.3, 0.06, rel * p));
    }
    v.push((
        287.1 + OFF,
        14.7,
        0.06,
        2.5e6 * (1.0 - (-2.0 * k * t).exp()),
    ));
    if cond == "Fe" {
        v.push((201.1 + OFF, 7.07, 0.06, 1.2e6 * (1.0 - (-0.04 * t).exp())));
    }
    v
}

/// One full-scan mzXML of 900 unit-resolution scans: 30 noise peaks per scan plus the ions, m/z jittered by 0.03.
fn file(cond: &str, t: f64, seed: u64) -> Vec<u8> {
    let mut r = Rng(0x9E37_79B9_7F4A_7C15 ^ seed);
    let comps = ions(cond, t);
    let scans: Vec<Sc> = (0..900u32)
        .map(|i| {
            let rt = 0.5 + i as f64 * 19.5 / 899.0;
            let mut pk: Vec<(f64, f64)> = (0..30)
                .map(|_| {
                    (
                        120.0 + 360.0 * r.next(),
                        (4e3f64.ln() + 0.9 * r.normal()).exp(),
                    )
                })
                .collect();
            for &(mz, c, sd, a) in &comps {
                let y = a * g(rt, c, sd) * (1.0 + 0.03 * r.normal());
                if y > 200.0 {
                    pk.push((mz + 0.03 * r.normal(), y));
                }
            }
            pk.sort_by(|a, b| a.0.total_cmp(&b.0));
            Sc {
                num: i + 1,
                level: 1,
                rt_s: rt * 60.0,
                pol: "+",
                mz: pk.iter().map(|p| p.0).collect(),
                it: pk.iter().map(|p| p.1).collect(),
                prec: None,
                kind: "Full",
                children: Vec::new(),
            }
        })
        .collect();
    let o = Opz {
        zlib: false,
        ..FLAT
    };
    // the shared header describes an Orbitrap: a triple quad here, so the files read as unit resolution
    let x = String::from_utf8(mzxml(&scans, o)).unwrap();
    let mut x = x
        .replace("Thermo Scientific", "SCIEX")
        .replace("Q Exactive", "API 3200");
    x = x
        .replace("value=\"Orbitrap\"", "value=\"Quadrupole\"")
        .replace("inductive detector", "electron multiplier");
    x.into_bytes()
}

/// The vendor folder `raw/` of the series and the paths of its files.
pub fn series(d: &Path) -> (PathBuf, Vec<String>) {
    let raw = d.join("raw");
    std::fs::create_dir_all(&raw).unwrap();
    let mut names = Vec::new();
    for (k, (n, cond, t)) in FILES.iter().enumerate() {
        let f = raw.join(format!("{n}.mzXML"));
        std::fs::write(&f, file(cond, *t, k as u64 + 1)).unwrap();
        names.push(s(&f).to_string());
    }
    (raw, names)
}
