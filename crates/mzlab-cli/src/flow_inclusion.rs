//! The native executor of `export.inclusion`: the expected products (a JSON array of `{formula, rt, priority?}`) become the inclusion list
//! of `mzlab_feat::export_inclusione`, in the columns of the list of mzFinder.

use mzlab_core::error::{Error, Result};
use mzlab_core::json;
use mzlab_feat::export_inclusione::{inclusion_expected, Expected};

/// The CSV of the list from the products, the adducts (comma separated) and the half window of RT in minutes.
pub(crate) fn inclusion_text(products: &[u8], adducts: &str, half_window: f64) -> Result<String> {
    let v = json::parse(products)?;
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
        });
    }
    let adducts: Vec<&str> = adducts
        .split(',')
        .map(str::trim)
        .filter(|a| !a.is_empty())
        .collect();
    inclusion_expected(&list, &adducts, half_window)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn products_become_the_list() {
        let j = br#"[{"formula":"C13H24N4O3S","rt":6.0,"priority":1}]"#;
        let t = inclusion_text(j, "[M+H]+", 0.5).unwrap();
        assert_eq!(t, "m/z,charge,RT_start_min,RT_end_min,note\n317.1642,1,5.5,6.5,\"C13H24N4O3S priority 1.0\"");
        assert!(inclusion_text(br#"[{"rt":1}]"#, "[M+H]+", 0.5).is_err());
    }
}
