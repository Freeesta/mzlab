//! `formula_mz`: `nominal_neutral` is the nominal mass (sum of the mass numbers of the most abundant isotopes, Murray et al., Pure Appl. Chem. 2013,
//! 85, 1515), `nominal` of an ion that mass plus the adduct shift; the old rounded values stay as `neutral0` and `mz0` (integer m/z).

use mzlab_core::elements::formula_mz;
use serde_json::Value;

fn j(formula: &str, adduct: &str) -> Value {
    serde_json::from_str(&formula_mz(formula, Some(adduct)).unwrap()).unwrap()
}

#[test]
fn tristearin_is_890_not_the_rounded_mass() {
    let v = j("C57H110O6", "[M+H]+");
    assert_eq!(v["nominal_neutral"], 890, "{v}");
    assert_eq!(v["nominal"], 891, "{v}"); // 890 + H+
    assert_eq!(v["adducts"]["[M+Na]+"]["nominal"], 913, "{v}");
    assert_eq!(v["adducts"]["[M-H]-"]["nominal"], 889, "{v}");
    // the rounded neutral mass 890.83 -> 891 and the rounded m/z 891.84 -> 892 are still there
    assert_eq!(v["neutral0"], 891, "{v}");
    assert_eq!(v["mz0"], 892, "{v}");
}

#[test]
fn a_halogenated_formula_uses_the_mass_numbers_of_the_main_isotopes() {
    // C2H4Cl2: 2 x 12 + 4 + 2 x 35 = 98 (exact 97.9690)
    let v = j("C2H4Cl2", "[M+H]+");
    assert_eq!(v["nominal_neutral"], 98, "{v}");
    assert_eq!(v["nominal"], 99, "{v}");
    assert_eq!(v["neutral0"], 98, "{v}");
}
