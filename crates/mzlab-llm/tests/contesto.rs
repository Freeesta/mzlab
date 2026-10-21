//! `contesto/1` on synthetic sources (12 files and a table of 800 rows within 8000 estimated tokens, same bytes, golden file), the schema validator
//! of the tool arguments and the compaction of a long conversation. `MZLAB_UPDATE_LLM=1 cargo test -p mzlab-llm` rewrites the golden file.

use std::path::PathBuf;

use mzlab_core::json::{parse, Out, Value};
use mzlab_llm::agent::{compact, estimate};
use mzlab_llm::context::{build, table_summary, TIC_POINTS, TOP_ROWS};
use mzlab_llm::conv::{Conversation, Message, Part, Role};
use mzlab_llm::schema::validate;

fn golden() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/contesto_12_file.json")
}

fn j(text: &str) -> Value {
    parse(text.as_bytes()).unwrap()
}

/// 12 synthetic files with a 600-point TIC each and a table of 800 rows.
fn sources(hide: bool) -> Value {
    let files: Vec<String> = (0..12)
        .map(|k| {
            let rt: Vec<String> = (0..600).map(|i| format!("{:.3}", i as f64 * 0.05 + k as f64 * 0.001)).collect();
            let y: Vec<String> = (0..600).map(|i| format!("{}", (i * 37 + k * 11) % 5003 + 1000)).collect();
            // only the first file brings its TIC: twelve of them would not fit and the budget would drop them all
            let tic = if k == 0 { format!(",\"tic\":{{\"rt\":[{}],\"i\":[{}]}}", rt.join(","), y.join(",")) } else { String::new() };
            format!("{{\"name\":\"sample_{k}.mzML\",\"instrument\":\"synthetic\",\"resolution\":\"unit\",\"polarity\":1,\"ms_levels\":[1],\"scans\":600,\"rt_min\":[0,30],\"mode\":\"centroid\"{tic}}}")
        })
        .collect();
    let mut csv = String::from("id,mz,rt,area,label\n");
    for i in 0..800 {
        csv += &format!(
            "f{i},{:.4},{:.3},{},x{}\n",
            100.0 + i as f64 * 0.37,
            1.0 + i as f64 * 0.011,
            (i * 7919) % 100003,
            i % 5
        );
    }
    let mut q = Out::default();
    q.str(&csv);
    let quoted = q.0;
    let text = format!(
        "{{\"profile\":\"ricerca\",\"hide_names\":{hide},\"files\":[{}],\"features_csv\":{}}}",
        files.join(","),
        quoted
    );
    j(&text)
}

#[test]
fn twelve_files_and_a_big_table_fit_and_the_bytes_are_stable() {
    let a = build(&sources(false), 8000).unwrap();
    assert_eq!(a, build(&sources(false), 8000).unwrap());
    assert!((a.len() as u64).div_ceil(3) <= 8000, "{} bytes", a.len());
    let v = j(&a);
    let files = v.get("files").unwrap().arr();
    assert_eq!(files.len(), 12);
    assert!(
        v.get("truncated").is_none_or(|t| t.arr().is_empty()),
        "nothing is cut at 8000"
    );
    assert_eq!(
        v.get("features")
            .and_then(|f| f.get("top"))
            .map(|t| t.arr().len()),
        Some(TOP_ROWS)
    );
    assert!(files.iter().all(|f| f
        .get("tic")
        .is_none_or(|t| t.get("rt").unwrap().arr().len() <= TIC_POINTS)));
    let got = a + "\n";
    if std::env::var("MZLAB_UPDATE_LLM").is_ok() {
        std::fs::write(golden(), &got).unwrap();
    }
    assert_eq!(
        got,
        std::fs::read_to_string(golden()).unwrap(),
        "MZLAB_UPDATE_LLM=1 rewrites it"
    );
}

#[test]
fn names_are_hidden_on_request_and_a_small_budget_says_what_it_cut() {
    let hidden = build(&sources(true), 8000).unwrap();
    assert!(!hidden.contains("sample_") && hidden.contains("\"f12\""));
    let small = j(&build(&sources(false), 1500).unwrap());
    assert!(!small.get("truncated").unwrap().arr().is_empty());
    assert!(build(&sources(false), 10).is_err());
    let course = j("{\"profile\":\"corso\"}");
    assert_eq!(
        build(&course, 8000).unwrap_err().key,
        "err.llm.profile.course"
    );
}

#[test]
fn a_table_summary_has_counts_ranges_and_the_top_rows_by_area() {
    let mut csv = String::from("id,area\n");
    for i in 0..120 {
        csv += &format!("r{i},{}\n", i);
    }
    let s = table_summary(&csv);
    assert_eq!(s.get("rows").and_then(Value::f64), Some(120.0));
    let top = s.get("top").map_or(&[][..], Value::arr);
    assert_eq!(top.len(), TOP_ROWS);
}

#[test]
fn the_schema_validator_checks_types_bounds_and_unknown_keys() {
    let s = j("{\"type\":\"object\",\"additionalProperties\":false,\"required\":[\"n\"],\"properties\":{\"n\":{\"type\":\"integer\",\"minimum\":1,\"maximum\":5},\"k\":{\"enum\":[\"a\",\"b\"]}}}");
    assert!(validate(&s, &j("{\"n\":3,\"k\":\"a\"}")).is_ok());
    for bad in [
        "{}",
        "{\"n\":0}",
        "{\"n\":6}",
        "{\"n\":1.5}",
        "{\"n\":1,\"k\":\"c\"}",
        "{\"n\":1,\"z\":1}",
        "[]",
    ] {
        assert!(validate(&s, &j(bad)).is_err(), "{bad}");
    }
    let one = j("{\"oneOf\":[{\"const\":\"x\"},{\"type\":\"string\"}]}");
    assert!(validate(&one, &j("\"y\"")).is_ok());
    assert!(validate(&one, &j("\"x\"")).is_err(), "matches both");
}

fn result(i: usize, size: usize) -> Message {
    let id = format!("t{i}");
    Message {
        role: Role::Tool,
        parts: vec![Part::ToolResult {
            id,
            ok: true,
            content: Value::Str("x".repeat(size)),
        }],
    }
}

#[test]
fn compaction_replaces_the_oldest_results_and_keeps_the_last_plan() {
    let mut c = Conversation {
        provider: "p".into(),
        model: "m".into(),
        system: "s".into(),
        max_output_tokens: None,
        messages: Vec::new(),
    };
    for i in 0..10 {
        c.messages.push(Message {
            role: Role::Assistant,
            parts: vec![Part::ToolCall {
                id: format!("t{i}"),
                name: if i == 9 {
                    "flow_get_plan"
                } else {
                    "flow_status"
                }
                .into(),
                args: j("{}"),
            }],
        });
        c.messages.push(result(i, 3000));
    }
    let before = estimate(&c, &[]);
    assert!(before > 9000);
    let n = compact(&mut c, &[], 10_000);
    assert!(n > 0 && estimate(&c, &[]) < before);
    let last = c.messages.last().unwrap();
    assert!(
        matches!(&last.parts[0], Part::ToolResult { content: Value::Str(s), .. } if s.len() == 3000),
        "the latest plan stays whole"
    );
    assert_eq!(
        compact(&mut c, &[], 1_000_000),
        0,
        "below 70% nothing changes"
    );
}
