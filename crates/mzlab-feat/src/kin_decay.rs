//! The first-order decay of a parent over the timed samples: the points (time, area, height) and the fit of ln(area) against time.
//!
//! The kinetics of a low-resolution row (`legacy::lr::kin`) and the presence of a row in its conditions (`groups::presence`) read it.

/// One point of `kinetics`: area and height of a timed sample (0 where the pipeline found no peak).
#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub time: f64,
    pub area: f64,
    pub height: f64,
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
        .map(|p| (p.time, libm::log(p.area)))
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
