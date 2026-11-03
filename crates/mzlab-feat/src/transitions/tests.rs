use super::*;
use mzlab_core::json::{parse as parse_json, Value};

/// One ion: Q1, polarity and its (apex RT, height) peaks.
type Ion = (f64, i8, &'static [(f64, f64)]);

/// The traces of `tools/wiff_sim.py` (sample 1 and 2: two isomers on 237.1, one peak on 253.1 and 180.1, a negative ion 235.1): one point every 1.2 s.
fn sim(sample: u32) -> Vec<Channel> {
    let peaks: &[Ion] = if sample == 1 {
        &[
            (237.1, 1, &[(2.5, 8.0e4), (4.0, 3.0e4)]),
            (253.1, 1, &[(1.5, 5.0e4)]),
            (180.1, 1, &[(5.0, 2.0e4)]),
            (235.1, -1, &[(3.0, 1.0e4)]),
        ]
    } else {
        &[
            (237.1, 1, &[(2.55, 6.0e4), (4.05, 4.5e4)]),
            (253.1, 1, &[(1.52, 2.0e4)]),
            (180.1, 1, &[(5.1, 1.0e4)]),
            (235.1, -1, &[(3.02, 7.0e3)]),
        ]
    };
    peaks
        .iter()
        .map(|&(q1, pol, p)| {
            let t: Vec<f64> = (0..300).map(|k| k as f64 * 1.2 / 60.0).collect();
            let y = t
                .iter()
                .map(|&x| {
                    let base = 50.0 + ((x * 1000.0 + q1) as i64 % 7) as f64;
                    base + p
                        .iter()
                        .map(|(rt, h)| h * (-0.5 * ((x - rt) / 0.05f64).powi(2)).exp())
                        .sum::<f64>()
                })
                .collect();
            Channel {
                q1,
                q3: None,
                polarity: Some(pol),
                t,
                y,
            }
        })
        .collect()
}

fn samples() -> Vec<(String, Vec<Channel>)> {
    vec![("s1.mzML".into(), sim(1)), ("s2.mzML".into(), sim(2))]
}

const TABLE: &str = "name;q1;q3;polarity;rt;rt_window;isomer_group\n\
isoA;237.1;;+;2.5;0.3;237\n\
isoB;237.1;;+;4.0;;237\n\
ionB;253.1;;+;1.5;;\n\
ionN;235.1;;-;3.0;;\n\
wrongPol;235.1;;+;3.0;;\n\
srm;237.1;194.1;+;2.5;;\n\
unknown;237.1;;+;;;237\n";

fn params(output: &str) -> Vec<(String, Value)> {
    vec![("output".into(), Value::Str(output.into()))]
}

/// The cells of the line of `name` (and `sample`) in a CSV text, by column name.
fn line<'a>(text: &'a str, sample: &str, name: &str) -> Vec<(&'a str, &'a str)> {
    let head: Vec<&str> = text.lines().next().unwrap().split(',').collect();
    let row = text
        .lines()
        .find(|l| {
            let c: Vec<&str> = l.split(',').collect();
            c.contains(&name) && (sample.is_empty() || c[0] == sample)
        })
        .unwrap_or_else(|| panic!("{name} in {text}"));
    head.into_iter().zip(row.split(',')).collect()
}

fn get<'a>(cells: &[(&str, &'a str)], col: &str) -> &'a str {
    cells.iter().find(|(h, _)| *h == col).map(|c| c.1).unwrap()
}

#[test]
fn q1_comes_from_formula_and_adduct_and_the_polarity_picks_the_adduct() {
    let rows = parse("name;formula;polarity\nparent;C15H12N2O;\nneg;C15H12N2O;-\n").unwrap();
    assert!((rows[0].q1 - 237.1022).abs() < 1e-3, "{}", rows[0].q1);
    assert_eq!((rows[0].polarity, rows[0].adduct.as_str()), (1, "[M+H]+"));
    assert!((rows[1].q1 - 235.0877).abs() < 1e-3, "{}", rows[1].q1);
    assert_eq!(rows[1].adduct, "[M-H]-");
    // a decimal comma when the fields are split by `;`
    assert_eq!(parse("name;q1;rt\na;237,1;2,5\n").unwrap()[0].rt, Some(2.5));
}

