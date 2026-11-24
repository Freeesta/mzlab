//! `curve_text` on invented responses: an exact line back-calculates its standards and flags the unknowns out of range, the internal standard
//! cancels a varying injection, the user's exclusions, the undefined weights, the skipped rows, edge peaks, S/N ≤ 0 and other units are logged,
//! LOD/LOQ follow their two formulae and the ICH ones match the numbers of Miller & Miller, bad files fail.

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
pub(super) fn exact() -> (String, String) {
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

pub(super) fn run(params: &[(&str, &str)], resp: &str, stds: &str) -> (Value, String, Vec<String>) {
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

pub(super) fn line<'a>(csv: &'a str, start: &str) -> &'a str {
    csv.lines()
        .find(|l| l.starts_with(start))
        .unwrap_or_else(|| panic!("{start}: {csv}"))
}

/// The `note` of a row: the third field from the end (`note,state,sn`).
pub(super) fn note_of(line: &str) -> &str {
    line.rsplit(',').nth(2).unwrap_or("")
}

#[test]
fn an_exact_line_back_calculates_its_standards_and_flags_the_unknowns() {
    let (r, s) = exact();
    let (doc, csv, warn) = run(&[], &r, &s);
    assert_eq!(
        doc.get("format").and_then(Value::str),
        Some("calibration/1")
    );
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
    assert_eq!(note_of(line(&csv, "A,u2,")), "above_range");
    assert_eq!(note_of(line(&csv, "A,b,blank,")), "below_range");
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
    assert_eq!(note_of(line(&csv, "A,u2,")), "no_istd");
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
    assert_eq!(note_of(line(&csv, "A,s3,")), "excluded_by_user");
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
    // unweighted, the zero standard is a point of the line but the range starts at the lowest standard above zero (the LLOQ), as the log says
    let r = format!("{r}u3,A,125,,\n");
    let s = format!("{s}u3;;;;sample\n");
    let (doc, csv, _) = run(&[("weight", "1")], &r, &s);
    let c = doc.get("curves").unwrap().at(0).unwrap();
    assert_eq!(num(c, "n"), 6.0);
    assert_eq!(
        c.get("range").unwrap().at(0).and_then(Value::f64),
        Some(1.0)
    );
    assert_eq!(note_of(line(&csv, "A,u3,")), "below_range");
    assert!(log_has(&doc, "range_lower", "lowest_nonzero_standard", "1"));
}

#[test]
fn the_ich_limits_of_the_miller_and_miller_line() {
    // Miller & Miller 6th ed. §5.4 (fluorescein, pg/mL), unweighted: b = 216.2/112 = 1.930357… and s_y/x = 0.432847713… (the book prints 1.93 and
    // 0.4329, so 3.3·0.4329/1.93 = 0.7402 and 10·0.4329/1.93 = 2.243 from its rounded numbers); exact: LOD 0.7399653784159…, LOQ 2.2423193285331…
    let x = [0, 2, 4, 6, 8, 10, 12];
    let y = ["2.1", "5.0", "9.0", "12.6", "17.3", "21.0", "24.7"];
    let mut resp = String::from("sample,name,area\n");
    let mut stds = String::from("sample;concentration;unit;role\n");
    for (x, y) in x.iter().zip(y) {
        resp += &format!("c{x},F,{y}\n");
        stds += &format!("c{x};{x};pg/mL;standard\n");
    }
    let (doc, _, _) = run(&[("weight", "1")], &resp, &stds);
    let c = doc.get("curves").unwrap().at(0).unwrap();
    let value = |k: &str| num(c.get(k).unwrap().at(0).unwrap(), "value");
    let exact = |got: f64, want: f64| assert!((got / want - 1.0).abs() < 1e-12, "{got} != {want}");
    exact(value("lod"), 0.739_965_378_415_927_4);
    exact(value("loq"), 2.242_319_328_533_113_3);
    // the book's own limit of detection, y_B + 3·s_B with s_B = s_y/x, is 0.67 pg/mL: the same σ and slope
    assert!((value("lod") * 3.0 / 3.3 - 0.67).abs() < 0.005);
}

/// The ICH value of a limit (the first of the pair) is `null`.
fn ich_is_null(c: &Value, key: &str) -> bool {
    let first = c.get(key).and_then(|v| v.at(0));
    first
        .and_then(|v| v.get("value"))
        .is_some_and(Value::is_null)
}

