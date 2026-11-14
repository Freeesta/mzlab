//! Kinetics of a low-resolution row: the areas of the timed samples, the first-order decay of the parent and the kinetic generation of a
//! product that grows (`crate::kinetic::generation`, shown as evidence only).

use super::super::series::FileIn;
use super::labels::Cell;
use crate::kinetic::{generation, ChainBank, FormBank, K_PARENT_MIN, MARGIN};

/// The old path of the decay of the parent, which lives in `crate::kin_decay` so that `groups` does not depend on `legacy`.
pub use crate::kin_decay::{decay, Decay, Point};

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
