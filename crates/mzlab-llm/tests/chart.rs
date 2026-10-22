//! `grafico/1`: the views of synthetic sources, a valid specification for every mark, the rejections (each with its `err.llm.chart.*` key), the reduction above
//! 5 000 rows (stable bytes) and the charts offered without a model.

use mzlab_core::json::{parse, Value};
use mzlab_llm::chart::{make, resolve, validate_text, MAX_ROWS};
use mzlab_llm::chart_views::{presets, views, views_json};

fn sources(files: usize, points: usize) -> Value {
    let f: Vec<String> = (0..files)
        .map(|k| {
            let rt: Vec<String> = (0..points)
                .map(|i| format!("{:.3}", i as f64 * 0.01))
                .collect();
            let y: Vec<String> = (0..points)
                .map(|i| format!("{}", (i * 37 + k * 11) % 5003 + 1000))
                .collect();
            let w: Vec<String> = (0..20)
                .map(|i| format!("{:.2}", 4.0 + (i % 5) as f64 * 0.5 + k as f64 * 0.1))
                .collect();
            format!(
                "{{\"name\":\"s{k}.mzML\",\"tic\":{{\"rt\":[{}],\"i\":[{}]}},\"widths\":[{}]}}",
                rt.join(","),
                y.join(","),
                w.join(",")
            )
        })
        .collect();
    let names: Vec<String> = (0..files).map(|k| format!("s{k}.mzML")).collect();
    let mut csv = format!("id,mz,rt,{}\n", names.join(","));
    for i in 0..40 {
        let a: Vec<String> = (0..files)
            .map(|k| ((i * 7919 + k * 13) % 100003).to_string())
            .collect();
        csv += &format!(
            "r{i},{:.4},{:.3},{}\n",
            100.0 + i as f64,
            1.0 + i as f64 * 0.1,
            a.join(",")
        );
    }
    let csv = {
        let mut o = mzlab_core::json::Out::default();
        o.str(&csv);
        o.0
    };
    let cv: Vec<String> = (0..40)
        .map(|i| format!("{{\"row\":\"r{i}\",\"cv_pct\":{}}}", 3 + i % 17))
        .collect();
    let drift: Vec<String> = (0..files)
        .map(|k| {
            format!(
                "{{\"order\":{k},\"file\":\"qc{}\",\"drift_min\":{}}}",
                k % 2,
                k as f64 * 0.01
            )
        })
        .collect();
    let blank: Vec<String> = (0..40)
        .map(|i| format!("{{\"row\":\"r{i}\",\"ratio\":{}}}", 1 + i % 9))
        .collect();
    parse(
        format!(
            "{{\"files\":[{}],\"features_csv\":{csv},\"qc\":{{\"cv\":[{}],\"drift\":[{}],\"blank\":[{}]}}}}",
            f.join(","),
            cv.join(","),
            drift.join(","),
            blank.join(",")
        )
        .as_bytes(),
    )
    .unwrap()
}

fn spec(r: &str, mark: &str, enc: &str) -> String {
    format!("{{\"schema\":\"grafico/1\",\"title\":\"T\",\"data\":{{\"ref\":\"{r}\"}},\"mark\":\"{mark}\",\"encoding\":{{{enc}}},\"why\":\"w\"}}")
}

fn key(s: &str, v: &[mzlab_llm::chart::View]) -> &'static str {
    validate_text(s, v).unwrap_err().key
}

#[test]
fn the_views_list_what_exists() {
    let v = views(&sources(3, 100));
    let ids: Vec<&str> = v.iter().map(|x| x.id.as_str()).collect();
    for want in [
        "tic:f1",
        "tic:f3",
        "tic:*",
        "features:area",
        "features:mzrt",
        "qc:cv",
        "drift:rt",
        "blank:ratio",
        "peaks:width",
    ] {
        assert!(ids.contains(&want), "{want} in {ids:?}");
    }
    assert!(!ids.contains(&"bpc:*"), "no base peak in these sources");
    assert!(views(&parse(b"{}").unwrap()).is_empty());
    let j = views_json(&v);
    assert!(j.contains("\"ref\":\"tic:*\"") && j.contains("\"unit\":\"min\""));
}

#[test]
fn every_mark_has_a_valid_specification() {
    let v = views(&sources(3, 100));
    for (r, mark, enc) in [
        ("tic:*", "line", "\"x\":{\"field\":\"rt\"},\"y\":{\"field\":\"intensity\"},\"color\":{\"field\":\"file\"}"),
        ("features:mzrt", "point", "\"x\":{\"field\":\"rt\"},\"y\":{\"field\":\"mz\"}"),
        ("features:area", "bar", "\"x\":{\"field\":\"file\"},\"y\":{\"field\":\"area\"}"),
        ("tic:f1", "stick", "\"x\":{\"field\":\"rt\"},\"y\":{\"field\":\"intensity\"}"),
        ("features:area", "box", "\"x\":{\"field\":\"file\"},\"y\":{\"field\":\"area\"}"),
        ("qc:cv", "hist", "\"x\":{\"field\":\"cv_pct\"}"),
        ("features:area", "heat", "\"x\":{\"field\":\"file\"},\"y\":{\"field\":\"row\"},\"color\":{\"field\":\"area\"}"),
    ] {
        let c = make(&spec(r, mark, enc), &v).unwrap_or_else(|e| panic!("{mark}: {}", e.key));
        let j = parse(c.as_bytes()).unwrap();
        assert_eq!(j.get("mark").and_then(Value::str), Some(mark));
        assert!(!j.get("data").unwrap().get("rows").unwrap().arr().is_empty());
    }
}

