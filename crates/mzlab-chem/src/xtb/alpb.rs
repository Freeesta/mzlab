//! ALPB implicit solvation (Ehlert et al. 2021, doi:10.1021/acs.jctc.1c00471), coupled to the SCC as an atomic potential.
//! Born radii of Onufriev, Bashford and Case (doi:10.1002/prot.20033), P16 kernel (Lange and Herbert 2012) and the ALPB shape
//! term follow dxtb 0.4.0 (`components/interactions/solvation/born.py`, `alpb.py`; Apache-2.0, github.com/grimme-lab/dxtb);
//! D3 van der Waals radii from tad-mctc 0.9.2 (`data/radii.py`, Apache-2.0). Water parameters of GFN2-xTB (Born scale, probe
//! radius, descreening and hydrogen-bond strength per element): `water::*`. The surface-tension (CDS) term is left out because it
//! does not depend on the charges, so it changes neither the charges nor the Fukui functions; the hydrogen-bond term, which does,
//! is the diagonal −c²·(exposed fraction of the atom)·q² with the exposed fraction from `sasa`.

use super::basis::{dist, ANGSTROM_TO_BOHR};
use super::{sasa, water};
use nalgebra::DMatrix;

/// Klamt's ALPB constant α.
const ALPB_ALPHA: f64 = 0.571412;
/// Real-space cutoff of the Born integral (bohr).
const BORN_CUTOFF: f64 = 66.0;
/// Onufriev-Bashford-Case parameters (α, β, γ).
const OBC: [f64; 3] = [1.0, 0.8, 4.85];

/// Parameters of one solvent.
pub struct Solvent {
    pub name: &'static str,
    pub epsilon: f64,
    pub born_scale: f64,
    /// Offset of the Born integration (bohr).
    pub born_offset: f64,
    /// Probe radius of the solvent-accessible surface (Å).
    pub probe: f64,
    /// Dielectric descreening per atom, as a function of the atomic number.
    pub descreening: fn(u8) -> f64,
    /// Hydrogen-bond strength c (√(kcal/mol)) per atomic number; 0 without the term.
    pub hbond: fn(u8) -> f64,
}

/// Water for GFN2-xTB (Ehlert et al. 2021, supporting information `param_alpb2_water.txt`).
pub const WATER: Solvent = Solvent {
    name: "water",
    epsilon: water::EPSILON,
    born_scale: water::BORN_SCALE,
    born_offset: 0.0,
    probe: water::PROBE,
    descreening: water::descreening,
    hbond: water::hbond,
};

/// D3 van der Waals radius (Å) for the elements of `mol3d`.
pub(super) fn vdw_d3(z: u8) -> f64 {
    match z {
        1 => 1.09155,
        6 => 1.45515,
        7 => 1.31125,
        8 => 1.24085,
        9 => 1.14980,
        15 => 1.75085,
        16 => 1.65535,
        17 => 1.55230,
        35 => 1.68310,
        53 => 1.86900,
        _ => 1.6,
    }
}

/// Descreening integral of atom j from atom i (radii and distance in bohr).
fn psi_pair(d: f64, rho: f64, rj: f64) -> f64 {
    let (pl, mi) = (d + rho, d - rho);
    if d < rj + rho && pl > rj {
        let (rj1, pl1) = (1.0 / rj, 1.0 / pl);
        rj1 - pl1 + 0.5 / d * (0.5 * mi * (pl1 - rj1 * pl * rj1) - (pl * rj1).ln())
    } else {
        let qu = mi / pl;
        let ln = if qu > 0.0 { 0.5 * qu.ln() / d } else { 0.0 };
        rho / (pl * mi) + ln
    }
}