fn log_has(doc: &Value, choice: &str, reason: &str, note: &str) -> bool {
    doc.get("log").unwrap().arr().iter().any(|l| {
        l.get("choice").and_then(Value::str) == Some(choice)
            && l.get("reason").and_then(Value::str) == Some(reason)
            && l.get("note").and_then(Value::str) == Some(note)
    })
}

#[test]
fn a_response_not_above_zero_stays_in_the_fit_but_is_logged_and_warned() {
    // blank-subtracted areas: 50·x − 60 is −10 at the lowest level
    let mut resp = String::from("sample,name,area\n");
    let mut stds = String::from("sample,concentration,role\n");
    for (i, (x, _)) in LEVELS.iter().enumerate() {
        resp += &format!("s{i},A,{}\n", 50.0 * x - 60.0);
        stds += &format!("s{i},{x},standard\n");
    }
    for w in ["1", "1/x", "1/x2"] {
        let (doc, _, warn) = run(&[("weight", w)], &resp, &stds);
        let c = doc.get("curves").unwrap().at(0).unwrap();
        assert_eq!(num(c, "n"), 5.0, "{w}");
        assert!(log_has(&doc, "kept", "response_not_positive", "-10"), "{w}");
        assert!(warn.contains(&"warn.flow.curve.notPositive".to_string()));
    }
    // under 1/y the weight is undefined: left out, and said once
    let (doc, _, warn) = run(&[("weight", "1/y")], &resp, &stds);
    assert_eq!(num(doc.get("curves").unwrap().at(0).unwrap(), "n"), 4.0);
    assert!(log_has(&doc, "excluded", "weight_undefined", ""));
    assert!(!warn.contains(&"warn.flow.curve.notPositive".to_string()));
}

#[test]
fn an_ich_limit_that_cannot_be_computed_says_why() {
    // two levels, two parameters: no residual degree of freedom
    let resp = "sample,name,area\ns1,A,150\ns2,A,200\n";
    let stds = "sample,concentration,role\ns1,1,standard\ns2,2,standard\n";
    let (doc, _, _) = run(&[("weight", "1")], resp, stds);
    let c = doc.get("curves").unwrap().at(0).unwrap();
    assert!(c.get("s_yx").unwrap().is_null());
    assert!(ich_is_null(c, "lod"));
    assert!(log_has(
        &doc,
        "limit_not_computed",
        "no_degrees_of_freedom",
        "ich"
    ));
    // a response that falls with the concentration
    let resp = "sample,name,area\ns1,A,300\ns2,A,200\ns3,A,90\n";
    let stds = format!("{stds}s3,3,standard\n");
    let (doc, _, _) = run(&[("weight", "1")], resp, &stds);
    let c = doc.get("curves").unwrap().at(0).unwrap();
    assert!(num(c, "b") < 0.0);
    assert!(ich_is_null(c, "loq"));
    assert!(doc
        .get("log")
        .unwrap()
        .arr()
        .iter()
        .any(|l| l.get("reason").and_then(Value::str) == Some("slope_not_positive")));
}

