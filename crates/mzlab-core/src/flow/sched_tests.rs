//! The live scheduler on a chain of four per-file steps and a barrier (`convert → centroid → index → detect → align`); the shared cases are in `sched_cases`.

use super::*;
use crate::flow::{parse, value_text};
use crate::json::{self, Value};
use crate::step::{DataType, Param, Port};

/// The steps of the tests as data, the form the shared cases carry: `name*` is a `many` port, `params` are numbers.
const STEPS: &str = r#"[
{"id":"t.convert","per_item":true,"ins":["raw*"],"outs":["mz*"]},
{"id":"t.centroid","per_item":true,"ins":["mz*"],"outs":["mz*"]},
{"id":"t.index","per_item":true,"ins":["mz*"],"outs":["index*"]},
{"id":"features.detect","per_item":true,"ins":["index*"],"outs":["peaks*"],"params":{"q":1}},
{"id":"features.align","ins":["peaks*"],"outs":["table"],"params":{"q":1}}]"#;

fn leak(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

fn ports(v: Option<&Value>) -> Vec<Port> {
    v.map(Value::arr)
        .unwrap_or_default()
        .iter()
        .filter_map(Value::str)
        .map(|n| {
            let p = Port::new(leak(n.trim_end_matches('*')), DataType::Raw, "");
            if n.ends_with('*') {
                p.many()
            } else {
                p
            }
        })
        .collect()
}

fn registry_from(spec: &Value) -> Vec<Step> {
    spec.arr()
        .iter()
        .map(|s| {
            let mut st = Step::core(leak(s.get("id").and_then(Value::str).unwrap()), "", "");
            st.per_item = s.get("per_item").and_then(Value::bool).unwrap_or(false);
            st.runs_in.native = true;
            st.ins = ports(s.get("ins"));
            st.outs = ports(s.get("outs"));
            if let Some(Value::Obj(kv)) = s.get("params") {
                st.params = kv
                    .iter()
                    .map(|(k, v)| Param::number(leak(k), "").default_num(v.f64().unwrap()))
                    .collect();
            }
            st
        })
        .collect()
}

pub(in crate::flow) fn registry() -> Vec<Step> {
    registry_from(&json::parse(STEPS.as_bytes()).unwrap())
}

/// The chain over `files`, with the `q` of detect and of align.
pub(in crate::flow) fn plan_text(files: &[&str], dq: u32, aq: u32) -> String {
    let inputs: Vec<String> = files
        .iter()
        .map(|f| format!("{{\"name\":\"{f}\",\"sha256\":\"sha-{f}\"}}"))
        .collect();
    let refs: Vec<String> = files.iter().map(|f| format!("\"file:{f}\"")).collect();
    format!(
        "{{\"format\":\"mzflow/2\",\"inputs\":[{}],\"steps\":[\
{{\"id\":\"conv\",\"step\":\"t.convert@1\",\"in\":{{\"raw\":[{}]}}}},\
{{\"id\":\"cen\",\"step\":\"t.centroid@1\",\"in\":{{\"mz\":[\"conv.mz\"]}}}},\
{{\"id\":\"idx\",\"step\":\"t.index@1\",\"in\":{{\"mz\":[\"cen.mz\"]}}}},\
{{\"id\":\"det\",\"step\":\"features.detect@1\",\"params\":{{\"q\":{dq}}},\"in\":{{\"index\":[\"idx.index\"]}}}},\
{{\"id\":\"aln\",\"step\":\"features.align@1\",\"params\":{{\"q\":{aq}}},\"in\":{{\"peaks\":[\"det.peaks\"]}}}}]}}",
        inputs.join(","),
        refs.join(",")
    )
}

fn plan(files: &[&str], dq: u32, aq: u32) -> Plan {
    parse(plan_text(files, dq, aq).as_bytes()).unwrap()
}

fn sched(files: &[&str]) -> Sched {
    Sched::new(&plan(files, 1, 1), &registry(), "test-1", &[])
}

fn jid(s: &str) -> JobId {
    match s.split_once('|') {
        Some((i, n)) => JobId::new(i, Some(n)),
        None => JobId::new(s, None),
    }
}

fn name(id: &JobId) -> String {
    match &id.item {
        Some(n) => format!("{}|{n}", id.instance),
        None => id.instance.clone(),
    }
}

fn state(s: &Sched, job: &str) -> &'static str {
    s.state(&jid(job)).map_or("gone", |x| x.name())
}

