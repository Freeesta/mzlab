//! The products behind `export.inclusion`, one reader for `flow run` and the browser (`interp` .wasm): a JSON list `[{formula, rt, priority?}]`, or the
//! `.mzfeat` of `tp.link` (accepted product rows with their neutral formula and RT). The default adduct follows the polarity the products were linked in.

use mzlab_core::elements::{self, parse_formula};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Value};

use crate::export_inclusione::{adduct_shifts, inclusion_expected_by, Expected};
use crate::interp::store;
use crate::store_read::{read_table_from, Slice};
use crate::tp::{NO_PARENT, ST_TP};

/// The adduct an unwritten `adducts` takes: `[M-H]-` for negative products, `[M+H]+` otherwise.
pub fn default_adduct(negative: bool) -> &'static str {
    if negative {
        "[M-H]-"
    } else {
        "[M+H]+"
    }
}

/// The expected products of `bytes` and whether they were linked in negative mode (false for a JSON list, which says nothing of the series).
pub fn products_of(bytes: &[u8]) -> Result<(Vec<Expected>, bool)> {
    if bytes.iter().find(|b| !b.is_ascii_whitespace()) == Some(&b'[') {
        return Ok((from_json(bytes)?, false));
    }
    from_mzfeat(bytes)
}

fn from_json(bytes: &[u8]) -> Result<Vec<Expected>> {
    let v = json::parse(bytes)?;
    let mut list = Vec::new();
    for p in v.arr() {
        let formula = p.get("formula").and_then(|x| x.str());
        let rt = p.get("rt").and_then(|x| x.f64());
        let (Some(formula), Some(rt)) = (formula, rt) else {
            return Err(Error::new("err.flow.inclusion.product"));
        };
        let priority = p.get("priority").and_then(|x| x.f64()).unwrap_or(0.0);
        list.push(Expected {
            formula: formula.to_string(),
            rt,
            priority,
            ion_mz: None,
        });
    }
    Ok(list)
}

/// The accepted products (`tp/1` status product) of a `tp.link` file, and whether the series was linked in negative mode: the detection polarity of the recipe, else
/// the sign of observed m/z minus neutral mass of the rows that have a formula. A row without a formula (unit resolution) is listed at its observed ion m/z.
fn from_mzfeat(bytes: &[u8]) -> Result<(Vec<Expected>, bool)> {
    let (table, meta) = read_table_from(&mut Slice(bytes), bytes.len() as u64)?;
    if table.tp.is_empty() || table.tp.parent_row == NO_PARENT {
        return Err(Error::new("error.tpNoBlock"));
    }
    let (_, ids) = store::read_meta(&mut Slice(bytes), bytes.len() as u64)?
        .ok_or_else(|| Error::new("error.rete.noInterp"))?;
    let (mut list, mut sign) = (Vec::new(), 0.0);
    for id in ids {
        let Some(row) = (0..table.len()).find(|&r| table.id(r) == id) else {
            continue;
        };
        if table.tp.status.get(row) != Some(&ST_TP) {
            continue;
        }
        let Some(text) = store::read_row(&mut Slice(bytes), bytes.len() as u64, id)? else {
            continue;
        };
        let v = json::parse(text.as_bytes())?;
        let neutral = v
            .get("change")
            .and_then(|c| c.get("neutral_formula"))
            .and_then(Value::str)
            .filter(|f| !f.is_empty());
        let rt = table.rt[row];
        let seen = table.mz[row];
        list.push(match neutral {
            Some(f) => {
                sign += parse_formula(f).map_or(0.0, |p| seen - elements::mass(&p));
                Expected {
                    formula: f.to_string(),
                    rt,
                    priority: 0.0,
                    ion_mz: None,
                }
            }
            None => Expected {
                formula: format!("observed m/z {seen:.4}"),
                rt,
                priority: 0.0,
                ion_mz: Some(seen),
            },
        });
    }
    let recipe = meta.get("recipe").and_then(Value::str).unwrap_or("");
    let negative = if recipe.contains("polarity: Some(-1)") {
        true
    } else {
        !recipe.contains("polarity: Some(1)") && sign < 0.0
    };
    Ok((list, negative))
}

/// The CSV of the inclusion list from `products` (see `products_of`), the adducts (`None` or empty = the default of the polarity) and the half window of RT in minutes.
pub fn inclusion_csv_of(
    products: &[u8],
    adducts: Option<&str>,
    half_window: f64,
) -> Result<String> {
    let (list, negative) = products_of(products)?;
    let adducts = adducts
        .filter(|a| !a.trim().is_empty())
        .unwrap_or_else(|| default_adduct(negative));
    inclusion_expected_by(&list, &adduct_shifts(adducts)?, half_window)
}