#[test]
fn invalid_specifications_come_back_with_their_reason() {
    let v = views(&sources(12, 100));
    let ok = "\"x\":{\"field\":\"rt\"},\"y\":{\"field\":\"intensity\"}";
    assert_eq!(key(&spec("nope:1", "line", ok), &v), "err.llm.chart.ref");
    assert_eq!(key(&spec("tic:*", "pie", ok), &v), "err.llm.chart.mark");
    assert_eq!(
        key(
            &spec(
                "tic:*",
                "line",
                "\"x\":{\"field\":\"nope\"},\"y\":{\"field\":\"intensity\"}"
            ),
            &v
        ),
        "err.llm.chart.field"
    );
    assert_eq!(
        key(
            &spec(
                "tic:*",
                "line",
                "\"x\":{\"field\":\"file\"},\"y\":{\"field\":\"intensity\"}"
            ),
            &v
        ),
        "err.llm.chart.type"
    );
    assert_eq!(key(&spec("tic:*", "box", ok), &v), "err.llm.chart.type");
    assert_eq!(
        key(
            &spec(
                "qc:cv",
                "hist",
                "\"x\":{\"field\":\"cv_pct\"},\"y\":{\"field\":\"cv_pct\"}"
            ),
            &v
        ),
        "err.llm.chart.type"
    );
    assert_eq!(
        key(
            &spec(
                "tic:*",
                "line",
                &format!("{ok},\"color\":{{\"field\":\"file\"}}")
            ),
            &v
        ),
        "err.llm.chart.color",
        "12 files, more than 8 colours"
    );
}

#[test]
fn colour_style_and_shape_are_refused() {
    let v = views(&sources(12, 100));
    let ok = "\"x\":{\"field\":\"rt\"},\"y\":{\"field\":\"intensity\"}";
    assert_eq!(
        key(
            &spec(
                "tic:*",
                "line",
                &format!("{ok},\"color\":{{\"field\":\"rt\"}}")
            ),
            &v
        ),
        "err.llm.chart.color"
    );
    assert_eq!(
        key(
            &spec(
                "tic:*",
                "line",
                &format!("{ok},\"color\":{{\"field\":\"file\",\"value\":\"red\"}}")
            ),
            &v
        ),
        "err.llm.chart.style"
    );
    let styled = spec("tic:*", "line", ok).replace("\"why\"", "\"palette\":[\"red\"],\"why\"");
    assert_eq!(key(&styled, &v), "err.llm.chart.style");
    let long = spec("tic:*", "line", ok).replace("\"T\"", &format!("\"{}\"", "x".repeat(61)));
    assert_eq!(key(&long, &v), "err.llm.chart.title");
    let rows = spec("tic:*", "line", ok)
        .replace("\"ref\":\"tic:*\"", "\"ref\":\"tic:*\",\"rows\":[[1,2]]");
    assert_eq!(key(&rows, &v), "err.llm.chart.rows");
    assert_eq!(
        key(
            &spec("tic:*", "line", ok).replace("grafico/1", "grafico/2"),
            &v
        ),
        "err.llm.chart.schema"
    );
    assert_eq!(key("not json", &v), "err.llm.chart.schema");
    let facet = spec("tic:*", "line", ok).replace("\"why\"", "\"facet\":\"rt\",\"why\"");
    assert_eq!(key(&facet, &v), "err.llm.chart.facet");
}

#[test]
fn no_view_is_an_answer_not_an_error() {
    let v = views(&sources(2, 50));
    let s = validate_text("{\"schema\":\"grafico/1\",\"error\":\"no_view\",\"why\":\"The concentration is not in any view.\"}", &v).unwrap();
    assert!(s.no_view);
    let out = resolve(&s, &v).unwrap();
    assert!(out.contains("\"error\":\"no_view\"") && out.contains("concentration"));
    assert!(validate_text(
        "{\"schema\":\"grafico/1\",\"error\":\"other\",\"why\":\"x\"}",
        &v
    )
    .is_err());
}

#[test]
fn a_long_view_is_reduced_to_5000_rows_with_the_same_bytes() {
    let v = views(&sources(12, 1400));
    let s = spec(
        "tic:*",
        "line",
        "\"x\":{\"field\":\"rt\"},\"y\":{\"field\":\"intensity\"},\"detail\":{\"field\":\"file\"}",
    );
    let a = make(&s, &v).unwrap();
    assert_eq!(a, make(&s, &views(&sources(12, 1400))).unwrap());
    let j = parse(a.as_bytes()).unwrap();
    let d = j.get("data").unwrap();
    assert!(d.get("rows").unwrap().arr().len() <= MAX_ROWS);
    assert_eq!(d.get("source_rows").and_then(Value::f64), Some(16800.0));
    assert_eq!(d.get("reduced").and_then(Value::bool), Some(true));
    // the peaks survive: the tallest point of each series is still there
    let top = d
        .get("rows")
        .unwrap()
        .arr()
        .iter()
        .filter_map(|r| r.at(1).and_then(Value::f64))
        .fold(0.0, f64::max);
    assert!(top >= 6000.0, "{top}");
}

#[test]
fn the_charts_without_a_model_are_built_by_rust() {
    let p = presets(&views(&sources(12, 300)));
    assert_eq!(p.len(), 5);
    for c in &p {
        let j = parse(c.as_bytes()).unwrap();
        assert!(!j.get("data").unwrap().get("rows").unwrap().arr().is_empty());
    }
    assert!(
        p[0].contains("\"detail\"") && !p[0].contains("\"color\""),
        "12 files: the series are the detail"
    );
    assert!(presets(&views(&parse(b"{}").unwrap())).is_empty());
}
