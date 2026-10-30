//! The adducts and the mass tolerance of the suspect screening (Attributio B2 `reglists::screen_types` re-exports them; `flow_screen_suspects` reads them), in a module that needs no feature so
//! that a `.wasm` without Attributio can use them. The shift of an adduct is the mass added to the neutral molecule to get the ion m/z.

pub const PROTON: f64 = 1.007276;
pub const POS_ADDUCTS: [(&str, f64); 3] = [
    ("[M+H]+", PROTON),
    ("[M+NH4]+", 18.033823),
    ("[M+Na]+", 22.989218),
];
pub const NEG_ADDUCTS: [(&str, f64); 2] = [("[M-H]-", -PROTON), ("[M+Cl]-", 34.969402)];
pub const PPM: f64 = 5.0;