const ALL: Limits = Limits {
    slots: 99,
    mem_mb: 1e9,
};

fn names(s: &Sched, lim: &Limits) -> Vec<String> {
    s.next(lim, |_| 1.0).iter().map(|j| name(&j.id)).collect()
}

/// Starts and finishes one job.
fn run(s: &mut Sched, job: &str) {
    assert!(
        s.started(&jid(job)),
        "{job} is not startable: {}",
        state(s, job)
    );
    assert_eq!(
        s.finished(&jid(job), Ok(Done { ms: 1.0 })),
        Landed::Accepted
    );
}

/// Runs everything that is ready until nothing is.
fn run_all(s: &mut Sched) {
    while let Some(j) = s.next(&ALL, |_| 1.0).first().map(|j| name(&j.id)) {
        run(s, &j);
    }
}

#[test]
fn a_file_moves_down_the_chain_while_another_is_still_converting() {
    let mut s = sched(&["f1.raw", "f2.raw", "f3.raw"]);
    assert_eq!(
        names(&s, &ALL),
        ["conv|f1.raw", "conv|f2.raw", "conv|f3.raw"]
    );
    for step in ["conv", "cen", "idx"] {
        run(&mut s, &format!("{step}|f1.raw"));
    }
    assert_eq!(state(&s, "conv|f3.raw"), "ready");
    // the file under way goes deeper before a new one is opened
    assert_eq!(names(&s, &ALL)[0], "det|f1.raw");
    assert_eq!(state(&s, "det|f1.raw"), "ready");
    assert_eq!(state(&s, "aln"), "waiting");
}

#[test]
fn the_barrier_waits_for_the_last_file() {
    let mut s = sched(&["f1.raw", "f2.raw"]);
    run_all(&mut s);
    assert_eq!(state(&s, "aln"), "done");
    let mut s = sched(&["f1.raw", "f2.raw"]);
    for f in ["f1.raw", "f2.raw"] {
        for step in ["conv", "cen", "idx"] {
            run(&mut s, &format!("{step}|{f}"));
        }
    }
    run(&mut s, "det|f1.raw");
    assert_eq!(state(&s, "aln"), "waiting");
    let n = s.nodes().into_iter().find(|n| n.instance == "det").unwrap();
    assert_eq!((n.parte, n.di), (1, 2));
    run(&mut s, "det|f2.raw");
    assert_eq!(state(&s, "aln"), "ready");
}

#[test]
fn a_parameter_of_align_makes_only_align_stale() {
    let mut s = sched(&["f1.raw", "f2.raw"]);
    run_all(&mut s);
    let cmds = s.update(&plan(&["f1.raw", "f2.raw"], 1, 2));
    assert!(cmds.is_empty());
    assert_eq!(state(&s, "aln"), "stale");
    assert_eq!(state(&s, "det|f1.raw"), "done");
    assert_eq!(names(&s, &ALL), ["aln"]);
    run(&mut s, "aln");
    assert_eq!(state(&s, "aln"), "done");
}

#[test]
fn a_parameter_of_detect_cancels_the_running_detect_and_stales_align() {
    let mut s = sched(&["f1.raw", "f2.raw"]);
    run_all(&mut s);
    s.update(&plan(&["f1.raw", "f2.raw"], 2, 1));
    assert_eq!(
        (state(&s, "det|f1.raw"), state(&s, "aln")),
        ("stale", "stale")
    );
    assert!(s.started(&jid("det|f2.raw")));
    let old = s.key(&jid("det|f2.raw")).unwrap().to_string();
    let cmds = s.update(&plan(&["f1.raw", "f2.raw"], 3, 1));
    assert_eq!(cmds, [Command::Cancel(jid("det|f2.raw"))]);
    assert_eq!(state(&s, "aln"), "stale");
    // the new run of f2 cannot start before the cancelled one reports, which is not used but belongs to its key
    assert!(!names(&s, &ALL).contains(&"det|f2.raw".to_string()));
    assert_eq!(
        s.finished(&jid("det|f2.raw"), Ok(Done { ms: 1.0 })),
        Landed::Discarded { key: old }
    );
    assert!(names(&s, &ALL).contains(&"det|f2.raw".to_string()));
}