#[test]
fn the_accuracy_is_the_same_in_the_points_and_in_the_csv_whatever_the_dilution() {
    // s3 and the QC are diluted twice: the concentration doubles, the accuracy compares the injected solution with its nominal value
    let (r, s) = exact();
    let s = s.replace(
        "concentration;unit;role\n",
        "concentration;unit;role;dilution\n",
    );
    let s = s.replace("s3;L3;5;ng/mL;standard", "s3;L3;5;ng/mL;standard;2");
    let r = format!("{r}q,A,450,,\n");
    let s = format!("{s}q;QC;7;ng/mL;qc;2\n");
    let (doc, csv, _) = run(&[], &r, &s);
    let c = doc.get("curves").unwrap().at(0).unwrap();
    let s3 = c.get("points").unwrap().at(2).unwrap();
    let row: Vec<&str> = line(&csv, "A,s3,").split(',').collect();
    close(row[7].parse().unwrap(), 10.0);
    close(row[9].parse().unwrap(), 100.0);
    assert_eq!(row[9].parse::<f64>().unwrap(), num(s3, "accuracy_pct"));
    let qc: Vec<&str> = line(&csv, "A,q,").split(',').collect();
    close(qc[7].parse().unwrap(), 14.0);
    close(qc[9].parse().unwrap(), 100.0);
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
    assert_eq!(note_of(line(&csv, "A,u,")), "no_curve");
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
    assert!(log_has(&doc, "skipped", "sum_row", "line 5"), "{doc:?}");
    // chrom.transitions peaks: a row without a peak has no response
    let resp = "sample,name,isomer_group,q1,q3,polarity,rt,rt_window,apex_rt,area,height,snr,fwhm_s,peak,baseline,below\n\
        s1,A,,200,100,+,1,0.5,1,150,10,8,3,ok,zero,0\ns2,A,,200,100,+,1,0.5,1,250,10,8,3,edge,zero,0\n\
        s3,A,,200,100,+,1,0.5,1,450,10,8,3,ok,zero,0\nu,A,,200,100,+,1,0.5,1,90,10,1,3,no_peak,zero,0\n";
    let (doc, csv, warn) = run(&[("weight", "1")], resp, &format!("{stds}u;;sample\n"));
    assert_eq!(note_of(line(&csv, "A,u,")), "no_response");
    // the peak cut at the window edge stays in the fit, never unseen; the row without a peak says which status it had
    assert!(log_has(&doc, "kept", "peak_at_window_edge", "edge"));
    assert!(log_has(
        &doc,
        "treated_as_no_response",
        "peak_status",
        "no_peak"
    ));
    assert!(
        warn.contains(&"warn.flow.curve.edge".to_string()),
        "{warn:?}"
    );
    let pts = doc
        .get("curves")
        .unwrap()
        .at(0)
        .unwrap()
        .get("points")
        .unwrap();
    assert_eq!(
        pts.at(1).unwrap().get("used").and_then(Value::bool),
        Some(true)
    );
    // the user leaves it out with `exclude`, as any other point
    let (doc, _, _) = run(&[("weight", "1"), ("exclude", "A|s2|edge")], resp, stds);
    assert!(log_has(&doc, "excluded", "excluded_by_user", "edge"));
}

#[test]
fn rows_without_a_sample_an_snr_not_above_zero_and_a_qc_in_another_unit_are_logged() {
    let (r, s) = exact();
    // a row without a sample; the S/N of the lowest standard is 0, so the S/N limits come from the next level (S/N 12 at 2 ng/mL)
    let r = r.replace("s1,A,150,10,6\n", "s1,A,150,10,0\n") + ",A,1,,\n";
    let s = s + "q1;QC;10;ug/L;qc\n";
    let r = r + "q1,A,600,,\n";
    let (doc, csv, warn) = run(&[], &r, &s);
    assert!(log_has(&doc, "skipped", "no_sample", "line 10"), "{doc:?}");
    assert!(log_has(&doc, "snr_not_used", "snr_not_positive", "0"));
    let c = doc.get("curves").unwrap().at(0).unwrap();
    close(
        num(c.get("lod").unwrap().at(1).unwrap(), "value"),
        3.0 * 2.0 / 12.0,
    );
    // the QC is reported in the unit of the curve, without an accuracy that would compare two units
    assert!(log_has(
        &doc,
        "reported_in_curve_unit",
        "unit_differs",
        "ug/L -> ng/mL"
    ));
    let q1: Vec<&str> = line(&csv, "A,q1,").split(',').collect();
    close(q1[7].parse().unwrap(), 10.0);
    assert_eq!((q1[8], q1[9]), ("ng/mL", ""));
    assert!(
        warn.contains(&"warn.flow.curve.unit".to_string()),
        "{warn:?}"
    );
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
    assert_eq!(fails(&[("zero", "yes")], &r, &s), "err.flow.curve.param");
    let num_zero = vec![("zero".to_string(), Value::Num(1.0))];
    assert_eq!(
        curve_text(&num_zero, r.as_bytes(), s.as_bytes())
            .unwrap_err()
            .key,
        "err.flow.curve.param"
    );
    let bool_zero = vec![("zero".to_string(), Value::Bool(true))];
    assert!(curve_text(&bool_zero, r.as_bytes(), s.as_bytes()).is_ok());
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
