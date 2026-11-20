//! The five chains an agent runs with `mzlab-cli`, each on synthetic data: every call must exit 0 and the chain must take exactly the number of
//! calls the docs promise (`for-agents.md`), so a chain cannot grow a step or lose a route without this file saying so.

use std::path::{Path, PathBuf};

use mzlab_core::json::{parse, Value};

use crate::convert_fixtures::makers;
use crate::flow_run_tests::fixture;
use crate::flow_stats_tests::table_for;

/// A new empty folder of the workflow `tag` (one per test: the tests run in parallel).
fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_workflow_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn s(path: &Path) -> &str {
    path.to_str().unwrap()
}

/// One agent session: every `call` runs the binary's entry point, requires exit 0 and is counted.
struct Chain {
    calls: usize,
}

impl Chain {
    fn new() -> Self {
        Chain { calls: 0 }
    }

    fn call(&mut self, args: &[&str]) -> crate::Outcome {
        self.calls += 1;
        let a: Vec<String> = args.iter().map(|x| x.to_string()).collect();
        let o = crate::execute(&a);
        assert_eq!(o.code, 0, "call {} {args:?}: {}", self.calls, o.stderr);
        o
    }

    /// The call's stdout as one JSON document that opens with its `schema` key.
    fn json(&mut self, args: &[&str], schema: &str) -> Value {
        let o = self.call(args);
        let text = String::from_utf8(o.stdout).unwrap();
        assert!(
            text.starts_with(&format!("{{\"schema\":\"{schema}\"")),
            "{args:?}: {text}"
        );
        parse(text.as_bytes()).unwrap()
    }

    fn done(self, calls: usize) {
        assert_eq!(self.calls, calls, "the chain takes {calls} calls");
    }
}

fn paths(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

/// The base chain of `fixture` plus a filter and a CSV export wired to a `samples.json` input.
const EXPORT_PLAN: &str = r#"{"format":"mzflow/2","inputs":[{"name":"s_t000.mzML","type":"raw"},{"name":"s_t010.mzML","type":"raw"},{"name":"s_t030.mzML","type":"raw"},{"name":"samples.json","type":"samples"}],"steps":[
{"id":"idx","step":"index.mzidx@1","in":{"source":["file:s_t000.mzML","file:s_t010.mzML","file:s_t030.mzML"]}},
{"id":"det","step":"features.detect@1","in":{"index":["idx.index"]}},
{"id":"ali","step":"features.align@1","in":{"peaks":["det.peaks"]}},
{"id":"gap","step":"features.gapfill@1","in":{"table":"ali.table","index":["idx.index"]}},
{"id":"flt","step":"table.filter@2","in":{"table":"gap.table","samples":"file:samples.json"}},
{"id":"exp","step":"export.table_csv@1","in":{"table":"flt.table","samples":"file:samples.json"}}]}"#;

/// 1. Unknown folder: list, inspect, propose parameters, check, run, export a CSV.
#[test]
fn inspect_params_check_run_export() {
    let (d, _) = fixture("wf_export");
    let data = d.join("data");
    let mut c = Chain::new();
    let list = c.json(&["file", "list", s(&data), "--json"], "folder/1");
    assert_eq!(list.get("items").unwrap().arr().len(), 3);
    let insp = c.call(&["llm", "inspect", s(&data)]);
    let insp_file = d.join("inspection.json");
    std::fs::write(&insp_file, &insp.stdout).unwrap();
    let params = c.json(
        &["llm", "params", "--inspection", s(&insp_file)],
        "param-proposal/1",
    );
    assert!(!params.get("choices").unwrap().arr().is_empty());
    let compact = c.json(&["flow", "steps", "--compact"], "steps-compact/1");
    assert!(compact.get("steps").is_some());
    let files = paths(&data);
    let mut table = vec!["sample", "table"];
    table.extend(files.iter().map(String::as_str));
    let samples = c.call(&table);
    std::fs::write(data.join("samples.json"), &samples.stdout).unwrap();
    let plan = d.join("export_plan.json");
    std::fs::write(&plan, EXPORT_PLAN).unwrap();
    let check = c.json(&["flow", "check", s(&plan), "--json"], "flowcheck/1");
    assert_eq!(check.get("valid").and_then(Value::bool), Some(true));
    let out = d.join("run");
    let run = c.json(
        &[
            "flow",
            "run",
            s(&plan),
            "--in",
            s(&data),
            "--out",
            s(&out),
            "--json",
        ],
        "flowrun/1",
    );
    assert_eq!(run.get("code").and_then(Value::f64), Some(0.0));
    let csv = std::fs::read_to_string(out.join("exp.csv")).unwrap();
    assert!(csv.lines().count() > 1, "the export has rows: {csv}");
    c.done(7);
    let _ = std::fs::remove_dir_all(&d);
}

