//! Exposed fraction of each atomic sphere (radius = van der Waals + probe) with the smooth overlap step of width 0.3 Å used by
//! the GBSA/ALPB surface term (Ehlert et al. 2021), integrated on a Fibonacci sphere.

use super::basis::ANGSTROM_TO_BOHR;

/// Points per atom.
const POINTS: usize = 230;
/// Half width of the smooth step (Å).
const WIDTH: f64 = 0.3;

fn sphere() -> Vec<[f64; 3]> {
    let golden = std::f64::consts::PI * (3.0 - 5f64.sqrt());
    (0..POINTS)
        .map(|k| {
            let z = 1.0 - (2.0 * k as f64 + 1.0) / POINTS as f64;
            let r = (1.0 - z * z).sqrt();
            let phi = golden * k as f64;
            [r * phi.cos(), r * phi.sin(), z]
        })
        .collect()
}

/// Smooth step from 0 (u ≤ −w) to 1 (u ≥ w).
fn step(u: f64, w: f64) -> f64 {
    if u <= -w {
        0.0
    } else if u >= w {
        1.0
    } else {
        0.5 + (0.75 / w - 0.25 / w.powi(3) * u * u) * u
    }
}

/// Exposed fraction (0…1) per atom; positions and radii in bohr.
pub fn exposed(pos: &[[f64; 3]], rad: &[f64]) -> Vec<f64> {
    let w = WIDTH * ANGSTROM_TO_BOHR;
    let grid = sphere();
    (0..pos.len())
        .map(|i| {
            let near: Vec<usize> = (0..pos.len())
                .filter(|&j| j != i && super::basis::dist(&pos[i], &pos[j]) < rad[i] + rad[j] + w)
                .collect();
            let mut sum = 0.0;
            for g in &grid {
                let p = [
                    pos[i][0] + rad[i] * g[0],
                    pos[i][1] + rad[i] * g[1],
                    pos[i][2] + rad[i] * g[2],
                ];
                let mut s = 1.0;
                for &j in &near {
                    s *= step(super::basis::dist(&p, &pos[j]) - rad[j], w);
                    if s == 0.0 {
                        break;
                    }
                }
                sum += s;
            }
            sum / POINTS as f64
        })
        .collect()
}
