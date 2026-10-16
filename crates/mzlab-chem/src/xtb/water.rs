//! ALPB water for GFN2-xTB: Ehlert, Stahn, Spicher, Grimme, «A Robust and Efficient Implicit Solvation Model for Fast
//! Semiempirical Methods», J. Chem. Theory Comput. 17 (2021) 4250, doi:10.1021/acs.jctc.1c00471; numbers from the public
//! supporting information on ChemRxiv (`parameters.tar`, `parameters/param_alpb2_water.txt`). Header of the file: ε, molar
//! mass, density, Born scale, probe radius (Å), free-energy shift, Born offset, unused; then one line per element Z = 1…86 with
//! surface tension, descreening and hydrogen-bond strength. The surface tension and the shift do not depend on the charges and
//! are not used.

pub const EPSILON: f64 = 80.2;
pub const BORN_SCALE: f64 = 1.47438678;
/// Probe radius (Å).
pub const PROBE: f64 = 1.13409020;
/// kcal/mol per Hartree.
pub const KCAL_PER_HARTREE: f64 = 627.509474;

/// Descreening per element, Z = 1…86.
const DESCREENING: [f64; 86] = [
    0.18678116, 1.99854836, 0.50934487, 0.3, 0.93372749, 0.73948749, 0.77003311, 0.3, 0.62227524,
    1.22892076, // 1-10
    0.65065466, 1.15301804, 1.00517744, 0.93204996, 0.8, 0.3, 0.8, 0.90930774, 0.3,
    1.13114016, // 11-20
    1.06981655, 1.42134411, 1.09146204, 1.14487481, 0.93238901, 1.66030803, 1.03734066, 1.21328994,
    0.96459170, 0.70218231, // 21-30
    0.94458397, 0.87849331, 1.04104293, 1.01890919, 1.42471942, 1.31471665, 0.3, 0.9950673,
    1.31750068, 0.91790577, // 31-40
    1.13352069, 0.90365194, 0.97192416, 1.13693166, 1.13908614, 1.03723586, 0.99373444, 0.57959099,
    0.80138068, 0.45705909, // 41-50
    1.12709787, 0.80213876, 1.37998347, 1.103872, 0.3, 0.3, 0.8, 0.8, 0.8, 0.8, // 51-60
    0.8, 0.8, 0.8, 0.8, 0.8, 0.8, 0.8, 0.8, 0.8, 0.8, // 61-70
    0.8, 1.11415978, 0.95102453, 1.31024463, 1.21914682, 0.91559959, 0.99579666, 0.90140483,
    1.09420509, 0.02609846, // 71-80
    0.49510247, 0.69445869, 0.54304437, 0.8, 0.8, 0.8, // 81-86
];

pub fn descreening(z: u8) -> f64 {
    DESCREENING.get(z as usize - 1).copied().unwrap_or(0.8)
}

/// Hydrogen-bond strength c (√(kcal/mol)); the energy of atom i is −c²·(exposed fraction)·qᵢ².
pub fn hbond(z: u8) -> f64 {
    match z {
        1 => 8.09964704,
        6 => 2.23420044,
        7 => 1.80633279,
        8 => 2.22319193,
        9 => 3.56359195,
        15 => 3.6292682,
        16 => 0.26212102,
        17 => 0.15334756,
        34 => 2.37162766,
        35 => 2.02275702,
        53 => 2.61716906,
        _ => 0.0,
    }
}