/// 2. Reproducible run: roles, check, run, record, verify.
#[test]
fn run_record_verify() {
    let (d, _) = fixture("wf_record");
    let (plan, data, out) = (d.join("plan.json"), d.join("data"), d.join("out"));
    let mut c = Chain::new();
    let files = paths(&data);
    let mut table = vec!["sample", "table"];
    table.extend(files.iter().map(String::as_str));
    let roles = d.join("roles.json");
    std::fs::write(&roles, c.call(&table).stdout).unwrap();
    c.call(&["flow", "check", s(&plan)]);
    c.call(&["flow", "run", s(&plan), "--in", s(&data), "--out", s(&out)]);
    let record = out.join("run.mzrun.json");
    c.call(&[
        "flow",
        "record",
        s(&out),
        "--roles",
        s(&roles),
        "--out",
        s(&record),
    ]);
    let v = c.json(&["flow", "verify", s(&record), "--json"], "verify/1");
    assert_eq!(v.get("ok").and_then(Value::bool), Some(true));
    c.done(5);
    let _ = std::fs::remove_dir_all(&d);
}

/// A base64 text of little-endian f64 values (the binary arrays of an mzML chromatogram).
fn b64(v: &[f64]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let raw: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
    let mut o = String::new();
    for c in raw.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        for k in 0..4 {
            o.push(if k <= c.len() {
                A[(n >> (18 - 6 * k) & 63) as usize] as char
            } else {
                '='
            });
        }
    }
    o
}

/// An mzML with one transition of 120 points holding a Gaussian peak at 2.0 min.
fn mrm_file(dir: &Path) -> PathBuf {
    let rt: Vec<f64> = (0..120).map(|i| i as f64 * 0.0333).collect();
    let y: Vec<f64> = rt
        .iter()
        .map(|t| 100.0 + 9000.0 * (-((t - 2.0) / 0.08).powi(2) / 2.0).exp())
        .collect();
    let arr = |acc: &str, v: &[f64]| {
        format!("<binaryDataArray encodedLength=\"1\"><cvParam accession=\"MS:1000523\" name=\"64-bit float\"/><cvParam accession=\"{acc}\" name=\"x\"/><binary>{}</binary></binaryDataArray>", b64(v))
    };
    let xml = format!(
        "<mzML><run><chromatogramList count=\"1\"><chromatogram index=\"0\" id=\"SRM SIC Q1=364.1 Q3=194.1 CE=20 sample=1\" defaultArrayLength=\"120\"><cvParam accession=\"MS:1001473\" name=\"x\"/><binaryDataArrayList count=\"2\">{}{}</binaryDataArrayList></chromatogram></chromatogramList></run></mzML>",
        arr("MS:1000595", &rt),
        arr("MS:1000515", &y)
    );
    let f = dir.join("mrm.mzML");
    std::fs::write(&f, xml).unwrap();
    f
}

/// 3. Trace to peak: channels, the CSV of one transition, its peaks.
#[test]
fn trace_then_peak() {
    let d = dir("wf_peak");
    let f = mrm_file(&d);
    let mut c = Chain::new();
    let ch = c.json(&["file", "channels", s(&f)], "channels/1");
    // the agent takes the id from the listing, never from memory
    let id = ch
        .get("channels")
        .unwrap()
        .arr()
        .iter()
        .find(|x| x.get("family").and_then(Value::str) == Some("srm"))
        .and_then(|x| x.get("id").and_then(Value::str))
        .expect("an srm channel")
        .to_string();
    let csv = c.call(&["trace", "chrom", s(&f), "--id", &id, "--format", "csv"]);
    let trace = d.join("trace.csv");
    std::fs::write(&trace, &csv.stdout).unwrap();
    let peaks = c.call(&["peak", "auto-lr", s(&trace)]);
    let text = String::from_utf8(peaks.stdout).unwrap();
    assert!(text.contains("\"peaks\":[{"), "one peak at least: {text}");
    c.done(3);
    let _ = std::fs::remove_dir_all(&d);
}