#[test]
fn a_file_added_at_the_end_adds_its_jobs_and_stales_the_barrier() {
    let mut s = sched(&["f1.raw", "f2.raw"]);
    run_all(&mut s);
    let before = s.jobs().len();
    s.update(&plan(&["f1.raw", "f2.raw", "f3.raw"], 1, 1));
    assert_eq!(s.jobs().len(), before + 4);
    assert_eq!(state(&s, "det|f1.raw"), "done");
    assert_eq!(state(&s, "conv|f3.raw"), "ready");
    assert_eq!(state(&s, "det|f3.raw"), "waiting");
    assert_eq!(state(&s, "aln"), "stale");
}

#[test]
fn a_file_added_at_the_front_stales_the_detections_but_not_the_conversions() {
    // the recipe of detect comes from the index of the FIRST input of the plan (decision D2): a new first file changes it for every detection
    let mut s = sched(&["f2.raw", "f3.raw"]);
    run_all(&mut s);
    s.update(&plan(&["f1.raw", "f2.raw", "f3.raw"], 1, 1));
    for j in ["conv|f2.raw", "cen|f2.raw", "idx|f2.raw", "idx|f3.raw"] {
        assert_eq!(state(&s, j), "done", "{j}");
    }
    for j in ["det|f2.raw", "det|f3.raw", "aln"] {
        assert_eq!(state(&s, j), "stale", "{j}");
    }
}

#[test]
fn a_node_taken_out_cancels_its_jobs() {
    let mut s = sched(&["f1.raw", "f2.raw"]);
    for step in ["conv", "cen", "idx"] {
        run(&mut s, &format!("{step}|f1.raw"));
    }
    assert!(s.started(&jid("det|f1.raw")));
    let text = plan_text(&["f1.raw", "f2.raw"], 1, 1);
    let cut = text.find(",{\"id\":\"det\"").unwrap();
    let short = parse(format!("{}]}}", &text[..cut]).as_bytes()).unwrap();
    let cmds = s.update(&short);
    assert_eq!(cmds, [Command::Cancel(jid("det|f1.raw"))]);
    assert_eq!(state(&s, "det|f1.raw"), "gone");
    assert_eq!(state(&s, "aln"), "gone");
    assert_eq!(
        s.finished(&jid("det|f1.raw"), Ok(Done { ms: 1.0 })).clone(),
        Landed::Discarded {
            key: s_key_of(&text)
        }
    );
}

/// The key det|f1 had in `text` (rebuilt on a fresh scheduler).
fn s_key_of(text: &str) -> String {
    let s = Sched::new(&parse(text.as_bytes()).unwrap(), &registry(), "test-1", &[]);
    s.key(&jid("det|f1.raw")).unwrap().to_string()
}

#[test]
fn the_key_comes_back_after_a_parameter_goes_and_returns() {
    let mut s = sched(&["f1.raw"]);
    let k = s.key(&jid("det|f1.raw")).unwrap().to_string();
    s.update(&plan(&["f1.raw"], 5, 1));
    assert_ne!(s.key(&jid("det|f1.raw")).unwrap(), k);
    s.update(&plan(&["f1.raw"], 1, 1));
    assert_eq!(s.key(&jid("det|f1.raw")).unwrap(), k);
}

#[test]
fn next_respects_slots_memory_and_exclusive_resources() {
    let mut s = sched(&["f1.raw", "f2.raw", "f3.raw"]);
    assert_eq!(
        names(
            &s,
            &Limits {
                slots: 2,
                mem_mb: 1e9
            }
        )
        .len(),
        2
    );
    let spec = s.next(
        &Limits {
            slots: 9,
            mem_mb: 5.0,
        },
        |_| 2.0,
    );
    assert_eq!(spec.len(), 2);
    assert!(spec.iter().all(|j| j.mb == 2.0));
    for f in ["f1.raw", "f2.raw"] {
        s.set_resources(&jid(&format!("conv|{f}")), vec!["dataset:x.wiff".into()]);
    }
    assert_eq!(names(&s, &ALL), ["conv|f1.raw", "conv|f3.raw"]);
    assert!(s.started(&jid("conv|f1.raw")));
    assert_eq!(names(&s, &ALL), ["conv|f3.raw"]);
    s.finished(&jid("conv|f1.raw"), Ok(Done { ms: 1.0 }));
    assert!(names(&s, &ALL).contains(&"conv|f2.raw".to_string()));
}

