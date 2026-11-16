//! `curve_text` on invented responses: an exact line back-calculates its standards and flags the unknowns out of range, the internal standard
//! cancels a varying injection, the user's exclusions and the undefined weights are logged, LOD/LOQ follow their two formulae, bad files fail.

use super::*;
use mzlab_core::json::parse;

fn p(pairs: &[(&str, &str)]) -> Vec<(String, Value)> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), Value::Str(v.to_string())))
        .collect()
}

const LEVELS: [(f64, &str); 5] = [
    (1.0, "L1"),
    (2.0, "L2"),
    (5.0, "L3"),
    (10.0, "L4"),
    (20.0, "L5"),
];

/// Standards s1..s5 of A at the five levels, unknowns u1 (10), u2 (100), a blank b; area = 100 + 50·x.
fn exact() -> (String, String) {
    let mut resp = String::from("sample,name,area,height,snr\n");
    let mut stds = String::from("sample;level;concentration;unit;role\n");
    for (i, (x, l)) in LEVELS.iter().enumerate() {
        resp += &format!(
            "s{},A,{},{},{}\n",
            i + 1,
            100.0 + 50.0 * x,
            10.0 * x,
            6.0 * x
        );
        stds += &format!("s{};{l};{x};ng/mL;standard\n", i + 1);
    }
    resp += "u1,A,600,,\nu2,A,5100,,\nb,A,100,,\n";
    stds += "u1;;;;sample\nu2;;;;sample\nb;;;;blank\n";
    (resp, stds)
}

fn run(params: &[(&str, &str)], resp: &str, stds: &str) -> (Value, String, Vec<String>) {
    let (json, csv, warn) = curve_text(&p(params), resp.as_bytes(), stds.as_bytes()).unwrap();
    (parse(json.as_bytes()).unwrap(), csv, warn)
}

fn num(v: &Value, k: &str) -> f64 {
    v.get(k).and_then(Value::f64).unwrap_or(f64::NAN)
}

fn close(got: f64, want: f64) {
    assert!(
        (got - want).abs() <= 1e-9 * want.abs().max(1.0),
        "{got} != {want}"
    );
}

fn line<'a>(csv: &'a str, start: &str) -> &'a str {
    csv.lines()
        .find(|l| l.starts_with(start))
        .unwrap_or_else(|| panic!("{start}: {csv}"))
}

#[test]
fn an_exact_line_back_calculates_its_standards_and_flags_the_unknowns() {
    let (r, s) = exact();
    let (doc, csv, warn) = run(&[], &r, &s);
    assert_eq!(doc.get("format").and_then(Value::str), Some("taratura/1"));
    assert_eq!(doc.get("weight").and_then(Value::str), Some("1/x"));
    let c = doc.get("curves").unwrap().at(0).unwrap();
    close(num(c, "a"), 100.0);
    close(num(c, "b"), 50.0);
    close(num(c, "r2"), 1.0);
    assert_eq!(num(c, "n"), 5.0);
    for pt in c.get("points").unwrap().arr() {
        close(num(pt, "back"), num(pt, "x"));
        close(num(pt, "accuracy_pct"), 100.0);
    }
    assert!(csv.starts_with(CSV_HEAD));
    let u1: Vec<&str> = line(&csv, "A,u1,").split(',').collect();
    close(u1[7].parse().unwrap(), 10.0);
    assert_eq!(u1[10], "ok");
    assert!(line(&csv, "A,u2,").ends_with(",above_range"));
    assert!(line(&csv, "A,b,blank,").ends_with(",below_range"));
    assert_eq!(warn, vec!["warn.flow.curve.range"]);
    // the same inputs give the same bytes
    let a = curve_text(&[], r.as_bytes(), s.as_bytes()).unwrap();
    let b = curve_text(&[], r.as_bytes(), s.as_bytes()).unwrap();
    assert_eq!(a, b);
}

#[test]
fn lod_and_loq_follow_the_residual_and_the_signal_to_noise_conventions() {
    let (r, s) = exact();
    // a residual on the third standard so that s_y/x is not zero
    let r = r.replace("s3,A,350,", "s3,A,356,");
    let (doc, _, _) = run(&[("weight", "1")], &r, &s);
    let c = doc.get("curves").unwrap().at(0).unwrap();
    let (s_yx, b) = (num(c, "s_yx"), num(c, "b"));
    let lod = c.get("lod").unwrap();
    let loq = c.get("loq").unwrap();
    assert_eq!(
        lod.at(0).unwrap().get("method").and_then(Value::str),
        Some("ich_3.3_s_yx_over_slope")
    );
    close(num(lod.at(0).unwrap(), "value"), 3.3 * s_yx / b);
    close(num(loq.at(0).unwrap(), "value"), 10.0 * s_yx / b);
    // S/N of the lowest standard is 6 at 1 ng/mL: 3/6 and 10/6
    assert_eq!(
        lod.at(1).unwrap().get("method").and_then(Value::str),
        Some("sn_3_lowest_standard")
    );
    close(num(lod.at(1).unwrap(), "value"), 0.5);
    close(num(loq.at(1).unwrap(), "value"), 10.0 / 6.0);
}

