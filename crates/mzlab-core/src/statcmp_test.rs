//! `statcmp::compare` on a hand-made table: t0, t1..t5 and two MQ blanks, with a parent, two products, a UV artefact, a background and a row below the pseudocount.

use crate::json::Value;
use crate::statcmp::{compare, plan, CmpCol, CmpIn, CmpOpts};

const NAMES: [&str; 8] = ["t0", "t1", "t2", "t3", "t4", "t5", "MQ_t0", "MQ_t5"];

fn rows() -> Vec<(&'static str, [f64; 8])> {
    vec![
        ("parent", [1e6, 8e5, 5e5, 2e5, 1e5, 5e4, 0.0, 0.0]),
        ("tp_new", [0.0, 5e3, 2e4, 5e4, 9e4, 1.2e5, 0.0, 0.0]),
        ("tp_up", [2e4, 3e4, 5e4, 8e4, 1e5, 1.1e5, 0.0, 0.0]),
        ("uv", [0.0, 1e3, 5e3, 2e4, 4e4, 6e4, 500.0, 3e4]),
        ("background", [5e4, 5e4, 5e4, 5e4, 5e4, 5e4, 800.0, 800.0]),
        ("small", [500.0, 600.0, 700.0, 800.0, 900.0, 1000.0, 0.0, 0.0]),
    ]
}

fn run(names: &[String], data: &[(&str, [f64; 8])], meta: Option<&[Option<CmpCol>]>, o: &CmpOpts) -> Value {
    let ids: Vec<String> = data.iter().map(|d| d.0.to_string()).collect();
    let matrix: Vec<Vec<f64>> = data.iter().map(|d| d.1.to_vec()).collect();
    let (mz, rt) = (vec![200.0; data.len()], vec![5.0; data.len()]);
    let inp = CmpIn { path: "t.csv", sha256: "00", ids: &ids, mz: &mz, rt: &rt, matrix: &matrix, names, meta };
    compare(&inp, o).ok().expect("compare")
}

fn opts() -> CmpOpts {
    CmpOpts { ref_sel: "t0".into(), blank_sel: "MQ*".into(), ..CmpOpts::default() }
}

fn names() -> Vec<String> {
    NAMES.iter().map(|s| s.to_string()).collect()
}

fn row<'a>(doc: &'a Value, id: &str) -> &'a Value {
    doc.get("rows").unwrap().arr().iter().find(|r| r.get("id").unwrap().str() == Some(id)).unwrap()
}

fn class(doc: &Value, id: &str) -> (String, Vec<String>) {
    let r = row(doc, id);
    (r.get("class").unwrap().str().unwrap().into(), r.get("reasons").unwrap().arr().iter().map(|v| v.str().unwrap().to_string()).collect())
}

