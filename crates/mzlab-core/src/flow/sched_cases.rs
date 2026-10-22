//! The shared cases of `tests/data/sched_cases.json` played on the scheduler: the file lists steps, a plan and calls, and carries the answer of each call.

use super::*;

const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/sched_cases.json");

#[test]
fn the_shared_cases_give_the_written_answers() {
    let text = std::fs::read_to_string(PATH).unwrap();
    let cases = json::parse(text.as_bytes()).unwrap();
    let update = std::env::var("MZLAB_UPDATE_SCHED_CASES").is_ok();
    let mut out = Vec::new();
    for case in cases.arr() {
        let played = play(case);
        let written = case.get("calls").unwrap().arr();
        if !update {
            let n = case.get("name").and_then(Value::str).unwrap_or("?");
            assert_eq!(played.len(), written.len(), "{n}");
            for (i, (got, want)) in played.iter().zip(written).enumerate() {
                assert_eq!(value_text(got), value_text(want), "{n}, call {i}");
            }
        }
        out.push(case_text(case, &played));
    }
    if update {
        std::fs::write(PATH, format!("[\n{}\n]\n", out.join(",\n"))).unwrap();
    }
}

fn case_text(case: &Value, calls: &[Value]) -> String {
    let get = |k: &str| value_text(case.get(k).unwrap());
    let lines: Vec<String> = calls
        .iter()
        .map(|c| format!("  {}", value_text(c)))
        .collect();
    format!(
        "{{\"name\":{},\n \"steps\":{},\n \"plan\":{},\n \"calls\":[\n{}\n ]}}",
        get("name"),
        get("steps"),
        get("plan"),
        lines.join(",\n")
    )
}

fn strs(items: impl IntoIterator<Item = String>) -> Value {
    Value::Arr(items.into_iter().map(Value::Str).collect())
}

fn with_expect(call: &Value, expect: Option<Value>) -> Value {
    let Value::Obj(kv) = call else {
        panic!("a call is an object")
    };
    let mut kv: Vec<(String, Value)> = kv.iter().filter(|(k, _)| k != "expect").cloned().collect();
    kv.extend(expect.map(|e| ("expect".to_string(), e)));
    Value::Obj(kv)
}

/// Replays the calls of a case on a fresh scheduler and returns them with the answers filled in.
fn play(case: &Value) -> Vec<Value> {
    let reg = registry_from(case.get("steps").unwrap());
    let mut s = Sched::new(&read(case.get("plan").unwrap()), &reg, "test-1", &[]);
    let calls = case.get("calls").unwrap().arr();
    calls
        .iter()
        .map(|call| with_expect(call, answer(&mut s, call)))
        .collect()
}

fn read(v: &Value) -> Plan {
    parse(value_text(v).as_bytes()).unwrap()
}

/// The calls that only act on the scheduler; false when `op` is not one of them.
fn act(s: &mut Sched, op: &str, call: &Value) -> bool {
    let job = || jid(call.get("job").and_then(Value::str).unwrap());
    match op {
        "resources" => {
            let res = call
                .get("res")
                .unwrap()
                .arr()
                .iter()
                .filter_map(Value::str)
                .map(str::to_string)
                .collect();
            s.set_resources(&job(), res);
        }
        "start" => assert!(s.started(&job())),
        "progress" => s.progress(&job(), call.get("frac").and_then(Value::f64).unwrap_or(0.0)),
        "cached" => assert!(s.cached(&job())),
        "pause" => s.pause(),
        "resume" => s.resume(),
        _ => return false,
    }
    true
}

/// Does one call on the scheduler and returns what the case records as its answer.
fn answer(s: &mut Sched, call: &Value) -> Option<Value> {
    let num = |k: &str, d: f64| call.get(k).and_then(Value::f64).unwrap_or(d);
    let op = call.get("op").and_then(Value::str).unwrap();
    if act(s, op, call) {
        return None;
    }
    match op {
        "next" => {
            let lim = Limits {
                slots: num("slots", 99.0) as usize,
                mem_mb: num("mem", 1e9),
            };
            let mb = call.get("mb");
            let est = |id: &JobId| {
                mb.and_then(|m| m.get(&name(id)))
                    .and_then(Value::f64)
                    .unwrap_or(1.0)
            };
            Some(strs(s.next(&lim, est).iter().map(|j| name(&j.id))))
        }
        "finish" => {
            let job = jid(call.get("job").and_then(Value::str).unwrap());
            let r = match call.get("error").and_then(Value::str) {
                Some(k) => Err(Error::new(leak(k))),
                None => Ok(Done { ms: num("ms", 1.0) }),
            };
            Some(Value::Str(
                match s.finished(&job, r) {
                    Landed::Accepted => "accepted",
                    Landed::Discarded { .. } => "discarded",
                    Landed::Unknown => "unknown",
                }
                .into(),
            ))
        }
        "update" => Some(strs(
            s.update(&read(call.get("plan").unwrap()))
                .iter()
                .map(|Command::Cancel(id)| name(id)),
        )),
        "states" => Some(strs(
            s.jobs()
                .iter()
                .map(|(id, st)| format!("{}={}", name(id), st.name())),
        )),
        "nodes" => Some(strs(s.nodes().iter().map(|n| {
            let frac = n.frac.map_or("?".to_string(), |f| format!("{f:.3}"));
            format!(
                "{}={} {}/{} {frac}",
                n.instance,
                n.state.name(),
                n.parte,
                n.di
            )
        }))),
        "events" => {
            let ev = json::parse(s.events_since(num("since", 0.0) as u64).as_bytes()).unwrap();
            if call.get("full").and_then(Value::bool) == Some(true) {
                return Some(ev);
            }
            let last = ev
                .arr()
                .last()
                .and_then(|e| e.get("seq"))
                .and_then(Value::f64)
                .unwrap_or(0.0);
            Some(Value::Arr(vec![
                Value::Num(ev.arr().len() as f64),
                Value::Num(last),
                Value::Num(s.rev() as f64),
            ]))
        }
        op => panic!("unknown op {op}"),
    }
}
