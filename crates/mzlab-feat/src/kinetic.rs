//! Kinetics as evidence: the shape of a product over the treatment time (formation, rise and fall, flat, decay) chosen by `AICc`, its stability
//! under resampling and the generation (first or second) of the product. Port of `TP_Mine/py/tpmine/hr/kinetics.py` (chain models solved
//! with a Pade matrix exponential) plus the four shapes. Numbers to sort and weigh, never a verdict. `libm` for `exp`/`ln`, fixed order sums.

mod bank;
mod evidence;
mod expm;
mod forms;
mod generation;
mod resample;
mod saturation;

pub use bank::{aicc, grid};
pub use evidence::{
    evidence_from_kinetics, kinetic_report, parent_kinetics, Controls, Coupling, Evidence,
    ParentKinetics, ParentSource, Report, Setup, Sign, COUPLING_FACTOR, POINTS_IF_WEIGHED,
    POINTS_MAX,
};
pub use forms::{bateman, choice, fit_forms, Form, FormBank, FormFit};
pub use generation::{generation, ChainBank, Gen, Generation, K_PARENT_MIN, MARGIN};
pub use resample::{Resample, SplitMix64, Stability};
pub use saturation::{parent_saturation, Saturation};

/// Version of the rules of this module (changes when its output changes).
pub const KINETIC_VERSION: u32 = 2;

/// Concentration of the third species of A -> I -> B -> sink, for the parity tests with the Python reference.
pub use expm::chain3;
