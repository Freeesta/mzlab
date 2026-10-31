//! The native executor of `export.inclusion`: the expected products (a JSON array of `{formula, rt, priority?}`, or the `.mzfeat` of `tp.link`) become the inclusion list
//! of `mzlab_feat::export_inclusione`, in the columns of the list of mzFinder.

use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;
use mzlab_feat::export_inclusione_tp::inclusion_csv_of;

use crate::flow::{io, read_one, FileRunner};

/// The CSV of the list from the products (a JSON list or the `.mzfeat` of `tp.link`), the adducts (comma separated names, or a list of the user as CSV;
/// empty = the default of the polarity of the products) and the half window of RT in minutes.
pub(crate) fn inclusion_text(products: &[u8], adducts: &str, half_window: f64) -> Result<String> {
    inclusion_csv_of(products, Some(adducts), half_window)
}

/// The executor: the adducts and the half window come from the parameters, the products from the port `tp`.
pub(crate) fn inclusion_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let find = |name: &str| {
        job.resolved
            .params
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v)
    };
    let adducts = find("adducts")
        .and_then(|v| v.str())
        .unwrap_or_default()
        .to_string();
    let half = find("rt_window").and_then(|v| v.f64()).unwrap_or(0.5);
    let products = read_one(r, job, "tp")?;
    let text = inclusion_text(&products.1, &adducts, half)?;
    let out = r.work.join(format!("{}.csv", job.instance.id));
    std::fs::write(&out, text).map_err(io)?;
    r.keep(out, DataType::Report)
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

    #[test]
    fn a_list_of_the_user_gives_the_rows_of_the_table_adduct_it_equals() {
        let j = br#"[{"formula":"C13H24N4O3S","rt":6.0,"priority":1}]"#;
        let list = "name;delta;charge;comment\nmine;H1;1;the proton";
        assert_eq!(
            inclusion_text(j, list, 0.5).unwrap(),
            inclusion_text(j, "[M+H]+", 0.5).unwrap()
        );
        let e = inclusion_text(j, "name;delta;charge\nbad;H1;2", 0.5).unwrap_err();
        assert_eq!(e.key, "error.export.adduct");
        assert!(inclusion_text(j, "name;delta\nx;", 0.5).is_err());
    }
}
