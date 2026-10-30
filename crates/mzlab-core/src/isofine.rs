//! `api/isotope_fine`: the fine isotope structure of one ion, for the isotope simulation of the spectrum panel (`hr.js`). Query `n` = `C:13,H:25,N:4`
//! (element counts of the ion), `z` = charge (default 1), `min` = lowest peak kept, % of the tallest (default 0.1). Answer: `[[m/z, %],...]` sorted by m/z,
//! every isotopologue kept (no cut above the lightest one: charge-distribution series of macromolecules).

use crate::error::{Error, Result};
use crate::isoscore::fine_pattern_cut;
use crate::json::Out;
use crate::routes::Query;

pub fn isotope_fine_route(query: &str) -> Result<String> {
    let q = Query::parse(query);
    let bad = || Error::new("err.route.unavailable");
    let mut counts = Vec::new();
    for part in q
        .get("n")
        .unwrap_or("")
        .split(',')
        .filter(|p| !p.is_empty())
    {
        let (el, k) = part.split_once(':').ok_or_else(bad)?;
        counts.push((el.to_string(), k.parse::<i64>().map_err(|_| bad())?));
    }
    let z = q
        .some("z")
        .map_or(Ok(1), |s| s.parse::<i32>())
        .map_err(|_| bad())?;
    let min_rel = q
        .some("min")
        .map_or(Ok(0.1), |s| s.parse::<f64>())
        .map_err(|_| bad())?;
    if counts.iter().all(|(_, k)| *k <= 0) || z == 0 {
        return Err(bad());
    }
    let mut o = Out::default();
    o.raw("[");
    for (i, p) in fine_pattern_cut(&counts, z, min_rel, f64::INFINITY)?
        .iter()
        .enumerate()
    {
        o.raw(if i > 0 { ",[" } else { "[" })
            .num(p.mz)
            .raw(",")
            .num(p.rel)
            .raw("]");
    }
    o.raw("]");
    Ok(o.0)
}
