//! The plan checker, the canonical form and the execution order of `mzlab_core::flow`, against the real descriptors of `mzlab_feat::steps::registry`.

use std::sync::atomic::{AtomicU64, Ordering};

use mzlab_core::budget::sha256_hex;
use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{
    canonical, check, execute, parse, prov_json, Agent, Artifact, Job, Outcome, Platform, Runner,
};
use mzlab_feat::steps::registry;

/// The four `mzflow/2` blocks of the paper test, in the order of the file (A to D).
fn paper_blocks() -> Vec<String> {
    let text = include_str!("../../../docs/studi/workflow/PROVE_SU_CARTA.md");
    text.split("```json\n")
        .skip(1)
        .filter_map(|b| b.split("```").next())
        .map(str::to_string)
        .collect()
}

fn problems(plan: &str, platform: Platform) -> Vec<String> {
    let plan = parse(plan.as_bytes()).unwrap();
    check(&plan, &registry(), platform)
        .iter()
        .map(|p| {
            let params: Vec<String> = p.params.iter().map(|(k, v)| format!("{k}={v}")).collect();
            format!(
                "{}|{}|{}|{}",
                p.instance,
                p.port,
                p.error_key,
                params.join(",")
            )
        })
        .collect()
}

#[test]
fn paper_block_a_mzmine() {
    // spectra.centroid, features.resolve and features.isotopes are not steps; the tolerances of the sketch are Da + ppm pairs, which our steps do not take
    let want = [
        "imp|raw|err.flow.port.unknown|port=raw",
        "imp|files|err.flow.port.missing|port=files",
        "md1||err.flow.step.unknown|step=spectra.centroid",
        "md2||err.flow.step.unknown|step=spectra.centroid",
        "eic||err.flow.param.unknown|param=mz_tol",
        "eic||err.flow.param.unknown|param=rt_range",
        "res||err.flow.step.unknown|step=features.resolve",
        "iso||err.flow.step.unknown|step=features.isotopes",
        "aln||err.flow.param.unknown|param=mz_tol",
        "aln||err.flow.param.unknown|param=rt_tol_min",
        "aln||err.flow.param.unknown|param=weight_mz",
        "aln||err.flow.param.unknown|param=weight_rt",
        "gap||err.flow.param.unknown|param=rt_tol_min",
    ];
    assert_eq!(problems(&paper_blocks()[0], Platform::Native), want);
}

#[test]
fn paper_block_b_openms() {
    // the files of the sketch are not declared in `inputs`; features.align has no `rtmap` port
    let want = [
        "ffm||err.flow.param.unknown|param=noise_floor",
        "ffm||err.flow.param.unknown|param=mz_tol",
        "ffm||err.flow.param.unknown|param=min_trace_s",
        "ffm||err.flow.param.unknown|param=snr_min",
        "ffm||err.flow.param.unknown|param=fwhm_s",
        "ffm|index|err.flow.input.unknown|ref=a.mzML",
        "ffm|index|err.flow.input.unknown|ref=b.mzML",
        "iso||err.flow.step.unknown|step=features.isotopes",
        "rt||err.flow.step.unknown|step=series.rt_align",
        "link|rtmap|err.flow.port.unknown|port=rtmap",
    ];
    assert_eq!(problems(&paper_blocks()[1], Platform::Native), want);
}

#[test]
fn paper_block_c_xcms() {
    // centWave is a `planned` algorithm; `fill` has no `index` input
    let want = [
        "cw||err.flow.algorithm.unavailable|step=features.detect,algorithm=centwave",
        "cw||err.flow.param.unknown|param=noise",
        "cw||err.flow.param.unknown|param=prefilter",
        "cw|index|err.flow.input.unknown|ref=ko15.mzML",
        "cw|index|err.flow.input.unknown|ref=wt15.mzML",
        "mrg||err.flow.step.unknown|step=features.merge_neighbours",
        "rt||err.flow.step.unknown|step=series.rt_align",
        "grp||err.flow.param.unknown|param=bandwidth_s",
        "grp||err.flow.param.unknown|param=min_fraction",
        "grp|rtmap|err.flow.port.unknown|port=rtmap",
        "grp|samples|err.flow.port.unknown|port=samples",
        "fill|index|err.flow.port.missing|port=index",
    ];
    assert_eq!(problems(&paper_blocks()[2], Platform::Native), want);
}