/// Born radii (bohr) for positions in bohr.
pub fn born_radii(el: &[u8], pos: &[[f64; 3]], s: &Solvent) -> Vec<f64> {
    let n = el.len();
    let rvdw: Vec<f64> = el.iter().map(|&z| vdw_d3(z) * ANGSTROM_TO_BOHR).collect();
    (0..n)
        .map(|j| {
            let mut psi = 0.0;
            for i in (0..n).filter(|&i| i != j) {
                let d = dist(&pos[i], &pos[j]);
                if d <= BORN_CUTOFF {
                    psi += psi_pair(d, rvdw[i] * (s.descreening)(el[i]), rvdw[j]);
                }
            }
            let sr = rvdw[j] - s.born_offset;
            let p2 = 0.5 * psi * sr;
            let t = p2 * (OBC[0] + p2 * (OBC[2] * p2 - OBC[1]));
            s.born_scale / (1.0 / sr - t.tanh() / rvdw[j])
        })
        .collect()
}

/// ALPB shape descriptor from the inertia tensor of solid spheres (bohr).
fn adet(el: &[u8], pos: &[[f64; 3]]) -> f64 {
    let rad: Vec<f64> = el.iter().map(|&z| vdw_d3(z) * ANGSTROM_TO_BOHR).collect();
    let vol: Vec<f64> = rad.iter().map(|r| r * r * r).collect();
    let vt: f64 = vol.iter().sum();
    let mut c = [0.0; 3];
    for (p, v) in pos.iter().zip(&vol) {
        for k in 0..3 {
            c[k] += p[k] * v / vt;
        }
    }
    let mut t = [[0.0; 3]; 3];
    for i in 0..el.len() {
        let d = [pos[i][0] - c[0], pos[i][1] - c[1], pos[i][2] - c[2]];
        let diag = d[0] * d[0] + d[1] * d[1] + d[2] * d[2] + 0.4 * rad[i] * rad[i];
        for a in 0..3 {
            for b in 0..3 {
                t[a][b] += vol[i] * (if a == b { diag } else { 0.0 } - d[a] * d[b]);
            }
        }
    }
    let det = t[0][0] * (t[1][1] * t[2][2] - t[1][2] * t[2][1])
        - t[0][1] * (t[1][0] * t[2][2] - t[1][2] * t[2][0])
        + t[0][2] * (t[1][0] * t[2][1] - t[1][1] * t[2][0]);
    (5.0 * det.cbrt() / (2.0 * vt)).sqrt()
}

/// Atom-pair ALPB matrix (Hartree, positions in bohr): energy ½ qᵀAq, potential Aq.
pub fn matrix(el: &[u8], pos: &[[f64; 3]], s: &Solvent) -> DMatrix<f64> {
    let n = el.len();
    let born = born_radii(el, pos, s);
    let alpbet = ALPB_ALPHA / s.epsilon;
    let keps = (1.0 / s.epsilon - 1.0) / (1.0 + alpbet);
    let shape = alpbet * adet(el, pos);
    let hbond = hbond_weights(el, pos, s);
    DMatrix::from_fn(n, n, |i, j| {
        let r = dist(&pos[i], &pos[j]);
        let ab = (born[i] * born[j]).sqrt();
        let arg = (ab / (ab + r * 1.028 / 16.0)).powi(16);
        let hb = if i == j { 2.0 * hbond[i] } else { 0.0 };
        keps * (1.0 / (r + ab * arg) + shape) + hb
    })
}

/// Hydrogen-bond weight per atom (Hartree): −c²·(exposed fraction), energy Σ wᵢqᵢ².
fn hbond_weights(el: &[u8], pos: &[[f64; 3]], s: &Solvent) -> Vec<f64> {
    let rad: Vec<f64> = el
        .iter()
        .map(|&z| (vdw_d3(z) + s.probe) * ANGSTROM_TO_BOHR)
        .collect();
    let exposed = sasa::exposed(pos, &rad);
    el.iter()
        .zip(exposed)
        .map(|(&z, f)| -(s.hbond)(z).powi(2) / water::KCAL_PER_HARTREE * f)
        .collect()
}
