//! The ion library of `features.group`: the ion species a row may be, as `m/z = (n·M + shift) / |z|`. Names are the ones of the ion identity
//! networking of mzmine/GNPS (Schmid et al. 2021, doi:10.1038/s41467-021-23953-9), so the edges read the same in GNPS.

use mzlab_core::error::{Error, Result};

const PROTON: f64 = mzlab_core::elements::PROTON_6DP;
const WATER: f64 = 18.010565;

/// Every species the step knows; a plan names the ones it searches.
pub const LIBRARY: &[Ion] = &[
    Ion {
        name: "[M+H]+",
        z: 1,
        n: 1,
        shift: PROTON,
    },
    Ion {
        name: "[M+Na]+",
        z: 1,
        n: 1,
        shift: 22.989218,
    },
    Ion {
        name: "[M+NH4]+",
        z: 1,
        n: 1,
        shift: 18.033823,
    },
    Ion {
        name: "[M+K]+",
        z: 1,
        n: 1,
        shift: 38.963158,
    },
    Ion {
        name: "[M+H-H2O]+",
        z: 1,
        n: 1,
        shift: PROTON - WATER,
    },
    Ion {
        name: "[M+H-2H2O]+",
        z: 1,
        n: 1,
        shift: PROTON - 2.0 * WATER,
    },
    Ion {
        name: "[M+2H]2+",
        z: 2,
        n: 1,
        shift: 2.0 * PROTON,
    },
    Ion {
        name: "[M+H+Na]2+",
        z: 2,
        n: 1,
        shift: PROTON + 22.989218,
    },
    Ion {
        name: "[2M+H]+",
        z: 1,
        n: 2,
        shift: PROTON,
    },
    Ion {
        name: "[2M+Na]+",
        z: 1,
        n: 2,
        shift: 22.989218,
    },
    Ion {
        name: "[2M+NH4]+",
        z: 1,
        n: 2,
        shift: 18.033823,
    },
    Ion {
        name: "[M-H]-",
        z: -1,
        n: 1,
        shift: -PROTON,
    },
    Ion {
        name: "[M+Cl]-",
        z: -1,
        n: 1,
        shift: 34.969402,
    },
    Ion {
        name: "[M+HCOO]-",
        z: -1,
        n: 1,
        shift: 44.998201,
    },
    Ion {
        name: "[M+CH3COO]-",
        z: -1,
        n: 1,
        shift: 59.013851,
    },
    Ion {
        name: "[M-H-H2O]-",
        z: -1,
        n: 1,
        shift: -PROTON - WATER,
    },
    Ion {
        name: "[M-2H]2-",
        z: -2,
        n: 1,
        shift: -2.0 * PROTON,
    },
    Ion {
        name: "[2M-H]-",
        z: -1,
        n: 2,
        shift: -PROTON,
    },
    Ion {
        name: "[2M+HCOO]-",
        z: -1,
        n: 2,
        shift: 44.998201,
    },
];

/// The default list of each polarity (the species of the IIMN examples).
pub const DEFAULT_POS: &str = "[M+H]+,[M+Na]+,[M+NH4]+,[M+K]+,[M+H-H2O]+,[M+2H]2+,[2M+H]+,[2M+Na]+";
pub const DEFAULT_NEG: &str = "[M-H]-,[M+Cl]-,[M+HCOO]-,[M+CH3COO]-,[M-H-H2O]-,[M-2H]2-,[2M-H]-";

/// One ion species: `n` molecules, charge `z`, `shift` = mass added to `n·M` (electrons included).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ion {
    pub name: &'static str,
    pub z: i8,
    pub n: u8,
    pub shift: f64,
}

impl Ion {
    /// The neutral mass this ion gives for an observed `mz`.
    pub fn neutral(&self, mz: f64) -> f64 {
        (mz * f64::from(self.z.unsigned_abs()) - self.shift) / f64::from(self.n)
    }
}

/// The species of a comma-separated list, in its order; an unknown name or a species of the other polarity is `err.flow.group.ion`.
pub fn parse_list(text: &str, positive: bool) -> Result<Vec<Ion>> {
    let mut out: Vec<Ion> = Vec::new();
    for name in text.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let ion = LIBRARY
            .iter()
            .find(|i| i.name == name && (i.z > 0) == positive)
            .ok_or_else(|| Error::new("err.flow.group.ion").with("ion", name))?;
        if !out.iter().any(|i| i.name == ion.name) {
            out.push(*ion);
        }
    }
    Ok(out)
}
