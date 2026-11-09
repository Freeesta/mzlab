//! The silicon isotopes of the composition search (`composition::isotope_ratios`) are the IUPAC abundances of the isotope score (`isoscore`).

use mzlab_core::composition::isotope_ratios;
use mzlab_core::isoscore::isotopes_of;

#[test]
fn silicon_ratios_use_the_isotope_score_abundances() {
    let si = isotopes_of("Si").expect("Si in the isotope table");
    let (r1, r2) = isotope_ratios(&[("Si".to_string(), 1)]);
    assert_eq!(r1, si[1].1 / si[0].1);
    assert_eq!(r2, si[2].1 / si[0].1);
}
