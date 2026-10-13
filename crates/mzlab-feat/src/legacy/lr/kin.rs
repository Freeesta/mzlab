//! Kinetics of a low-resolution row: the areas of the timed samples, the first-order decay of the parent and the kinetic generation of a
//! product that grows (`crate::kinetic::generation`, shown as evidence only).

use super::super::series::FileIn;
use super::labels::Cell;
use crate::kinetic::{generation, ChainBank, FormBank, K_PARENT_MIN, MARGIN};

/// One point of `kinetics`: area and height of a timed sample (0 where the pipeline found no peak).
#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub time: f64,
    pub area: f64,
    pub height: f64,
}

pub fn kinetics(files: &[FileIn], cells: &[Cell]) -> Vec<Point> {
    let mut ks: Vec<usize> = (0..files.len())
        .filter(|&k| files[k].kind == "sample" && files[k].time.is_some())
        .collect();
    mzlab_core::sortc::by(&mut ks, |&a, &b| files[a].t.total_cmp(&files[b].t));
    ks.iter()
        .map(|&k| {
            let c = &cells[k];
            Point {
                time: files[k].t,
                area: if c.detected { c.area } else { 0.0 },
                height: if c.detected { c.height } else { 0.0 },
            }
        })
        .collect()
}

/// `parent_decay`: k (1/min), t1/2 (min), R² of ln(area) against time on the detected points.
#[derive(Debug, Clone, Copy)]
pub struct Decay {
    pub k: f64,
    pub half_life: Option<f64>,
    pub r2: f64,
    pub points: usize,
}

pub fn decay(kin: &[Point]) -> Option<Decay> {
    let pts: Vec<(f64, f64)> = kin
        .iter()
        .filter(|p| p.area > 0.0)
        .map(|p| (p.time, p.area.ln()))
        .collect();
    if pts.len() < 3 {
        return None;
    }
    let n = pts.len() as f64;
    let (mt, my) = (
        pts.iter().map(|p| p.0).sum::<f64>() / n,
        pts.iter().map(|p| p.1).sum::<f64>() / n,
    );
    let sxx: f64 = pts.iter().map(|p| (p.0 - mt).powi(2)).sum();
    let sxy: f64 = pts.iter().map(|p| (p.0 - mt) * (p.1 - my)).sum();
    if sxx <= 0.0 {
        return None;
    }
    let slope = sxy / sxx;
    let icpt = my - slope * mt;
    let ss_res: f64 = pts.iter().map(|p| (p.1 - slope * p.0 - icpt).powi(2)).sum();
    let ss_tot: f64 = pts.iter().map(|p| (p.1 - my).powi(2)).sum();
    let k = -slope;
    Some(Decay {
        k,
        half_life: (k > 0.0).then(|| std::f64::consts::LN_2 / k),
        r2: if ss_tot > 0.0 {
            1.0 - ss_res / ss_tot
        } else {
            0.0
        },
        points: pts.len(),
    })
}

/// The models of the generation, built once on the times of the series (t >= 0).
pub struct Gens {
    times: Vec<f64>,
    forms: Option<FormBank>,
    chain: Option<ChainBank>,
}

impl Gens {
    pub fn new(kin: &[Point], k_parent: Option<f64>) -> Gens {
        let times: Vec<f64> = kin.iter().map(|p| p.time).filter(|t| *t >= 0.0).collect();
        let ok = times.len() >= 4;
        let kp = k_parent.filter(|k| *k > 0.0).unwrap_or(K_PARENT_MIN);
        Gens {
            forms: ok.then(|| FormBank::new(&times)),
            chain: ok.then(|| ChainBank::new(&times, kp)),
            times,
        }
    }
}

/// First or second generation of a product that grows: (verdict, text of the criterion).
pub fn gen_of(g: &Gens, kin: &[Point]) -> Option<(&'static str, String)> {
    let y: Vec<f64> = kin
        .iter()
        .filter(|p| p.time >= 0.0)
        .map(|p| p.area)
        .collect();
    if y.len() != g.times.len() || y.iter().copied().fold(0.0, f64::max) <= 0.0 {
        return None;
    }
    let r = generation(g.forms.as_ref()?, g.chain.as_ref()?, &y, MARGIN)?;
    let what = match r.verdict.name() {
        "first" => "1st generation (formed directly from the parent)",
        "second" => "2nd generation (a product of a product)",
        _ => "generation undecidable with these points",
    };
    Some((
        r.verdict.name(),
        format!(
            "{what}; AICc direct {:.1} vs through an intermediate {:.1}",
            r.aicc_first, r.aicc_second
        ),
    ))
}