const GROUPS: [&str; 7] = [
    "blank.mzML",
    "ref_1.mzML",
    "ref_2.mzML",
    "ref_3.mzML",
    "trt_1.mzML",
    "trt_2.mzML",
    "trt_3.mzML",
];

/// 4. Statistics: samples, design proposed, design checked, plan checked, plan run.
#[test]
fn samples_design_stats() {
    let d = dir("wf_stats");
    for n in GROUPS {
        std::fs::write(d.join(n), n.as_bytes()).unwrap();
    }
    std::fs::write(d.join("table.mzfeat"), table_for(&GROUPS)).unwrap();
    let files: Vec<String> = GROUPS.iter().map(|n| s(&d.join(n)).to_string()).collect();
    let mut c = Chain::new();
    let mut table = vec!["sample", "table"];
    table.extend(files.iter().map(String::as_str));
    let samples = d.join("samples.json");
    std::fs::write(&samples, c.call(&table).stdout).unwrap();
    let design = d.join("design.json");
    c.json(
        &[
            "sample",
            "design",
            "propose",
            "--samples",
            s(&samples),
            "--out",
            s(&design),
        ],
        "design_proposal/1",
    );
    c.json(
        &[
            "sample",
            "design",
            "validate",
            s(&design),
            "--samples",
            s(&samples),
        ],
        "design_check/1",
    );
    let plan = d.join("plan.json");
    std::fs::write(
        &plan,
        r#"{"format":"mzflow/2","inputs":[{"name":"table.mzfeat","type":"table"},{"name":"samples.json","type":"samples"},{"name":"design.json","type":"design"}],"steps":[
{"id":"st","step":"table.stats@1","in":{"table":"file:table.mzfeat","samples":"file:samples.json","design":"file:design.json"}}]}"#,
    )
    .unwrap();
    c.call(&["flow", "check", s(&plan)]);
    c.call(&[
        "flow",
        "run",
        s(&plan),
        "--in",
        s(&d),
        "--out",
        s(&d.join("run")),
    ]);
    assert!(d.join("run").join("prov.json").is_file());
    c.done(5);
    let _ = std::fs::remove_dir_all(&d);
}

/// 5. A folder of vendor files: list, what each holds, a dry run, conversion, validation of the mzML written.
#[test]
fn folder_triage_convert_validate() {
    let d = dir("wf_convert");
    let src = makers(&d, true);
    let (a, b) = (src.join("a.mzXML"), src.join("b.cdf"));
    let out = d.join("mzml");
    let mut c = Chain::new();
    // the folder listing shows the maker files the page would offer (`rotto.raw` is one; a.mzXML and b.cdf are named directly below)
    let list = c.json(&["file", "list", s(&src), "--json"], "folder/1");
    assert_eq!(list.get("items").unwrap().arr().len(), 1);
    c.json(&["sample", "list", s(&a), s(&b), "--json"], "sample-list/1");
    let dry = c.json(
        &[
            "file",
            "convert",
            s(&a),
            s(&b),
            "--out",
            s(&out),
            "--dry-run",
            "--json",
        ],
        "convert/1",
    );
    assert_eq!(dry.get("items").unwrap().arr().len(), 2);
    assert!(!out.join("a.mzML").exists(), "a dry run writes nothing");
    c.json(
        &["file", "convert", s(&a), s(&b), "--out", s(&out), "--json"],
        "convert/1",
    );
    let (ma, mb) = (out.join("a.mzML"), out.join("b.mzML"));
    let v = c.json(
        &["file", "validate", s(&ma), s(&mb), "--json"],
        "mzml-validation/1",
    );
    assert_eq!(v.get("files").unwrap().arr().len(), 2);
    c.done(5);
    let _ = std::fs::remove_dir_all(&d);
}