#[test]
fn every_wrong_row_gives_its_error() {
    let e = parse("name;q1;polarity;adduct;formula\na;;+;;\nb;237;x;;\nc;;-;[M+H]+;C6H6\nd;-3;;;\ne;;;;C6Q\nf;100;;;\nf;101;;;\n").unwrap_err();
    let keys: Vec<(usize, &str)> = e.iter().map(|r| (r.line, r.key)).collect();
    assert_eq!(
        keys,
        [
            (2, "error.liste.noMz"),
            (3, "error.liste.badPolarity"),
            (4, "error.liste.badAdduct"),
            (5, "error.liste.badMz"),
            (6, "error.liste.badFormula"),
            (8, "error.liste.twice"),
        ]
    );
    assert_eq!(
        parse("q1;name\n1;a\n").unwrap_err()[0].key,
        "error.liste.noHeader"
    );
    assert_eq!(
        parse("name;mass\na;1\n").unwrap_err()[0].key,
        "error.liste.badColumn"
    );
}

#[test]
fn the_example_table_reads_and_its_isobaric_rows_share_a_group() {
    let rows = parse(crate::steps::EXAMPLE_1).unwrap();
    assert_eq!(rows.len(), 14);
    assert!(rows.iter().all(|r| r.rt.is_none()
        && r.q3.is_none()
        && r.polarity == 1
        && !r.isomer_group.is_empty()));
    let same: Vec<&Row> = rows.iter().filter(|r| r.isomer_group == "253").collect();
    assert_eq!(same.len(), 4);
    assert!(
        same.iter().all(|r| (r.q1 - 253.0972).abs() < 1e-3),
        "{same:?}"
    );
    // the cells are kept as written: q1 stays empty in the CSV of the plan
    assert_eq!(to_csv(&rows), crate::steps::EXAMPLE_1);
}

#[test]
fn each_row_takes_its_peak_inside_its_window_and_says_why_it_has_none() {
    let text = channels_text(&params("peaks"), TABLE, &samples()).unwrap();
    assert!(text.starts_with("sample,name,isomer_group,q1,q3,polarity,rt,rt_window,apex_rt,area,height,snr,fwhm_s,peak\n"));
    assert_eq!(text.lines().count(), 1 + 2 * 7);
    for (sample, name, apex) in [
        ("s1.mzML", "isoA", 2.5),
        ("s1.mzML", "isoB", 4.0),
        ("s2.mzML", "isoA", 2.55),
        ("s2.mzML", "ionN", 3.02),
    ] {
        let c = line(&text, sample, name);
        assert_eq!(get(&c, "peak"), "ok", "{c:?}");
        let rt: f64 = get(&c, "apex_rt").parse().unwrap();
        assert!((rt - apex).abs() <= 0.021, "{name} {rt}");
    }
    let area = |s, n| get(&line(&text, s, n), "area").parse::<f64>().unwrap();
    // gaussian of height 8e4 and sigma 0.05 min: 8e4 * 0.05 * 60 * sqrt(2 pi) = 601 593 counts x s
    assert!(
        (area("s1.mzML", "isoA") / 601_593.0 - 1.0).abs() < 0.05,
        "{}",
        area("s1.mzML", "isoA")
    );
    assert!(area("s1.mzML", "isoA") > 2.0 * area("s1.mzML", "isoB"));
    assert_eq!(get(&line(&text, "s1.mzML", "wrongPol"), "peak"), "no_trace");
    assert_eq!(get(&line(&text, "s1.mzML", "srm"), "peak"), "no_trace");
    assert_eq!(get(&line(&text, "s1.mzML", "unknown"), "peak"), "no_rt");
    // the same bytes on every call
    assert_eq!(
        text,
        channels_text(&params("peaks"), TABLE, &samples()).unwrap()
    );
}

