//! A timed run repeated `giri` times and what is kept of each: wall time, CPU, page faults, peak memory, allocations; median and interquartile range.
use std::time::Instant;

use serde_json::{json, Value};

use crate::conta::{self, Alloc};

#[derive(Clone, Debug)]
pub struct Giro {
    pub s: f64,
    pub cpu: f64,
    pub rss_mb: f64,
    pub heap_mb: f64,
    pub alloc: Alloc,
    pub guasti_minori: u64,
    pub guasti_maggiori: u64,
}

pub fn mediana(v: &[f64]) -> f64 {
    let mut x: Vec<f64> = v.iter().copied().filter(|a| a.is_finite()).collect();
    if x.is_empty() {
        return f64::NAN;
    }
    mzlab_core::sortc::slice_by(&mut x, f64::total_cmp);
    let n = x.len();
    if n % 2 == 1 {
        x[n / 2]
    } else {
        (x[n / 2 - 1] + x[n / 2]) / 2.0
    }
}

/// Interquartile range (linear interpolation between order statistics).
pub fn iqr(v: &[f64]) -> f64 {
    let mut x: Vec<f64> = v.to_vec();
    mzlab_core::sortc::slice_by(&mut x, f64::total_cmp);
    let q = |p: f64| {
        let r = p * (x.len().saturating_sub(1)) as f64;
        let (a, b) = (r.floor() as usize, r.ceil() as usize);
        x[a] + (x[b] - x[a]) * (r - a as f64)
    };
    if x.is_empty() {
        f64::NAN
    } else {
        q(0.75) - q(0.25)
    }
}

/// Runs `f` `giri` times; the value of the last run is returned.
pub fn ripeti<R>(giri: usize, mut f: impl FnMut() -> R) -> (Vec<Giro>, R) {
    let mut out = Vec::new();
    let mut ultimo = None;
    for _ in 0..giri.max(1) {
        let rss_ok = conta::azzera_rss_picco();
        conta::heap_picco_mb();
        let (cpu0, (g0, m0), a0) = (conta::cpu_s(), conta::guasti(), Alloc::ora());
        let t = Instant::now();
        let r = f();
        let s = t.elapsed().as_secs_f64();
        eprintln!("  run {:.2} s", s);
        let (g1, m1) = conta::guasti();
        out.push(Giro {
            s,
            cpu: conta::cpu_s() - cpu0,
            rss_mb: if rss_ok {
                conta::rss_picco_mb()
            } else {
                f64::NAN
            },
            heap_mb: conta::heap_picco_mb(),
            alloc: Alloc::ora().da(a0),
            guasti_minori: g1 - g0,
            guasti_maggiori: m1 - m0,
        });
        ultimo = Some(r);
    }
    (out, ultimo.expect("at least one run"))
}

fn tondo(x: f64) -> Value {
    if x.is_finite() {
        json!((x * 1e6).round() / 1e6)
    } else {
        Value::Null
    }
}

/// The JSON of a series of runs: the runs, the median and the range of the time, the first run apart.
pub fn riassunto(giri: &[Giro]) -> Value {
    let s: Vec<f64> = giri.iter().map(|g| g.s).collect();
    let med = |f: fn(&Giro) -> f64| mediana(&giri.iter().map(f).collect::<Vec<_>>());
    json!({
        "secondi": s.iter().map(|x| tondo(*x)).collect::<Vec<_>>(),
        "mediana_s": tondo(mediana(&s)),
        "iqr_s": tondo(iqr(&s)),
        "primo_giro_s": tondo(s[0]),
        "mediana_senza_primo_s": tondo(mediana(&s[1.min(s.len() - 1)..])),
        "cpu_s": tondo(med(|g| g.cpu)),
        "rss_picco_mb": tondo(med(|g| g.rss_mb)),
        "heap_picco_mb": tondo(med(|g| g.heap_mb)),
        "allocazioni": med(|g| g.alloc.chiamate as f64),
        "byte_allocati": med(|g| g.alloc.byte as f64),
        "guasti_pagina_minori": med(|g| g.guasti_minori as f64),
        "guasti_pagina_maggiori": med(|g| g.guasti_maggiori as f64),
    })
}

pub fn arrotonda(x: f64) -> Value {
    tondo(x)
}