#[test]
fn the_order_of_the_keys_in_the_file_does_not_change_the_plan_keys() {
    let a = Sched::new(
        &plan(&["f1.raw", "f2.raw"], 1, 1),
        &registry(),
        "test-1",
        &[],
    );
    // the steps in another order, the object keys in another order
    let text = "{\"steps\":[\
{\"in\":{\"peaks\":[\"det.peaks\"]},\"params\":{\"q\":1},\"step\":\"features.align@1\",\"id\":\"aln\"},\
{\"in\":{\"index\":[\"idx.index\"]},\"params\":{\"q\":1},\"step\":\"features.detect@1\",\"id\":\"det\"},\
{\"in\":{\"mz\":[\"cen.mz\"]},\"step\":\"t.index@1\",\"id\":\"idx\"},\
{\"in\":{\"mz\":[\"conv.mz\"]},\"step\":\"t.centroid@1\",\"id\":\"cen\"},\
{\"in\":{\"raw\":[\"file:f1.raw\",\"file:f2.raw\"]},\"step\":\"t.convert@1\",\"id\":\"conv\"}],\
\"inputs\":[{\"sha256\":\"sha-f1.raw\",\"name\":\"f1.raw\"},{\"sha256\":\"sha-f2.raw\",\"name\":\"f2.raw\"}],\"format\":\"mzflow/2\"}";
    let b = Sched::new(&parse(text.as_bytes()).unwrap(), &registry(), "test-1", &[]);
    for (id, _) in a.jobs() {
        assert_eq!(a.key(&id), b.key(&id), "{}", name(&id));
    }
    assert_eq!(a.jobs().len(), b.jobs().len());
}

#[test]
fn keys_do_not_hold_the_position_of_an_item() {
    let a = sched(&["f1.raw", "f2.raw"]);
    let b = sched(&["f0.raw", "f1.raw", "f2.raw"]);
    for j in ["conv|f1.raw", "cen|f2.raw", "idx|f2.raw"] {
        assert_eq!(a.key(&jid(j)), b.key(&jid(j)), "{j}");
    }
    assert!(a
        .jobs()
        .iter()
        .all(|(id, _)| id.item.as_deref().is_none_or(|n| n.ends_with(".raw"))));
}

#[test]
fn an_input_without_sha_is_identified_by_the_text_the_caller_gives() {
    let text = plan_text(&["f1.raw"], 1, 1).replace("\"sha256\":\"sha-f1.raw\"", "\"sha256\":\"\"");
    let p = parse(text.as_bytes()).unwrap();
    let id = [("f1.raw".to_string(), "f1.raw:10:1".to_string())];
    let a = Sched::new(&p, &registry(), "test-1", &id);
    let b = Sched::new(
        &p,
        &registry(),
        "test-1",
        &[("f1.raw".into(), "f1.raw:11:1".into())],
    );
    assert_ne!(a.key(&jid("conv|f1.raw")), b.key(&jid("conv|f1.raw")));
    let v2 = Sched::new(&p, &registry(), "test-2", &id);
    assert_ne!(a.key(&jid("conv|f1.raw")), v2.key(&jid("conv|f1.raw")));
}

/// Two files that carry the same placeholder fingerprint (the page sends `loading` for every index it is still waiting for) never share a job key: a key that held no name made
/// every detection of the series land in one file.
#[test]
fn files_with_the_same_identity_get_different_keys() {
    let text = plan_text(&["f1.raw", "f2.raw"], 1, 1).replace("\"sha256\":\"sha-f1.raw\"", "\"sha256\":\"\"").replace("\"sha256\":\"sha-f2.raw\"", "\"sha256\":\"\"");
    let p = parse(text.as_bytes()).unwrap();
    let same = [("f1.raw".to_string(), "loading".to_string()), ("f2.raw".to_string(), "loading".to_string())];
    let s = Sched::new(&p, &registry(), "test-1", &same);
    for step in ["conv", "cen", "idx", "det"] {
        assert_ne!(s.key(&jid(&format!("{step}|f1.raw"))), s.key(&jid(&format!("{step}|f2.raw"))), "{step}");
    }
}