#[test]
fn paper_block_d_patroon() {
    // `det` is not in the sketch; tp.generate and tp.link are not steps; features.ms2_link reads scans, not a table
    let want = [
        "fg|peaks|err.flow.ref.unknown|ref=det.peaks",
        "flt||err.flow.param.unknown|param=abs_min_intensity",
        "flt|samples|err.flow.input.unknown|ref=samples.json",
        "ms2|table|err.flow.port.unknown|port=table",
        "ms2|index|err.flow.port.missing|port=index",
        "ms2|peaks|err.flow.port.missing|port=peaks",
        "tpl||err.flow.step.unknown|step=tp.generate",
        "tpb||err.flow.step.unknown|step=tp.generate",
        "lnk||err.flow.step.unknown|step=tp.link",
    ];
    assert_eq!(problems(&paper_blocks()[3], Platform::Native), want);
}

#[test]
fn a_step_with_no_wasm_module_is_refused_on_the_web_only() {
    let plan = r#"{"inputs":[{"name":"a.raw","sha256":"aa","type":"raw"}],
        "steps":[{"id":"c","step":"convert.mzml@1","in":{"raw":"file:a.raw"}}]}"#;
    assert!(problems(plan, Platform::Native).is_empty());
    assert_eq!(
        problems(plan, Platform::Wasm),
        ["c||err.flow.unavailable|step=convert.mzml,platform=wasm"]
    );
}

#[test]
fn types_cardinality_facets_cycles_and_ranges() {
    let plan = r#"{"inputs":[{"name":"a.raw","sha256":"aa","type":"raw","facets":{"ms_levels":[2]}},
                             {"name":"s.json","sha256":"bb","type":"samples"}],
        "steps":[
         {"id":"imp","step":"import.raw@1","in":{"files":"file:a.raw"}},
         {"id":"idx","step":"index.mzidx@1","in":{"source":["imp.raw"]}},
         {"id":"det","step":"features.detect@1","in":{"index":"idx.index"},"params":{"min_height":-5,"axis":"furlong","polarity":2.5}},
         {"id":"ali","step":"features.align@1","in":{"peaks":"det.peaks"}},
         {"id":"bad","step":"table.filter@1","in":{"table":["ali.table","ali.table"],"samples":"det.peaks"}},
         {"id":"one","step":"features.gapfill@1","in":{"table":"ali.table","index":"idx.index"}},
         {"id":"c1","step":"table.filter@1","in":{"table":"c2.table","samples":"file:s.json"}},
         {"id":"c2","step":"table.filter@1","in":{"table":"c1.table","samples":"file:s.json"}}]}"#;
    // parameters first, then ports, then cycles; `samples` is a single input and `det.peaks` is a list
    let want = [
        "det||err.flow.param.range|param=min_height",
        "det||err.flow.param.range|param=axis",
        "det||err.flow.param.range|param=polarity",
        // the file only has MS2 scans: detection needs MS1
        "det|index|err.flow.facet|facet=ms_levels",
        "bad|table|err.flow.cardinality|port=table,count=2",
        "bad|samples|err.flow.cardinality|port=samples,count=many",
        "bad|samples|err.flow.type|from=peaks,to=samples",
        "c1||err.flow.cycle|",
        "c2||err.flow.cycle|",
    ];
    assert_eq!(problems(plan, Platform::Native), want);
}

