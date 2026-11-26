//! The m/z a sample name carries (`Analyte_237.1`) against the precursors the file holds: a warning with the numbers, never a blocked result.
//! Heuristic by design (names are free text): a number counts as an m/z mark when it stands alone between separators, is not followed by a unit and is
//! in the range of a precursor; the user reads the numbers and decides.

/// The precursor and the name mass farthest from agreeing at best: the closest pair of the name's masses and the precursors.
#[derive(Debug, Clone, PartialEq)]
pub struct Mismatch {
    pub name_mz: Vec<f64>,
    pub nearest_name_mz: f64,
    pub nearest_precursor: f64,
    pub delta: f64,
}

/// Words that follow a number in a name when it is an amount, a time or a count and not a mass.
const UNITS: [&str; 21] = [
    "ng", "ug", "mg", "pg", "g", "ppb", "ppm", "ppt", "um", "nm", "mm", "pm", "ul", "ml", "l",
    "min", "h", "s", "v", "ev", "da",
];

/// The name without its file extension and the part tag of an item (`x.wiff#MS2`), a decimal point is not an extension.
fn stem(name: &str) -> &str {
    let name = name.split('#').next().unwrap_or(name);
    match name.rsplit_once('.') {
        Some((a, ext)) if ext.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) => a,
        _ => name,
    }
}

/// The masses a name carries, in the order they appear: decimals with 1 to 4 digits in 50..=2500, and integers of 3 or 4 digits in 100..=2500 (not
/// 1990..=2100, years), each alone between separators or right after `mz`, never in front of a unit (`100_ng`, `5ppb`).
pub fn mz_in_name(name: &str) -> Vec<f64> {
    let s: Vec<char> = stem(name).chars().collect();
    let (mut out, mut i) = (Vec::new(), 0);
    while i < s.len() {
        if !s[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let from = i;
        while i < s.len()
            && (s[i].is_ascii_digit()
                || (s[i] == '.' && s.get(i + 1).is_some_and(char::is_ascii_digit)))
        {
            i += 1;
        }
        let text: String = s[from..i].iter().collect();
        let before: String = s[from.saturating_sub(2)..from]
            .iter()
            .collect::<String>()
            .to_lowercase();
        let stuck = from > 0 && s[from - 1].is_ascii_alphabetic() && !before.ends_with("mz");
        let after: String = s[i..]
            .iter()
            .take_while(|c| !matches!(c, '_' | '-' | ' '))
            .collect::<String>()
            .to_lowercase();
        let unit = !after.is_empty()
            && (after.starts_with('%')
                || UNITS.contains(&after.trim_end_matches(|c: char| c.is_ascii_digit())));
        let next_word: String = s[i..]
            .iter()
            .skip_while(|c| matches!(c, '_' | '-' | ' '))
            .take_while(|c| c.is_ascii_alphabetic())
            .collect::<String>()
            .to_lowercase();
        let amount = after.is_empty() && UNITS.contains(&next_word.as_str());
        if stuck || unit || amount {
            continue;
        }
        let Ok(v) = text.parse::<f64>() else { continue };
        let ok = match text.split_once('.') {
            Some((_, d)) => (1..=4).contains(&d.len()) && (50.0..=2500.0).contains(&v),
            None => {
                (3..=4).contains(&text.len())
                    && (100.0..=2500.0).contains(&v)
                    && !(1990.0..=2100.0).contains(&v)
            }
        };
        if ok {
            out.push(v);
        }
    }
    out
}

/// `None` when the name has no mass, the file has no precursor, or some mass of the name is within `tol` of some precursor; else the closest pair.
pub fn mismatch(name: &str, precursors: &[f64], tol: f64) -> Option<Mismatch> {
    let name_mz = mz_in_name(name);
    let mut best: Option<(f64, f64, f64)> = None;
    for &n in &name_mz {
        for &p in precursors.iter().filter(|p| p.is_finite()) {
            let d = (n - p).abs();
            if best.is_none_or(|b| d < b.2) {
                best = Some((n, p, d));
            }
        }
    }
    let (n, p, d) = best?;
    (d > tol).then_some(Mismatch {
        name_mz,
        nearest_name_mz: n,
        nearest_precursor: p,
        delta: ((p - n) * 1e4).round() / 1e4,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masses_are_found_between_separators_and_amounts_are_not() {
        assert_eq!(mz_in_name("Analyte_237.1"), vec![237.1]);
        assert_eq!(mz_in_name("MS3_237-194.wiff"), vec![237.0, 194.0]);
        assert_eq!(mz_in_name("Std_mz253.1_rep2"), vec![253.1]);
        assert_eq!(
            mz_in_name("QC_100_ng_2024"),
            Vec::<f64>::new(),
            "an amount and a year"
        );
        assert_eq!(mz_in_name("CE35_DP60_5ppb_20240115"), Vec::<f64>::new());
        assert_eq!(
            mz_in_name("x_237.1.mzML"),
            vec![237.1],
            "the extension is not a decimal part"
        );
    }

    #[test]
    fn a_mismatch_reports_the_closest_pair_and_a_match_reports_nothing() {
        let m = mismatch("Name_237.1", &[253.1, 300.0], 1.0).unwrap();
        assert_eq!(
            (m.nearest_name_mz, m.nearest_precursor, m.delta),
            (237.1, 253.1, 16.0)
        );
        assert!(mismatch("Name_253", &[253.1], 1.0).is_none());
        assert!(
            mismatch("Name_237.1", &[], 1.0).is_none(),
            "no precursor, nothing to compare"
        );
        assert!(
            mismatch("Name", &[253.1], 1.0).is_none(),
            "no mass in the name"
        );
        assert!(
            mismatch("MS3_237-194", &[194.1], 1.0).is_none(),
            "one of the masses is enough"
        );
    }
}
