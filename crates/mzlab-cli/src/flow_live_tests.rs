//! `LiveRun` on three synthetic mzML files: the run equals `flow run` (files and `prov/1`), an update keeps the jobs whose `plan_key` is the same and
//! cancels only the running ones whose key changed, a cancelled job leaves no half file, the events follow `docs/schemi/flowlive-1.json`.

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use mzlab_core::flow::sched::{JobId, State};
use mzlab_core::json::{parse, Value};

use mzlab_core::flow::Hooks;

use super::LiveRun;
use crate::flow::{steps, FileRunner, Meter};
use crate::flow_run_tests::{fixture, flow_run, folder, p, untimed};

const LIMIT: Duration = Duration::from_secs(120);

/// The plan of the fixture with `extra` parameters on the step `id`.
fn plan_with(dir: &Path, id: &str, params: &str) -> Vec<u8> {
    let text = std::fs::read_to_string(dir.join("plan.json")).unwrap();
    let at = format!(r#""id":"{id}","step":"#);
    let k = text.find(&at).unwrap() + at.len();
    let end = k + text[k..].find('"').unwrap() + 1;
    let end = end + text[end..].find('"').unwrap() + 1;
    let (head, tail) = text.split_at(end);
    format!(r#"{head},"params":{params}{tail}"#).into_bytes()
}

fn live(dir: &Path, text: &[u8], tag: &str, workers: usize) -> LiveRun {
    let work = dir.join(tag);
    LiveRun::start(text, steps(), &dir.join("data"), &work, workers, &|_| false).unwrap()
}

/// Checks one event line against the schema file: required keys, no other key, the enums and the types.
fn valid_event(line: &str, schema: &Value) {
    let ev = parse(line.as_bytes()).unwrap();
    let props = schema.get("properties").unwrap();
    let Value::Obj(fields) = &ev else {
        panic!("{line}")
    };
    for k in schema.get("required").unwrap().arr() {
        assert!(ev.get(k.str().unwrap()).is_some(), "{line}: {k:?}");
    }
    for (k, v) in fields {
        let spec = props.get(k).unwrap_or_else(|| panic!("{line}: key {k}"));
        if let Some(c) = spec.get("const") {
            assert_eq!(v.str(), c.str(), "{line}");
        }
        if let Some(e) = spec.get("enum") {
            assert!(e.arr().iter().any(|x| x.str() == v.str()), "{line}: {k}");
        }
        if let Some(t) = spec.get("type") {
            let types: Vec<&str> = if t.str().is_some() {
                vec![t.str().unwrap()]
            } else {
                t.arr().iter().filter_map(Value::str).collect()
            };
            let ok = v.is_null() && types.contains(&"null")
                || v.str().is_some() && types.contains(&"string")
                || v.f64().is_some() && types.iter().any(|t| ["integer", "number"].contains(t))
                || matches!(v, Value::Obj(_)) && types.contains(&"object");
            assert!(ok, "{line}: {k}");
        }
    }
}

#[test]
fn flow_live_gives_the_files_and_the_provenance_of_flow_run() {
    let (dir, _) = fixture("live_same");
    let out = dir.join("out");
    let o = flow_run(&[
        p(&dir.join("plan.json")),
        "--in",
        p(&dir.join("data")),
        "--out",
        p(&out),
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let text = std::fs::read(out.join("plan.json")).unwrap();

    let run = live(&dir, &text, "w1", 2);
    assert!(run.wait_idle(LIMIT));
    let done = run.finish().unwrap();
    let prov = crate::flow_run::prov_text(&text, &done);
    assert_eq!(
        untimed(&prov),
        untimed(&std::fs::read_to_string(out.join("prov.json")).unwrap())
    );

    let schema = parse(&std::fs::read("../../docs/schemi/flowlive-1.json").unwrap()).unwrap();
    let lines = run.event_lines(0);
    assert!(lines.iter().any(|(_, l)| l.contains(r#""state":"done""#)));
    for (_, l) in &lines {
        valid_event(l, &schema);
    }
    drop(run);

    // `--jsonl` changes the standard output only
    let again = dir.join("again");
    let o = flow_run(&[
        p(&dir.join("plan.json")),
        "--in",
        p(&dir.join("data")),
        "--out",
        p(&again),
        "--jsonl",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(folder(&out), folder(&again));
    let o = flow_run(&[
        p(&dir.join("plan.json")),
        "--in",
        p(&dir.join("data")),
        "--out",
        p(&again),
        "--jsonl",
        "--json",
    ]);
    assert_eq!(o.code, 2, "{}", o.stderr);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The two `prov/1` texts are the same but for the times; the first difference is shown otherwise.
fn same_prov(a: &str, b: &str) {
    let (a, b) = (untimed(a), untimed(b));
    let k = a.bytes().zip(b.bytes()).position(|(x, y)| x != y);
    if let Some(k) = k.or((a.len() != b.len()).then_some(0)) {
        let at = |t: &str| t[k.saturating_sub(200)..(k + 200).min(t.len())].to_string();
        panic!("prov/1 differs near {k}:\n{}\n{}", at(&a), at(&b));
    }
}

static CALLS: AtomicUsize = AtomicUsize::new(0);
static HOLD_AT: AtomicUsize = AtomicUsize::new(usize::MAX);
static HOLD: AtomicBool = AtomicBool::new(false);
/// Tests that count the calls of the meter run one at a time.
static SERIAL: Mutex<()> = Mutex::new(());

fn tick() {
    let n = CALLS.fetch_add(1, Ordering::SeqCst);
    while n == HOLD_AT.load(Ordering::SeqCst) && HOLD.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn zero() -> u64 {
    0
}

/// A live run of one worker whose `hold`-th job (from 0) waits in the meter until `HOLD` goes down.
fn held(dir: &Path, text: &[u8], tag: &str, hold: usize) -> LiveRun {
    CALLS.store(0, Ordering::SeqCst);
    HOLD_AT.store(hold, Ordering::SeqCst);
    HOLD.store(true, Ordering::SeqCst);
    let meter = Meter {
        reset: tick,
        peak: zero,
    };
    let mut plan = mzlab_core::flow::parse(text).unwrap();
    for f in &mut plan.inputs {
        let (bytes, sha) = crate::flow_set::digest(&dir.join("data").join(&f.name)).unwrap();
        (f.sha256, f.bytes) = (sha, Some(bytes));
    }
    std::fs::create_dir_all(dir.join(tag)).unwrap();
    let runner = FileRunner::cached(&dir.join("data"), &dir.join(tag), Some(meter), Some(false));
    LiveRun::with_runner(plan, steps(), runner, &dir.join("data"), (1, &|_| false)).unwrap()
}

fn running(run: &LiveRun, id: &JobId) -> bool {
    (0..400).any(|_| {
        std::thread::sleep(Duration::from_millis(10));
        let jobs = super::lock(&run.shared.core).sched.jobs();
        jobs.iter()
            .any(|(j, s)| j == id && matches!(s, State::Running { .. }))
    })
}

#[test]
fn flow_live_update_in_the_middle_of_the_run_keeps_the_detections_when_only_the_alignment_changes()
{
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (dir, _) = fixture("live_ali");
    let base = std::fs::read(dir.join("plan.json")).unwrap();
    // idx0 det0 idx1 det1 idx2 det2 ali gap: the alignment is the seventh job
    let run = held(&dir, &base, "w_a", 6);
    let ali = JobId::new("ali", None);
    assert!(running(&run, &ali));
    let seq = run.event_lines(0).last().map_or(0, |x| x.0);
    let changed = plan_with(&dir, "ali", r#"{"rt_gap":0.2}"#);
    let cancels = run.update(&changed, &|_| false).unwrap();
    assert!(cancels.contains(r#""instance":"ali""#), "{cancels}");
    assert!(
        !cancels.contains(r#""instance":"det""#) && !cancels.contains(r#""instance":"idx""#),
        "{cancels}"
    );
    HOLD.store(false, Ordering::SeqCst);
    assert!(run.wait_idle(LIMIT));
    let again: Vec<String> = run.event_lines(seq).into_iter().map(|x| x.1).collect();
    let redone = |step: &str| {
        again.iter().any(|l| {
            l.contains(&format!(r#""instance":"{step}""#)) && l.contains(r#""state":"running""#)
        })
    };
    assert!(redone("ali") && redone("gap"), "{again:?}");
    assert!(!redone("det") && !redone("idx"), "{again:?}");

    std::fs::create_dir_all(dir.join("fresh")).unwrap();
    let fresh = crate::flow::run_plan(
        &changed,
        &dir.join("data"),
        &dir.join("fresh"),
        1,
        Some(Meter {
            reset: tick,
            peak: zero,
        }),
    )
    .unwrap();
    same_prov(
        &fresh.1,
        &crate::flow_run::prov_text(&changed, &run.finish().unwrap()),
    );
    drop(run);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn flow_live_new_detection_parameter_cancels_only_the_detection_that_runs() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (dir, _) = fixture("live_det");
    let base = std::fs::read(dir.join("plan.json")).unwrap();
    // the fourth job is the detection of the second file
    let run = held(&dir, &base, "w_d", 3);
    let det1 = JobId::new("det", Some("s_t010.mzML"));
    assert!(running(&run, &det1));
    let changed = plan_with(&dir, "det", r#"{"min_height":50000}"#);
    let cancels = run.update(&changed, &|_| false).unwrap();
    assert!(
        cancels.contains("s_t010.mzML") && cancels.matches("instance").count() == 1,
        "{cancels}"
    );
    assert!(cancels.contains(r#""instance":"det""#), "{cancels}");
    HOLD.store(false, Ordering::SeqCst);
    assert!(run.wait_idle(LIMIT));
    let done = run.finish().unwrap();
    std::fs::create_dir_all(dir.join("fresh")).unwrap();
    let fresh = crate::flow::run_plan(
        &changed,
        &dir.join("data"),
        &dir.join("fresh"),
        1,
        Some(Meter {
            reset: tick,
            peak: zero,
        }),
    )
    .unwrap();
    same_prov(&fresh.1, &crate::flow_run::prov_text(&changed, &done));
    drop(run);

    // nothing half-written stays in the work folder: every file is whole (an index or a feature file reads)
    for e in std::fs::read_dir(dir.join("w_d")).unwrap().flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        assert!(
            !name.ends_with(".part") && !name.ends_with(".tmp"),
            "{name}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn flow_live_keeps_a_result_while_its_files_are_as_they_were() {
    let (dir, _) = fixture("live_back");
    let base = std::fs::read(dir.join("plan.json")).unwrap();
    let run = live(&dir, &base, "w_b", 1);
    assert!(run.wait_idle(LIMIT));
    let key = super::lock(&run.shared.core)
        .sched
        .key(&JobId::new("ali", None))
        .unwrap()
        .to_string();
    assert!(run.shared.reuse(&key).is_some_and(|o| o.cached));
    let file = super::lock(&run.shared.kept)
        .get(key.as_str())
        .unwrap()
        .files[0]
        .0
        .clone();
    std::fs::write(&file, b"another table").unwrap();
    assert!(
        run.shared.reuse(&key).is_none(),
        "a file written again is not the result of the key"
    );
    drop(run);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn flow_live_a_claimed_job_is_run_by_the_caller_and_the_rest_follows() {
    use mzlab_core::step::DataType;

    use super::Lend;

    let (dir, _) = fixture("live_lend");
    let base = std::fs::read(dir.join("plan.json")).unwrap();
    let first = live(&dir, &base, "w_l1", 2);
    assert!(first.wait_idle(LIMIT));
    drop(first);

    let work = dir.join("w_l2");
    let take = |id: &JobId| id.instance == "idx";
    let run = LiveRun::start(&base, steps(), &dir.join("data"), &work, 2, &take).unwrap();
    let idx = run.jobs_of("index.mzidx");
    assert_eq!(idx.len(), 3);
    // nothing past the claimed jobs starts, and the workers do not take them
    std::thread::sleep(Duration::from_millis(300));
    let state = |id: &JobId| {
        super::lock(&run.shared.core)
            .sched
            .jobs()
            .into_iter()
            .find(|j| j.0 == *id)
            .unwrap()
            .1
    };
    assert!(
        matches!(state(&idx[0]), State::Ready),
        "{:?}",
        state(&idx[0])
    );
    assert!(run.lend(&idx[0], Lend::Running(Some(0.5))));
    assert!(matches!(state(&idx[0]), State::Running { frac: Some(f) } if f == 0.5));
    // the files the caller made are the ones of the first run
    for (k, id) in idx.iter().enumerate() {
        let name = format!("idx-s_t{:03}.mzidx", [0, 10, 30][k]);
        let file = dir.join("w_l1").join(name);
        assert!(run.lend(id, Lend::Done(vec![("index", file, DataType::Mzidx)])));
    }
    assert!(run.wait_idle(LIMIT));
    assert!(run
        .jobs_of("features.align")
        .iter()
        .all(|id| matches!(state(id), State::Done)));
    assert!(run.finish().is_ok());
    drop(run);
    let _ = std::fs::remove_dir_all(&dir);
}
