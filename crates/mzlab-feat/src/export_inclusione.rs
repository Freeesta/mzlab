//! The inclusion list written from the expected products of a plan (`tp/1` rows or hypotheses) and the adducts chosen: one row per product and adduct,
//! with an RT window around the expected RT, in the columns of the list of mzFinder (`legacy::out::inclusion_row`), sorted by RT. Used by the
//! `export.inclusion` step; it never raises a level and adds no verdict.

use mzlab_core::elements;
use mzlab_core::error::Result;

use crate::legacy::out::{inclusion_row, INCLUSION_HEADER};

/// A product expected in the samples: neutral formula, expected RT in minutes and the priority written in the note.
#[derive(Debug, Clone, PartialEq)]
pub struct Expected {
    pub formula: String,
    pub rt: f64,
    pub priority: f64,
}

/// The inclusion list: every product in every adduct of `adducts` (names of `elements::adducts`), RT window `rt ± half_window` minutes (start not
/// below 0), rows sorted by RT start, then m/z, then formula. An adduct unknown to the table is an error.
pub fn inclusion_expected(
    products: &[Expected],
    adducts: &[&str],
    half_window: f64,
) -> Result<String> {
    let mut rows: Vec<(f64, f64, String, String)> = Vec::new();
    for p in products {
        let neutral = elements::mass(&elements::parse_formula(&p.formula)?);
        for a in adducts {
            let mz = elements::ion_mz(neutral, a).ok_or_else(|| {
                mzlab_core::error::Error::new("error.export.adduct").with("adduct", a)
            })?;
            let (start, end) = ((p.rt - half_window).max(0.0), p.rt + half_window);
            rows.push((
                start,
                mz,
                p.formula.clone(),
                inclusion_row(mz, start, end, &p.formula, p.priority),
            ));
        }
    }
    rows.sort_by(|a, b| {
        a.0.total_cmp(&b.0)
            .then(a.1.total_cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    });
    let mut lines = vec![INCLUSION_HEADER.to_string()];
    lines.extend(rows.into_iter().map(|r| r.3));
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tim(rt: f64) -> Expected {
        // Timolol, C13H24N4O3S.
        Expected {
            formula: "C13H24N4O3S".into(),
            rt,
            priority: 1.0,
        }
    }

    #[test]
    fn rows_sorted_by_rt_with_window() {
        let ps = [
            tim(6.0),
            Expected {
                formula: "C13H24N4O4S".into(),
                rt: 2.0,
                priority: 2.5,
            },
        ];
        let csv = inclusion_expected(&ps, &["[M+H]+"], 0.5).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "m/z,charge,RT_start_min,RT_end_min,note");
        assert!(lines[1].starts_with("333.1591,1,1.5,2.5,\"C13H24N4O4S priority 2.5\""));
        assert!(lines[2].starts_with("317.1642,1,5.5,6.5,\"C13H24N4O3S priority 1.0\""));
        assert_eq!(csv, inclusion_expected(&ps, &["[M+H]+"], 0.5).unwrap());
    }

    #[test]
    fn window_start_not_below_zero_and_unknown_adduct() {
        let csv = inclusion_expected(&[tim(0.2)], &["[M+H]+"], 0.5).unwrap();
        assert!(csv.contains(",1,0.0,0.7,"));
        assert!(inclusion_expected(&[tim(1.0)], &["[M+X]+"], 0.5).is_err());
    }
}
