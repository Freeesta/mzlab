//! GFN2-xTB basis, integrals and zeroth-order Hamiltonian for one geometry.
//! Restructured from sci-form 0.15.2 `src/xtb/gfn2.rs` (`solve_gfn2`, `compute_coordination_numbers`, `covalent_radius_d3_bohr`);
//! MIT, see LICENSE-sci-form. Parameters and STO-nG multipole integrals are used from the `sci_form` crate as they are.

use nalgebra::DMatrix;
use sci_form::xtb::gfn2_params::{self, Gfn2ElementParams, EV_TO_HARTREE, GAM3_SHELL};
use sci_form::xtb::sto_overlap::sto_multipole_with_ng;

pub const ANGSTROM_TO_BOHR: f64 = 1.889_725_988_6;
pub const EV_PER_HARTREE: f64 = 27.211_385_05;

/// One shell: atom, Hubbard parameter (Hartree), reference occupation, third-order parameter (Hartree).
pub struct Shell {
    pub atom: usize,
    pub eta: f64,
    pub ref_occ: f64,
    pub gam3: f64,
}

/// Basis functions (atom, shell, l, m), shells and the shell of each function.
pub struct Basis {
    pub ao: Vec<(usize, usize, u8, u8)>,
    pub shells: Vec<Shell>,
    pub ao_shell: Vec<usize>,
}

/// Overlap S and the dipole / traceless quadrupole integrals (`[k][(i, j)]` about the atom of `i`).
pub struct Ints {
    pub s: DMatrix<f64>,
    pub dp: [DMatrix<f64>; 3],
    pub qp: [DMatrix<f64>; 6],
}

pub fn par(z: u8) -> &'static Gfn2ElementParams {
    gfn2_params::get_gfn2_params(z).expect("element checked by the caller")
}

