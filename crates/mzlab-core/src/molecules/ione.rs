//! What a typed formula or m/z stands for: the neutral molecule M or an ion of the adduct table. The parent of a Flow plan is always kept as the neutral
//! formula (and mass), so the search for products, the mass tolerances and the plan all read the same molecule whatever the user typed.

use crate::composition::adduct_atoms;
use crate::elements::{adducts, fmt, mass, parse_formula, Formula};
use crate::error::{Error, Result};
use crate::json::{round, Out};

/// The adduct name that means "the text is M itself".
pub const NEUTRAL: &str = "neutral";

/// `q` is a formula (an ion with the atoms of `adduct` inside it) or an m/z, `adduct` is `neutral` or a name of the adduct table. JSON: `kind` (`formula` or
/// `mass`), `adduct`, `formula` (neutral, Hill order; empty for an m/z), `massa` (neutral monoisotopic, 6 decimals) and `mz` (the ion m/z; null for neutral).
pub fn ione(q: &str, adduct: &str) -> Result<String> {
    let q = q.trim();
    let shift = if adduct == NEUTRAL {
        None
    } else {
        Some(
            adducts()
                .iter()
                .find(|(a, _)| *a == adduct)
                .map(|(_, s)| *s)
                .ok_or_else(|| Error::new("err.comp.ion").with("ion", adduct))?,
        )
    };
    let mut o = Out::default();
    o.raw("{\"adduct\":").str(adduct);
    if let Some(mz) = q.parse::<f64>().ok().filter(|x| x.is_finite()) {
        let neutral = mz - shift.unwrap_or(0.0);
        if neutral <= 0.0 {
            return Err(Error::new("err.comp.mz"));
        }
        o.raw(",\"formula\":\"\",\"kind\":\"mass\",\"massa\":")
            .num(round(neutral, 6));
        o.raw(",\"mz\":").opt(shift.map(|_| round(mz, 6)));
    } else {
        let ion = parse_formula(q)?;
        let neutral = match shift {
            None => ion,
            Some(_) => take_adduct(&ion, adduct)?,
        };
        let m = mass(&neutral);
        o.raw(",\"formula\":").str(&fmt(&neutral));
        o.raw(",\"kind\":\"formula\",\"massa\":").num(round(m, 6));
        o.raw(",\"mz\":").opt(shift.map(|s| round(m + s, 6)));
    }
    o.raw("}");
    Ok(o.0)
}

/// The formula of M from the formula of the ion: the atoms of the adduct taken away (or put back, as in `[M-H]-`); an adduct that the ion does not hold is an error.
fn take_adduct(ion: &Formula, adduct: &str) -> Result<Formula> {
    let atoms = adduct_atoms(adduct).map_or(&[][..], |a| a.0);
    let mut f: Vec<(String, i64)> = ion.iter().map(|(e, n)| (e.clone(), *n as i64)).collect();
    for (el, n) in atoms {
        match f.iter_mut().find(|(e, _)| e == el) {
            Some(x) => x.1 -= n,
            None => f.push((el.to_string(), -n)),
        }
    }
    if f.iter().any(|x| x.1 < 0) || f.iter().all(|x| x.1 == 0) {
        return Err(Error::new("err.ion.notHeld").with("ion", adduct));
    }
    Ok(f.into_iter()
        .filter(|x| x.1 > 0)
        .map(|(e, n)| (e, n as u64))
        .collect())
}