#[test]
fn every_row_gets_its_class_and_reason() {
    let doc = run(&names(), &rows(), None, &opts());
    let c = |id| class(&doc, id);
    assert_eq!(c("parent"), ("decreasing".to_string(), vec!["falls".to_string()]));
    assert_eq!(c("tp_new"), ("candidate".to_string(), vec!["absent_in_reference".to_string(), "above_reference".to_string()]));
    assert_eq!(c("tp_up"), ("candidate".to_string(), vec!["above_reference".to_string()]));
    assert_eq!(c("uv"), ("blank_artefact".to_string(), vec!["blank_level".to_string()]));
    assert_eq!(c("background"), ("unchanged".to_string(), vec![]));
    assert_eq!(c("small"), ("no_signal".to_string(), vec!["below_pseudocount".to_string()]));
    assert_eq!(doc.get("rows").unwrap().arr().len(), 6, "nothing is removed");
    let n = |k: &str| doc.get("counts").unwrap().get(k).unwrap().f64().unwrap();
    assert_eq!([n("candidate"), n("blank_artefact"), n("decreasing"), n("unchanged"), n("no_signal")], [2.0, 1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn the_pseudocount_comes_from_the_blanks_and_the_ratios_use_it() {
    let doc = run(&names(), &rows(), None, &opts());
    let p = doc.get("params").unwrap();
    // positive blank values 500, 800, 800, 30000: median 800, x3 = 2400
    assert_eq!(p.get("pseudocount_origin").unwrap().str(), Some("blanks"));
    assert_eq!(p.get("pseudocount").unwrap().f64(), Some(2400.0));
    let r = row(&doc, "tp_up");
    let want = (1.1e5 + 2400.0) / (2e4 + 2400.0);
    assert!((r.get("ratio_ref").unwrap().f64().unwrap() - want).abs() < 1e-9);
    assert!((r.get("log2fc_ref").unwrap().f64().unwrap() - want.log2()).abs() < 1e-9);
    assert_eq!(r.get("max_file").unwrap().str(), Some("t5"));
    assert!(row(&doc, "uv").get("blank_rho").unwrap().is_null(), "two blanks are too few for a trend");
}

#[test]
fn the_choices_say_what_was_defaulted_and_where_the_times_came_from() {
    let doc = run(&names(), &rows(), None, &opts());
    let ch = doc.get("choices").unwrap().arr();
    let find = |k: &str| ch.iter().find(|c| c.get("key").unwrap().str() == Some(k));
    assert_eq!(find("factor").unwrap().get("value").unwrap().f64(), Some(3.0));
    assert!(find("pseudocount").is_some() && find("class_rules").is_some() && find("floor").is_some() && find("rho_min").is_some());
    let t = find("time:t3").unwrap();
    assert_eq!(t.get("reason").unwrap().str(), Some("name"));
    assert!(t.get("value").unwrap().f64().is_some());
}

#[test]
fn a_given_pseudocount_and_a_looser_factor_are_not_defaults() {
    let o = CmpOpts { pseudo: Some(100.0), factor: Some(1.5), ..opts() };
    let doc = run(&names(), &rows(), None, &o);
    assert_eq!(doc.get("params").unwrap().get("pseudocount_origin").unwrap().str(), Some("given"));
    let ch = doc.get("choices").unwrap().arr();
    assert!(!ch.iter().any(|c| c.get("key").unwrap().str() == Some("factor")));
    // with a pseudocount of 100 the small row has a signal and a ratio 1.83 against t0, above the factor 1.5: a candidate
    assert_eq!(class(&doc, "small").0, "candidate");
}

#[test]
fn a_blank_that_rises_with_time_marks_an_artefact_even_when_the_treated_signal_is_far_above_it() {
    let meta: Vec<Option<CmpCol>> = [(0.0, "t0"), (1.0, "sample"), (2.0, "sample"), (3.0, "sample"), (4.0, "sample"), (5.0, "sample"), (0.0, "blank"), (5.0, "blank")]
        .iter()
        .map(|(t, r)| Some(CmpCol { name: String::new(), role: Some(r.to_string()), time: Some(*t) }))
        .collect();
    let mut names = names();
    names.push("MQ_t3".into());
    let mut meta = meta;
    meta.push(Some(CmpCol { name: String::new(), role: Some("blank".into()), time: Some(3.0) }));
    // the filler rows keep the median of the blanks (and so the pseudocount) low, as the many background features of a real table do
    let mut matrix = vec![vec![0.0, 1e5, 2e5, 4e5, 6e5, 9e5, 2e3, 9e3, 5e3]];
    let mut ids = vec!["irradiated".to_string()];
    for k in 0..5 {
        matrix.push(vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1e3, 1e3, 1e3]);
        ids.push(format!("filler{k}"));
    }
    let (mz, rt) = (vec![200.0; 6], vec![5.0; 6]);
    let inp = CmpIn { path: "t.csv", sha256: "00", ids: &ids, mz: &mz, rt: &rt, matrix: &matrix, names: &names, meta: Some(&meta) };
    let o = CmpOpts { ref_sel: "role:reference".into(), blank_sel: "role:blank".into(), ..CmpOpts::default() };
    let doc = compare(&inp, &o).ok().unwrap();
    let r = &doc.get("rows").unwrap().arr()[0];
    assert_eq!(r.get("blank_rho").unwrap().f64(), Some(1.0));
    assert_eq!(r.get("class").unwrap().str(), Some("blank_artefact"));
    assert_eq!(r.get("reasons").unwrap().arr()[0].str(), Some("blank_rises"));
    let t = doc.get("choices").unwrap().arr().iter().find(|c| c.get("key").unwrap().str() == Some("time:MQ_t3")).unwrap();
    assert_eq!(t.get("reason").unwrap().str(), Some("samples"));
}

fn fails(o: &CmpOpts) -> (Option<&'static str>, String) {
    let (ids, names) = (vec!["a".to_string()], names());
    let m = vec![vec![1.0; 8]];
    let inp = CmpIn { path: "t.csv", sha256: "00", ids: &ids, mz: &[1.0], rt: &[1.0], matrix: &m, names: &names, meta: None };
    let e = compare(&inp, o).err().expect("must fail");
    (e.key, e.text)
}

#[test]
fn selectors_that_match_nothing_or_overlap_or_need_samples_are_refused() {
    let (k, t) = fails(&CmpOpts { ref_sel: "zz*".into(), ..opts() });
    assert_eq!(k, Some("err.compare.selector"));
    assert!(t.contains("t0, t1") && t.contains("MQ_t5"), "the hint lists the columns: {t}");
    assert_eq!(fails(&CmpOpts { blank_sel: "t0,MQ*".into(), ..opts() }).0, Some("err.compare.overlap"));
    assert_eq!(fails(&CmpOpts { ref_sel: "role:reference".into(), ..opts() }).0, Some("err.compare.samples"));
    assert_eq!(fails(&CmpOpts { ref_sel: "T0".into(), quantity: Some("peak".into()), ..opts() }).0, Some("err.compare.param"));
}

#[test]
fn the_plan_lists_the_roles_and_computes_nothing() {
    let (ids, names) = (vec!["a".to_string()], names());
    let m = vec![vec![1.0; 8]];
    let inp = CmpIn { path: "t.csv", sha256: "00", ids: &ids, mz: &[1.0], rt: &[1.0], matrix: &m, names: &names, meta: None };
    let doc = plan(&inp, &opts()).ok().unwrap();
    assert_eq!(doc.get("schema").unwrap().str(), Some("compare-plan/1"));
    assert!(doc.get("rows").is_none() && doc.get("counts").is_none());
    assert_eq!(doc.get("roles").unwrap().get("treated").unwrap().get("columns").unwrap().arr().len(), 5);
    assert_eq!(doc.get("params").unwrap().get("pseudocount").unwrap().str(), Some("auto"));
}
