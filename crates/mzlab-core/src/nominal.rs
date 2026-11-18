//! The nominal mass of a formula (sum of the mass numbers of the most abundant isotopes), split off `elements`.

use crate::elements::{mass, nuclide, Formula};

/// The nominal mass of a formula: the sum of the mass numbers of the most abundant isotope of each element (Murray et al., Pure Appl. Chem. 2013,
/// 85, 1515); an element outside the table counts by its rounded exact mass.
pub fn nominal_mass(f: &Formula) -> f64 {
    f.iter()
        .map(|(e, n)| {
            let a = nuclide(e, None).map_or_else(
                || mass(&vec![(e.clone(), 1)]).round(),
                |(a, _)| f64::from(a),
            );
            a * *n as f64
        })
        .sum()
}