#[test]
fn the_internal_standard_cancels_a_varying_injection() {
    // the analyte follows the injected volume v, so does the IS: the ratio is 2·x exactly
    let vols = [1.0, 0.5, 2.0, 0.8, 1.3];
    let mut resp = String::from("sample,analyte,area\n");
    let mut stds = String::from("sample,level,concentration,unit,role\n");
    for (i, ((x, l), v)) in LEVELS.iter().zip(vols).enumerate() {
        resp += &format!(
            "s{},A,{}\ns{},IS,{}\n",
            i + 1,
            2000.0 * x * v,
            i + 1,
            1000.0 * v
        );
        stds += &format!("s{},{l},{x},ug/L,std\n", i + 1);
    }
    resp += "u1,A,9000\nu1,IS,1500\nu2,A,5000\n";
    stds += "u1,,,,sample\nu2,,,,sample\n";
    let (doc, csv, warn) = run(&[("istd", "IS")], &resp, &stds);
    let curves = doc.get("curves").unwrap().arr();
    assert_eq!(curves.len(), 1, "the IS is not fitted");
    close(num(&curves[0], "b"), 2.0);
    close(num(&curves[0], "a"), 0.0);
    assert_eq!(curves[0].get("istd").and_then(Value::str), Some("IS"));
    // 9000 / 1500 = 6 → 3 ug/L
    let u1: Vec<&str> = line(&csv, "A,u1,").split(',').collect();
    assert_eq!((u1[4], u1[5], u1[6]), ("9000", "1500", "6"));
    close(u1[7].parse().unwrap(), 3.0);
    assert!(line(&csv, "A,u2,").ends_with(",no_istd"));
    assert!(warn.contains(&"warn.flow.curve.noResponse".to_string()));
    let log = doc.get("log").unwrap().arr();
    assert!(log
        .iter()
        .any(|l| l.get("reason").and_then(Value::str) == Some("internal_standard")));
    // without the IS the line has residuals
    let (doc, _, _) = run(&[], &resp, &stds);
    assert!(num(doc.get("curves").unwrap().at(0).unwrap(), "r2") < 0.99);
    // an IS that is not an analyte of the responses is refused
    assert!(curve_text(&p(&[("istd", "D5")]), resp.as_bytes(), stds.as_bytes()).is_err());
    let (doc, _, _) = run(&[("istd", "A|IS")], &resp, &stds);
    close(num(doc.get("curves").unwrap().at(0).unwrap(), "b"), 2.0);
}

#[test]
fn an_excluded_point_is_logged_with_the_users_reason_and_left_out_of_the_fit() {
    let (r, s) = exact();
    let r = r.replace("s3,A,350,", "s3,A,900,");
    let (doc, csv, warn) = run(&[("exclude", "A|s3|spiked vial"), ("weight", "1")], &r, &s);
    let c = doc.get("curves").unwrap().at(0).unwrap();
    close(num(c, "b"), 50.0);
    assert_eq!(num(c, "n"), 4.0);
    let s3 = c.get("points").unwrap().at(2).unwrap();
    assert_eq!(s3.get("used").and_then(Value::bool), Some(false));
    assert_eq!(
        s3.get("reason").and_then(Value::str),
        Some("excluded_by_user")
    );
    // still back-calculated, so the user sees how far it is
    close(num(s3, "back"), 16.0);
    let log = doc.get("log").unwrap().at(0).unwrap();
    assert_eq!(log.get("note").and_then(Value::str), Some("spiked vial"));
    assert!(line(&csv, "A,s3,").ends_with(",excluded_by_user"));
    assert!(warn.contains(&"warn.flow.curve.excluded".to_string()));
    // `*` excludes a sample for every analyte; a sample that does not exist is refused
    assert!(curve_text(&p(&[("exclude", "*|s3")]), r.as_bytes(), s.as_bytes()).is_ok());
    assert!(curve_text(&p(&[("exclude", "A|s9")]), r.as_bytes(), s.as_bytes()).is_err());
}

#[test]
fn a_zero_standard_has_no_weight_under_one_over_x_and_is_logged() {
    let (r, s) = exact();
    let r = format!("{r}s0,A,100,,\n");
    let s = format!("{s}s0;L0;0;ng/mL;standard\n");
    let (doc, _, warn) = run(&[], &r, &s);
    let c = doc.get("curves").unwrap().at(0).unwrap();
    assert_eq!(num(c, "n"), 5.0);
    let log = doc.get("log").unwrap().arr();
    assert!(log.iter().any(
        |l| l.get("reason").and_then(Value::str) == Some("weight_undefined")
            && l.get("sample").and_then(Value::str) == Some("s0")
    ));
    assert!(warn.contains(&"warn.flow.curve.excluded".to_string()));
    // unweighted, the zero standard is a point of the line
    let (doc, _, _) = run(&[("weight", "1")], &r, &s);
    let c = doc.get("curves").unwrap().at(0).unwrap();
    assert_eq!(num(c, "n"), 6.0);
    assert_eq!(
        c.get("range").unwrap().at(0).and_then(Value::f64),
        Some(0.0)
    );
}