#[test]
fn unknown_version_and_duplicates() {
    let plan = r#"{"steps":[{"id":"a","step":"features.detect@9"},{"id":"a","step":"features.detect@1","preset":"nope"}]}"#;
    let want = [
        "a||err.flow.instance.duplicate|",
        "a||err.flow.step.version|step=features.detect,version=9,available=1",
        "a||err.flow.preset.unknown|preset=nope",
        "a|index|err.flow.port.missing|port=index",
    ];
    assert_eq!(problems(plan, Platform::Native), want);
}

fn detect_hash(params: &str) -> String {
    let plan = parse(
        format!(r#"{{"steps":[{{"id":"d","step":"features.detect@1","params":{params}}}]}}"#)
            .as_bytes(),
    )
    .unwrap();
    let reg = registry();
    let step = reg.iter().find(|s| s.id == "features.detect").unwrap();
    canonical(&plan.steps[0], step).unwrap().params_sha256()
}

#[test]
fn the_canonical_form_ignores_how_the_file_is_written() {
    let base = detect_hash(r#"{"floor":20000,"min_height":100000}"#);
    // key order, float spelling, a value equal to its default, a parameter that does not change the result
    assert_eq!(base, detect_hash(r#"{"min_height":1e5,"floor":2e4}"#));
    assert_eq!(base, detect_hash(r#"{"min_height":100000.0,"axis":"ppm"}"#));
    assert_eq!(base, detect_hash(r#"{"min_height":1e5,"budget":1000000}"#));
    assert_ne!(base, detect_hash(r#"{"min_height":1.0001e5}"#));
}

#[test]
fn the_preset_then_the_file_then_the_algorithm_give_the_values() {
    let reg = registry();
    let step = reg.iter().find(|s| s.id == "features.detect").unwrap();
    let plan = parse(br#"{"steps":[{"id":"d","step":"features.detect@1","preset":"lr","params":{"floor":3000}}]}"#).unwrap();
    let r = canonical(&plan.steps[0], step).unwrap();
    let get = |k: &str| r.params.iter().find(|p| p.0 == k).unwrap().1.clone();
    assert_eq!(get("axis").str(), Some("Da"));
    assert_eq!(get("floor").f64(), Some(3000.0));
    assert_eq!(get("min_height").f64(), Some(20_000.0));
    assert_eq!(r.algorithm, Some(("masstrace".to_string(), 1)));
    assert_eq!(r.preset.as_deref(), Some("lr"));
    // sorted keys, shortest numbers
    assert!(r.text().starts_with(r#"{"axis":"Da","budget":96000000,"#));
}

/// A runner that makes every output from the hash of its inputs, the step and the parameters, and writes down how long it took in a way that varies.
struct Fake {
    counter: AtomicU64,
    fail: Option<(&'static str, usize)>,
}

impl Runner for Fake {
    fn run(&self, job: &Job) -> Result<Outcome> {
        if self.fail == Some((job.instance.id.as_str(), job.item.unwrap_or(0))) {
            return Err(Error::new("err.file.index").with("item", job.item.unwrap_or(0)));
        }
        let seen: Vec<String> = job
            .inputs
            .iter()
            .flat_map(|(_, v)| v.iter().map(|a| a.sha256.clone()))
            .collect();
        let n = self.counter.fetch_add(1, Ordering::SeqCst);
        let text = format!(
            "{}|{}|{:?}",
            seen.join(","),
            job.resolved.params_sha256(),
            job.item
        );
        let outputs = job
            .step
            .outs
            .iter()
            .map(|o| {
                let sha =
                    sha256_hex(format!("{}|{text}|{}", job.instance.step_ref(), o.name).as_bytes());
                (
                    o.name.to_string(),
                    Artifact {
                        sha256: sha,
                        bytes: text.len() as u64,
                        ty: o.ty,
                        label: None,
                    },
                )
            })
            .collect();
        Ok(Outcome {
            outputs,
            ms: n as f64 + 0.5,
            heap_peak_bytes: Some(n * 1000),
            ..Outcome::default()
        })
    }
}

const CHAIN: &str = r#"{"format":"mzflow/2","program":{"name":"mzLab","version":"x"},
 "inputs":[{"name":"a.raw","sha256":"0a","bytes":10,"type":"raw","facets":{"ms_levels":[1,2]}},
           {"name":"b.raw","sha256":"0b","bytes":20,"type":"raw","facets":{"ms_levels":[1,2]}},
           {"name":"c.raw","sha256":"0c","bytes":30,"type":"raw","facets":{"ms_levels":[1,2]}}],
 "steps":[
  {"id":"gap","step":"features.gapfill@1","in":{"table":"ali.table","index":"idx.index"}},
  {"id":"ali","step":"features.align@1","in":{"peaks":"det.peaks"}},
  {"id":"det","step":"features.detect@1","in":{"index":"idx.index"},"preset":"hr"},
  {"id":"est","step":"features.estimate@1","in":{"index":"idx.index"}},
  {"id":"idx","step":"index.mzidx@1","in":{"source":"imp.raw"}},
  {"id":"imp","step":"import.raw@1","in":{"files":["file:a.raw","file:b.raw","file:c.raw"]}}]}"#;

fn agent() -> Agent {
    Agent {
        name: "mzLab".into(),
        version: "x".into(),
        commit: None,
        platform: "native",
    }
}

#[test]
fn one_worker_or_many_give_the_same_record_but_the_clock() {
    let plan = parse(CHAIN.as_bytes()).unwrap();
    let reg = registry();
    assert_eq!(check(&plan, &reg, Platform::Native), []);
    let run = |workers: usize| {
        let r = execute(
            &plan,
            &reg,
            &Fake {
                counter: AtomicU64::new(0),
                fail: None,
            },
            workers,
        )
        .unwrap();
        let mut r = r;
        for a in &mut r.activities {
            a.ms = 0.0;
            a.heap_peak_bytes = None;
        }
        let text = prov_json(&agent(), Some("p"), &r);
        (r, text)
    };
    let (one, one_text) = run(1);
    let (many, many_text) = run(4);
    assert_eq!(one, many);
    let ids: Vec<&str> = one.activities.iter().map(|a| a.instance.as_str()).collect();
    assert_eq!(ids, ["imp", "idx", "det", "est", "ali", "gap"]);
    // the outputs of a per_item step are gathered by index of the input
    let imp = &one.activities[0];
    assert_eq!(imp.used, ["file:a.raw", "file:b.raw", "file:c.raw"]);
    assert_eq!(imp.generated, ["imp.raw[0]", "imp.raw[1]", "imp.raw[2]"]);
    assert_eq!(
        one.activities[4].used,
        ["det.peaks[0]", "det.peaks[1]", "det.peaks[2]"]
    );
    assert_eq!(one_text, many_text);
}

#[test]
fn an_error_stops_only_the_descendants() {
    let plan = parse(CHAIN.as_bytes()).unwrap();
    let run = execute(
        &plan,
        &registry(),
        &Fake {
            counter: AtomicU64::new(0),
            fail: Some(("det", 1)),
        },
        2,
    )
    .unwrap();
    let ids: Vec<&str> = run.activities.iter().map(|a| a.instance.as_str()).collect();
    assert_eq!(ids, ["imp", "idx", "det", "est"]);
    assert_eq!(run.skipped, ["ali", "gap"]);
    let det = run.activities.iter().find(|a| a.instance == "det").unwrap();
    assert_eq!(det.error.as_ref().unwrap().error_key, "err.file.index");
    assert!(det.generated.is_empty());
    assert!(run
        .activities
        .iter()
        .find(|a| a.instance == "est")
        .unwrap()
        .error
        .is_none());
}

#[test]
fn a_plan_with_problems_does_not_start() {
    let plan = parse(br#"{"steps":[{"id":"x","step":"nothing@1"}]}"#).unwrap();
    let e = execute(
        &plan,
        &registry(),
        &Fake {
            counter: AtomicU64::new(0),
            fail: None,
        },
        1,
    )
    .unwrap_err();
    assert_eq!(e.key, "err.flow.check");
}