/// A fingerprint set after the session started (the index of a file that was loading) takes effect at the next `update`, even with the plan unchanged: the page must call it.
#[test]
fn a_new_identity_rekeys_the_jobs_at_the_next_update() {
    let text = plan_text(&["f1.raw"], 1, 1).replace("\"sha256\":\"sha-f1.raw\"", "\"sha256\":\"\"");
    let p = parse(text.as_bytes()).unwrap();
    let mut s = Sched::new(&p, &registry(), "test-1", &[("f1.raw".into(), "loading".into())]);
    let before = s.key(&jid("det|f1.raw")).map(String::from);
    s.set_identity("f1.raw", "10:1:centroid");
    assert_eq!(s.key(&jid("det|f1.raw")).map(String::from), before);
    s.update(&p);
    assert_ne!(s.key(&jid("det|f1.raw")).map(String::from), before);
}

#[test]
fn a_node_shows_its_worst_job_and_the_mean_share() {
    let mut s = sched(&["f1.raw", "f2.raw"]);
    run(&mut s, "conv|f1.raw");
    assert!(s.started(&jid("conv|f2.raw")));
    let card = |s: &Sched| {
        s.nodes()
            .into_iter()
            .find(|n| n.instance == "conv")
            .unwrap()
    };
    assert_eq!((card(&s).state.name(), card(&s).frac), ("running", None));
    s.progress(&jid("conv|f2.raw"), 0.5);
    assert_eq!(card(&s).frac, Some(0.75));
    s.finished(&jid("conv|f2.raw"), Err(Error::new("error.featMemory")));
    assert_eq!(card(&s).state.name(), "refused");
    assert_eq!(state(&s, "cen|f2.raw"), "skipped");
    s.retry(&jid("conv|f2.raw"));
    assert_eq!(state(&s, "conv|f2.raw"), "ready");
}

#[test]
fn cached_pause_and_a_held_job() {
    let mut s = sched(&["f1.raw", "f2.raw"]);
    assert!(s.cached(&jid("conv|f1.raw")));
    assert_eq!(state(&s, "conv|f1.raw"), "reused");
    assert_eq!(state(&s, "cen|f1.raw"), "ready");
    s.pause();
    assert!(names(&s, &ALL).is_empty());
    assert_eq!(state(&s, "conv|f2.raw"), "paused");
    s.resume();
    assert!(s.block(&jid("conv|f2.raw"), "permission.memory"));
    assert_eq!(state(&s, "conv|f2.raw"), "blocked");
    assert!(!names(&s, &ALL).contains(&"conv|f2.raw".to_string()));
    s.unblock(&jid("conv|f2.raw"));
    assert_eq!(state(&s, "conv|f2.raw"), "ready");
}

#[test]
fn a_step_that_does_not_exist_blocks_its_node_only() {
    let text = plan_text(&["f1.raw"], 1, 1).replace("t.centroid@1", "t.nothing@1");
    let s = Sched::new(&parse(text.as_bytes()).unwrap(), &registry(), "test-1", &[]);
    assert_eq!(
        (state(&s, "conv|f1.raw"), state(&s, "cen")),
        ("ready", "blocked")
    );
    // downstream of an unknown step the number of items is unknown: one job, not one per file
    assert_eq!(state(&s, "idx"), "waiting");
}

#[test]
fn events_grow_in_seq_and_carry_the_revision() {
    let mut s = sched(&["f1.raw"]);
    run(&mut s, "conv|f1.raw");
    s.update(&plan(&["f1.raw"], 2, 1));
    let all = json::parse(s.events_since(0).as_bytes()).unwrap();
    let seqs: Vec<f64> = all
        .arr()
        .iter()
        .map(|e| e.get("seq").unwrap().f64().unwrap())
        .collect();
    assert!(seqs.windows(2).all(|w| w[1] == w[0] + 1.0) && seqs[0] == 1.0);
    let last = all.arr().last().unwrap();
    assert_eq!(last.get("rev").unwrap().f64(), Some(1.0));
    assert_eq!(s.rev(), 1);
    let tail = json::parse(s.events_since(seqs[seqs.len() - 1] as u64 - 1).as_bytes()).unwrap();
    assert_eq!(tail.arr().len(), 1);
    assert_eq!(s.events_since(1_000_000), "[]");
}

#[path = "sched_cases.rs"]
mod cases;