#[test]
fn a_curve_without_enough_levels_reports_its_error_and_quantifies_nothing() {
    let resp = "sample,name,area\ns1,A,100\ns2,A,110\nu,A,50\n";
    let stds = "sample,concentration,role\ns1,1,standard\ns2,1,standard\nu,,sample\n";
    let (doc, csv, warn) = run(&[], resp, stds);
    let c = doc.get("curves").unwrap().at(0).unwrap();
    assert_eq!(
        c.get("error").and_then(Value::str),
        Some("err.flow.curve.singular")
    );
    assert!(c.get("a").unwrap().is_null());
    assert!(line(&csv, "A,u,").ends_with(",no_curve"));
    assert_eq!(warn, vec!["warn.flow.curve.failed"]);
}

#[test]
fn a_quadratic_through_zero_is_fitted_when_asked() {
    let mut resp = String::from("sample,name,area\n");
    let mut stds = String::from("sample,concentration,role\n");
    for (i, (x, _)) in LEVELS.iter().enumerate() {
        resp += &format!("s{i},A,{}\n", 40.0 * x - 0.5 * x * x);
        stds += &format!("s{i},{x},standard\n");
    }
    let (doc, _, _) = run(
        &[("model", "quadratic"), ("zero", "true"), ("weight", "1/x2")],
        &resp,
        &stds,
    );
    let c = doc.get("curves").unwrap().at(0).unwrap();
    assert_eq!(num(c, "a"), 0.0);
    close(num(c, "b"), 40.0);
    close(num(c, "c"), -0.5);
    assert_eq!(doc.get("zero").and_then(Value::bool), Some(true));
}

#[test]
fn the_manual_integrations_and_the_transitions_tables_are_read_as_they_are() {
    // manual integrations: the user's label names the analyte, `sum` rows are skipped; a decimal comma with `;`
    let resp = "channel;name;sample;start_min;end_min;apex_rt_min;area;height;snr;baseline;overlap;pv;label\n\
        200>100;T1;s1;1;2;1,5;1,5E+02;10;8;zero;;;A\n200>100;T1;s2;1;2;1,5;2,5E+02;20;9;zero;;;A\n\
        200>100;T1;s3;1;2;1,5;4,5E+02;30;9;zero;;;A\n200>100;T1;sum;;;;8,5E+02;;;;;;\n";
    let stds = "sample;concentration;role\ns1;1;standard\ns2;2;standard\ns3;4;standard\n";
    let (doc, _, _) = run(&[("weight", "1")], resp, stds);
    let c = doc.get("curves").unwrap().at(0).unwrap();
    assert_eq!(c.get("analyte").and_then(Value::str), Some("A"));
    close(num(c, "b"), 100.0);
    // chrom.transitions peaks: a row without a peak has no response
    let resp = "sample,name,isomer_group,q1,q3,polarity,rt,rt_window,apex_rt,area,height,snr,fwhm_s,peak,baseline,below\n\
        s1,A,,200,100,+,1,0.5,1,150,10,8,3,ok,zero,0\ns2,A,,200,100,+,1,0.5,1,250,10,8,3,edge,zero,0\n\
        s3,A,,200,100,+,1,0.5,1,450,10,8,3,ok,zero,0\nu,A,,200,100,+,1,0.5,1,90,10,1,3,no_peak,zero,0\n";
    let (_, csv, _) = run(&[("weight", "1")], resp, &format!("{stds}u;;sample\n"));
    assert!(line(&csv, "A,u,").ends_with(",no_response"));
}

#[test]
fn files_and_parameters_that_cannot_be_read_fail() {
    let (r, s) = exact();
    let fails = |params: &[(&str, &str)], r: &str, s: &str| {
        curve_text(&p(params), r.as_bytes(), s.as_bytes())
            .unwrap_err()
            .key
    };
    assert_eq!(
        fails(&[("weight", "1/x^2")], &r, &s),
        "err.flow.curve.param"
    );
    assert_eq!(fails(&[("model", "cubic")], &r, &s), "err.flow.curve.param");
    assert_eq!(
        fails(&[], &r, &s.replace("u1;;;;sample", "u1;;;;spike")),
        "err.flow.curve.role"
    );
    assert_eq!(
        fails(&[], &format!("{r}u1,A,1,,\n"), &s),
        "err.flow.curve.duplicate"
    );
    assert_eq!(
        fails(&[], &r, &s.replace("s2;L2;2;", "s2;L2;;")),
        "err.flow.curve.row"
    );
    assert_eq!(
        fails(&[], &r, &s.replace("s2;L2;2;ng/mL", "s2;L2;2;mg/L")),
        "err.flow.curve.unit"
    );
    assert_eq!(
        fails(&[], "sample,name\ns1,A\n", &s),
        "err.flow.curve.responses"
    );
    assert_eq!(fails(&[], &r, "sample;role\n"), "err.flow.curve.standards");
    assert_eq!(
        fails(&[], &r.replace("600", "6x0"), &s),
        "err.flow.curve.row"
    );
}