#[test]
fn a_window_without_a_peak_is_no_peak() {
    let t = "name;q1;rt;rt_window\nflat;180.1;1.0;0.2\n";
    let text = channels_text(&params("peaks"), t, &samples()).unwrap();
    assert_eq!(
        get(&line(&text, "s1.mzML", "flat"), "peak"),
        "no_peak",
        "{text}"
    );
    let wide = channels_text(&params("table"), t, &samples()).unwrap();
    assert!(wide.lines().nth(1).unwrap().ends_with(",,,,,0,,"), "{wide}");
}

#[test]
fn the_table_has_the_area_and_apex_of_each_file_and_their_spread() {
    let text = channels_text(&params("table"), TABLE, &samples()).unwrap();
    assert_eq!(
        text.lines().next().unwrap(),
        "name,isomer_group,q1,q3,polarity,rt,rt_window,area s1.mzML,area s2.mzML,apex_rt s1.mzML,apex_rt s2.mzML,n_peaks,rt_mean,rt_sd"
    );
    let c = line(&text, "", "isoA");
    assert_eq!(get(&c, "n_peaks"), "2");
    let mean: f64 = get(&c, "rt_mean").parse().unwrap();
    assert!((mean - 2.525).abs() < 0.02, "{mean}");
    assert!(!get(&c, "rt_sd").is_empty());
    assert_eq!(get(&line(&text, "", "unknown"), "n_peaks"), "0");
}

#[test]
fn the_rows_without_rt_get_the_peaks_of_their_channel_sorted_by_area() {
    let text = channels_text(&params("candidates"), TABLE, &samples()).unwrap();
    let rows: Vec<&str> = text.lines().skip(1).collect();
    // only the row without an RT; its two isomers, each found in both files, the larger first
    assert_eq!(rows.len(), 2, "{text}");
    let cell = |l: &str, k: usize| l.split(',').nth(k).unwrap().to_string();
    assert!(rows
        .iter()
        .all(|l| l.starts_with("unknown,237,237.1000,,+,,,")));
    assert_eq!(
        (cell(rows[0], 7), cell(rows[1], 7)),
        ("1".into(), "2".into())
    );
    let rt0: f64 = cell(rows[0], 8).parse().unwrap();
    let rt1: f64 = cell(rows[1], 8).parse().unwrap();
    assert!(
        (rt0 - 2.525).abs() < 0.03 && (rt1 - 4.025).abs() < 0.03,
        "{text}"
    );
    assert_eq!(
        (cell(rows[0], 11), cell(rows[1], 11)),
        ("2".into(), "2".into())
    );
}

#[test]
fn an_empty_table_or_files_without_channels_are_refused() {
    let e = channels_text(&params("peaks"), "name;q1\n", &samples()).unwrap_err();
    assert_eq!(e.key, "err.transitions.empty");
    let e = channels_text(&params("peaks"), TABLE, &[("a".into(), Vec::new())]).unwrap_err();
    assert_eq!(e.key, "err.transitions.noChannel");
    let e = channels_text(&params("peaks"), "name;q1\na;x\n", &samples()).unwrap_err();
    assert_eq!(e.key, "err.transitions.table");
}

#[test]
fn the_list_editor_keeps_the_order_and_gives_the_csv_of_the_plan() {
    let body = r#"{"kind":"transitions","rows":[{"name":"b","formula":"C6H6"},{"name":"a","q1":"100","rt":"2"}]}"#;
    let out = crate::liste::route(body).unwrap();
    let v = parse_json(out.as_bytes()).unwrap();
    let csv = v.get("csv").and_then(Value::str).unwrap();
    assert_eq!(csv, "name;q1;q3;polarity;rt;rt_window;isomer_group;note;formula;adduct\nb;;;;;;;;C6H6;\na;100;;;2;;;;;\n");
    assert_eq!(v.get("sha256").and_then(Value::str).unwrap().len(), 64);
    let mut req = mzlab_core::json::Out::default();
    req.raw("{\"kind\":\"transitions\",\"csv\":")
        .str(csv)
        .raw("}");
    let again = crate::liste::route(&req.0).unwrap();
    assert_eq!(again, out);
    let bad = crate::liste::route(r#"{"kind":"transitions","rows":[{"name":""}]}"#).unwrap();
    assert!(
        bad.contains("error.liste.noName") && bad.contains("\"line\":2"),
        "{bad}"
    );
}
