//! Tests of the `tables/1` entry: invented two-row tables, the facts of the meta, the step from `prov/1` and from the page.

use super::*;
use crate::aligned::{Aligned, CELL_DETECTED};
use crate::recipe::Recipe;
use crate::store::{write_table, Origin};
use crate::store_read::Slice;
use mzlab_core::json::parse;

fn table(polarity: Option<i8>) -> Vec<u8> {
    let mut t = Aligned::with_capacity(2, 2);
    for (mz, rt) in [(200.1, 1.5), (250.2, 2.5)] {
        t.mz.push(mz);
        t.rt.push(rt);
        t.area.push(1e6);
        t.height.push(1e5);
        t.apex_rt.push(rt as f32);
        t.width.push(0.1);
        t.state.push(CELL_DETECTED);
    }
    let mut r = Recipe::default();
    r.detect.polarity = polarity;
    write_table(&t, &r, &Origin::default())
}

fn entry_of(b: &[u8], made: Option<&Made>) -> String {
    entry("gap.mzfeat", &mut Slice(b), b.len() as u64, made).unwrap()
}

const PROV: &str = "{\"schema\":\"prov/1\",\"entity\":{},\"activity\":{\"gap\":{\"step\":\"features.gapfill@1\",\"ms\":12.34,\"warnings\":[\"w\"]},\"gap2\":{\"step\":\"features.align@1\",\"ms\":1,\"cached\":true}}}";

#[test]
fn an_entry_without_a_step_has_the_facts_of_the_meta() {
    let b = table(Some(1));
    let e = entry_of(&b, None);
    let v = parse(e.as_bytes()).unwrap();
    assert_eq!(v.get("rows").and_then(|x| x.f64()), Some(2.0));
    assert_eq!(v.get("files").and_then(|x| x.f64()), Some(2.0));
    assert_eq!(v.get("polarity").and_then(|x| x.f64()), Some(1.0));
    assert_eq!(v.get("bytes").and_then(|x| x.f64()), Some(b.len() as f64));
    assert_eq!(v.get("identity").and_then(|x| x.bool()), Some(false));
    assert!(v.get("instance").is_none());
}

#[test]
fn the_polarity_is_left_out_when_the_recipe_does_not_say() {
    assert_eq!(recipe_polarity("polarity: Some(-1)"), -1);
    assert_eq!(recipe_polarity("polarity: Some(0)"), 0);
    assert_eq!(recipe_polarity("polarity: None"), 0);
    let e = entry_of(&table(None), None);
    assert!(!e.contains("polarity"), "{e}");
    let e = entry_of(&table(Some(-1)), None);
    assert!(e.contains("\"polarity\":-1"), "{e}");
}

#[test]
fn the_step_comes_from_the_prov_activity() {
    let prov = parse(PROV.as_bytes()).unwrap();
    assert_eq!(instance_of(&prov, "gap.mzfeat").as_deref(), Some("gap"));
    assert_eq!(instance_of(&prov, "gap2-3.mzfeat").as_deref(), Some("gap2"));
    assert_eq!(instance_of(&prov, "gapx.mzfeat"), None);
    let m = Made::from_prov(&prov, "gap").unwrap();
    assert_eq!(
        (m.step.as_str(), m.status.as_str(), m.warnings),
        ("features.gapfill@1", "ok", 1)
    );
    let e = entry_of(&table(None), Some(&m));
    assert!(e.ends_with("\"instance\":\"gap\",\"step\":\"features.gapfill@1\",\"status\":\"ok\",\"ms\":12.3,\"warnings\":1}"), "{e}");
    assert_eq!(Made::from_prov(&prov, "gap2").unwrap().status, "cached");
}

#[test]
fn the_page_and_prov_give_the_same_bytes() {
    let prov = parse(PROV.as_bytes()).unwrap();
    let from_prov = Made::from_prov(&prov, "gap").unwrap();
    let page = parse(b"{\"instance\":\"gap\",\"step\":\"features.gapfill@1\",\"status\":\"ok\",\"ms\":12.34,\"warnings\":1}").unwrap();
    let from_page = Made::from_json(&page).unwrap();
    let b = table(None);
    assert_eq!(
        entry_of(&b, Some(&from_prov)),
        entry_of(&b, Some(&from_page))
    );
}

#[test]
fn a_file_that_is_not_a_table_is_an_error_and_the_document_lists_it() {
    let err = entry("x.mzfeat", &mut Slice(b"not a table at all"), 18, None).unwrap_err();
    assert_eq!(err.key, "error.badMzfeat");
    let b = table(None);
    let d = document(
        &[entry_of(&b, None)],
        &[("x.mzfeat".into(), err.key.into())],
    );
    let v = parse(d.as_bytes()).unwrap();
    assert_eq!(v.get("tables").unwrap().arr().len(), 1);
    assert_eq!(
        v.get("skipped")
            .unwrap()
            .at(0)
            .unwrap()
            .get("error_key")
            .and_then(|x| x.str()),
        Some("error.badMzfeat")
    );
}