pub fn dist(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

pub fn basis(el: &[u8]) -> Basis {
    let mut b = Basis {
        ao: Vec::new(),
        shells: Vec::new(),
        ao_shell: Vec::new(),
    };
    for (i, &z) in el.iter().enumerate() {
        let p = par(z);
        for sh in 0..p.nshell {
            let l = p.ang_shell[sh];
            let kind = (p.kind as usize).saturating_sub(1);
            let idx = b.shells.len();
            b.shells.push(Shell {
                atom: i,
                eta: p.hubbard * p.shell_hubbard[sh],
                ref_occ: p.ref_occ[sh],
                gam3: p.gam3 * GAM3_SHELL[(l as usize).min(3)][kind],
            });
            for m in 0..(2 * l as usize + 1) {
                b.ao.push((i, sh, l, m as u8));
                b.ao_shell.push(idx);
            }
        }
    }
    b
}

fn pair(
    el: &[u8],
    pos: &[[f64; 3]],
    a: (usize, usize, u8, u8),
    b: (usize, usize, u8, u8),
) -> (f64, [f64; 3], [f64; 6]) {
    let (pa, pb) = (par(el[a.0]), par(el[b.0]));
    sto_multipole_with_ng(
        pa.pqn[a.1],
        a.2,
        a.3,
        pa.slater[a.1],
        &pos[a.0],
        pa.ngauss[a.1],
        pb.pqn[b.1],
        b.2,
        b.3,
        pb.slater[b.1],
        &pos[b.0],
        pb.ngauss[b.1],
    )
}

/// Overlap and multipole integrals (positions in bohr).
#[allow(clippy::needless_range_loop)]
pub fn ints(el: &[u8], pos: &[[f64; 3]], b: &Basis) -> Ints {
    let n = b.ao.len();
    let z = || DMatrix::zeros(n, n);
    let mut o = Ints {
        s: DMatrix::identity(n, n),
        dp: [z(), z(), z()],
        qp: [z(), z(), z(), z(), z(), z()],
    };
    for i in 0..n {
        let (_, _, q) = pair(el, pos, b.ao[i], b.ao[i]);
        for c in 0..6 {
            o.qp[c][(i, i)] = q[c];
        }
        for j in (i + 1)..n {
            let (ai, aj) = (b.ao[i].0, b.ao[j].0);
            let (s, d, q) = pair(el, pos, b.ao[i], b.ao[j]);
            if ai == aj {
                for k in 0..3 {
                    o.dp[k][(i, j)] = d[k];
                    o.dp[k][(j, i)] = d[k];
                }
                for c in 0..6 {
                    o.qp[c][(i, j)] = q[c];
                    o.qp[c][(j, i)] = q[c];
                }
                continue;
            }
            o.s[(i, j)] = s;
            o.s[(j, i)] = s;
            shifted(&mut o, i, j, s, d, q, sub(&pos[ai], &pos[aj]));
        }
    }
    o
}

fn sub(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Stores the integrals of (i, j) about the atom of i and, shifted, about the atom of j (traceless quadrupoles).
fn shifted(o: &mut Ints, i: usize, j: usize, s: f64, d: [f64; 3], q: [f64; 6], r: [f64; 3]) {
    for k in 0..3 {
        o.dp[k][(i, j)] = d[k];
        o.dp[k][(j, i)] = d[k] + r[k] * s;
    }
    let raw = [
        2.0 * d[0] * r[0] + s * r[0] * r[0],
        d[0] * r[1] + d[1] * r[0] + s * r[0] * r[1],
        2.0 * d[1] * r[1] + s * r[1] * r[1],
        d[0] * r[2] + d[2] * r[0] + s * r[0] * r[2],
        d[1] * r[2] + d[2] * r[1] + s * r[1] * r[2],
        2.0 * d[2] * r[2] + s * r[2] * r[2],
    ];
    let tr = 0.5 * (raw[0] + raw[2] + raw[5]);
    for c in 0..6 {
        o.qp[c][(i, j)] = q[c];
        let diag = matches!(c, 0 | 2 | 5);
        o.qp[c][(j, i)] = q[c] + 1.5 * raw[c] - if diag { tr } else { 0.0 };
    }
}

/// D3 covalent radius (Pyykkö and Atsumi 2009, ×4/3) in bohr, for the elements of `mol3d`.
fn covalent_d3(z: u8) -> f64 {
    let r = match z {
        1 => 0.32,
        6 => 0.75,
        7 => 0.71,
        8 => 0.63,
        9 => 0.64,
        15 => 1.10,
        16 => 1.02,
        17 => 0.99,
        35 => 1.14,
        53 => 1.32,
        _ => 1.20,
    };
    4.0 / 3.0 * r * ANGSTROM_TO_BOHR
}

/// Coordination numbers with the double-exponential counting function of GFN2 (positions in bohr).
pub fn coordination(el: &[u8], pos: &[[f64; 3]]) -> Vec<f64> {
    let count = |k: f64, r: f64, r0: f64| 1.0 / (1.0 + (-k * (r0 / r - 1.0)).exp());
    let mut cn = vec![0.0; el.len()];
    for i in 0..el.len() {
        for j in (i + 1)..el.len() {
            let r = dist(&pos[i], &pos[j]);
            if !(0.1..=25.0).contains(&r) {
                continue;
            }
            let rc = covalent_d3(el[i]) + covalent_d3(el[j]);
            let c = count(10.0, r, rc) * count(20.0, r, rc + 2.0);
            cn[i] += c;
            cn[j] += c;
        }
    }
    cn
}

/// Zeroth-order Hamiltonian (Hartree): CN-shifted self-energies and the GFN2 off-diagonal formula.
pub fn h0(el: &[u8], pos: &[[f64; 3]], b: &Basis, s: &DMatrix<f64>, cn: &[f64]) -> DMatrix<f64> {
    let n = b.ao.len();
    let mut h = DMatrix::zeros(n, n);
    for i in 0..n {
        let (a, sh, _, _) = b.ao[i];
        let p = par(el[a]);
        h[(i, i)] = (p.selfenergy[sh] - p.kcn[sh] * cn[a]) * EV_TO_HARTREE;
    }
    for i in 0..n {
        let (a, sa, la, _) = b.ao[i];
        let pa = par(el[a]);
        for j in (i + 1)..n {
            let (c, sc, lc, _) = b.ao[j];
            if a == c || s[(i, j)].abs() < 1e-15 {
                continue;
            }
            let pc = par(el[c]);
            let (zi, zj) = (pa.slater[sa], pc.slater[sc]);
            let zij = (2.0 * (zi * zj).sqrt() / (zi + zj)).sqrt();
            let den = pa.pauling_en - pc.pauling_en;
            let enp = 1.0 + 0.02 * den * den;
            let k = gfn2_params::kshell_factor(la, lc);
            let rr = (dist(&pos[a], &pos[c]) / (pa.atomic_rad + pc.atomic_rad)).sqrt();
            let poly = (1.0 + pa.shpoly[sa] * rr) * (1.0 + pc.shpoly[sc] * rr);
            let v = 0.5 * (h[(i, i)] + h[(j, j)]) * s[(i, j)] * zij * k * enp * poly;
            h[(i, j)] = v;
            h[(j, i)] = v;
        }
    }
    h
}

/// Shell-pair Coulomb kernel γ (Hartree) with the GFN2 arithmetic average and exponent 2.
pub fn gamma(pos: &[[f64; 3]], b: &Basis) -> DMatrix<f64> {
    let n = b.shells.len();
    let mut g = DMatrix::zeros(n, n);
    for i in 0..n {
        g[(i, i)] = b.shells[i].eta;
        for j in (i + 1)..n {
            let (si, sj) = (&b.shells[i], &b.shells[j]);
            let eta = 0.5 * (si.eta + sj.eta);
            let v = if si.atom == sj.atom {
                eta
            } else {
                let r = dist(&pos[si.atom], &pos[sj.atom]);
                1.0 / (r * r + 1.0 / (eta * eta)).sqrt()
            };
            g[(i, j)] = v;
            g[(j, i)] = v;
        }
    }
    g
}
