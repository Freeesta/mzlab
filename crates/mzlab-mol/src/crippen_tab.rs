//! Atomic contributions (logP, molar refractivity) of the Wildman-Crippen model, copied by hand from the published table (S. A. Wildman and
//! G. M. Crippen, J. Chem. Inf. Comput. Sci. 1999, 39, 868-873, Table 1); the atom typing that picks one is in `crippen.rs`. A type with no
//! published MR has 0.

/// (logP, MR) of one atom type.
pub(crate) type Ty = (f64, f64);

pub(crate) const C1: Ty = (0.1441, 2.503);
pub(crate) const C2: Ty = (0.0, 2.433);
pub(crate) const C3: Ty = (-0.2035, 2.753);
pub(crate) const C4: Ty = (-0.2051, 2.731);
pub(crate) const C5: Ty = (-0.2783, 5.007);
pub(crate) const C6: Ty = (0.1551, 3.513);
pub(crate) const C7: Ty = (0.0017, 3.888);
pub(crate) const C8: Ty = (0.08452, 2.464);
pub(crate) const C9: Ty = (-0.1444, 2.412);
pub(crate) const C10: Ty = (-0.0516, 2.488);
pub(crate) const C11: Ty = (0.1193, 2.582);
pub(crate) const C12: Ty = (-0.0967, 2.576);
pub(crate) const C13: Ty = (-0.5443, 4.041);
pub(crate) const C14: Ty = (0.0, 3.257);
pub(crate) const C15: Ty = (0.245, 3.564);
pub(crate) const C16: Ty = (0.198, 3.18);
pub(crate) const C17: Ty = (0.0, 3.104);
pub(crate) const C18: Ty = (0.1581, 3.35);
pub(crate) const C19: Ty = (0.2955, 4.346);
pub(crate) const C20: Ty = (0.2713, 3.904);
pub(crate) const C21: Ty = (0.136, 3.509);
pub(crate) const C22: Ty = (0.4619, 4.067);
pub(crate) const C23: Ty = (0.5437, 3.853);
pub(crate) const C24: Ty = (0.1893, 2.673);
pub(crate) const C25: Ty = (-0.8186, 3.135);
pub(crate) const C26: Ty = (0.264, 4.305);
pub(crate) const C27: Ty = (0.2148, 2.693);
pub(crate) const CS: Ty = (0.08129, 3.243);

pub(crate) const H1: Ty = (0.123, 1.057);
pub(crate) const H2: Ty = (-0.2677, 1.395);
pub(crate) const H3: Ty = (0.2142, 0.9627);
pub(crate) const H4: Ty = (0.298, 1.805);
pub(crate) const HS: Ty = (0.1125, 1.112);

pub(crate) const N1: Ty = (-1.019, 2.262);
pub(crate) const N2: Ty = (-0.7096, 2.173);
pub(crate) const N3: Ty = (-1.027, 2.827);
pub(crate) const N4: Ty = (-0.5188, 3.0);
pub(crate) const N5: Ty = (0.08387, 1.757);
pub(crate) const N6: Ty = (0.1836, 2.428);
pub(crate) const N7: Ty = (-0.3187, 1.839);
pub(crate) const N8: Ty = (-0.4458, 2.819);
pub(crate) const N9: Ty = (0.01508, 1.725);
pub(crate) const N10: Ty = (-1.95, 0.0);
pub(crate) const N11: Ty = (-0.3239, 2.202);
pub(crate) const N12: Ty = (-1.119, 0.0);
pub(crate) const N13: Ty = (-0.3396, 0.2604);
pub(crate) const N14: Ty = (0.2887, 3.359);
pub(crate) const NS: Ty = (-0.4806, 2.134);

pub(crate) const O1: Ty = (0.1552, 1.08);
pub(crate) const O2: Ty = (-0.2893, 0.8238);
pub(crate) const O3: Ty = (-0.0684, 1.085);
pub(crate) const O4: Ty = (-0.4195, 1.182);
pub(crate) const O5: Ty = (0.0335, 3.367);
pub(crate) const O6: Ty = (-0.3339, 0.7774);
pub(crate) const O7: Ty = (-1.189, 0.0);
pub(crate) const O8: Ty = (0.1788, 3.135);
pub(crate) const O9: Ty = (-0.1526, 0.0);
pub(crate) const O10: Ty = (0.1129, 0.2215);
pub(crate) const O11: Ty = (0.4833, 0.389);
pub(crate) const O12: Ty = (-1.326, 0.0);
pub(crate) const OS: Ty = (-0.1188, 0.6865);

pub(crate) const F: Ty = (0.4202, 1.108);
pub(crate) const CL: Ty = (0.6895, 5.853);
pub(crate) const BR: Ty = (0.8456, 8.927);
pub(crate) const I: Ty = (0.8857, 14.02);
/// Charged halogens and alkali cations.
pub(crate) const HAL: Ty = (-2.996, 0.0);
pub(crate) const P: Ty = (0.8612, 6.92);
/// Charged or hypervalent aliphatic sulfur.
pub(crate) const S2: Ty = (-0.0024, 7.365);
pub(crate) const S1: Ty = (0.6482, 7.591);
pub(crate) const S3: Ty = (0.6237, 6.691);
/// Main-group metals and metalloids (Li, Be, B, Si, As, Se and their groups).
pub(crate) const ME1: Ty = (-0.3808, 5.754);
/// Transition metals.
pub(crate) const ME2: Ty = (-0.0025, 0.0);
