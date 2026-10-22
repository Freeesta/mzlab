//! `execute` over the scheduler: the record is the same for any seed of job duration and any number of workers, a file moves on before the others are
//! through the step before, and the outputs of a per-file step are keyed by the name of the input.

use super::*;
use crate::budget::sha256_hex;
use crate::flow::{parse, prov_json, Agent};
use crate::step::{Port, Step};
use std::sync::Mutex;

fn leak(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

fn step(id: &str, per_item: bool, ins: &str, outs: &str) -> Step {
    let ports = |n: &str| {
        let p = Port::new(leak(n.trim_end_matches('*')), DataType::Raw, "");
        if n.ends_with('*') {
            p.many()
        } else {
            p
        }
    };
    let mut s = Step::core(leak(id), "", "");
    s.per_item = per_item;
    s.runs_in.native = true;
    s.ins = vec![ports(ins)];
    s.outs = vec![ports(outs)];
    s
}

fn registry() -> Vec<Step> {
    vec![
        step("t.convert", true, "raw*", "mz*"),
        step("t.index", true, "mz*", "index*"),
        step("t.table", false, "index*", "table"),
    ]
}

const FILES: [&str; 3] = ["a.raw", "b.raw", "c.raw"];

fn plan() -> Plan {
    let inputs: Vec<String> = FILES
        .iter()
        .map(|f| format!("{{\"name\":\"{f}\",\"sha256\":\"sha-{f}\"}}"))
        .collect();
    let refs: Vec<String> = FILES.iter().map(|f| format!("\"file:{f}\"")).collect();
    let text = format!(
        "{{\"format\":\"mzflow/2\",\"inputs\":[{}],\"steps\":[\
{{\"id\":\"conv\",\"step\":\"t.convert@1\",\"in\":{{\"raw\":[{}]}}}},\
{{\"id\":\"idx\",\"step\":\"t.index@1\",\"in\":{{\"mz\":[\"conv.mz\"]}}}},\
{{\"id\":\"tab\",\"step\":\"t.table@1\",\"in\":{{\"index\":[\"idx.index\"]}}}}]}}",
        inputs.join(","),
        refs.join(",")
    );
    parse(text.as_bytes()).unwrap()
}

/// Makes every output from the hash of what it reads, takes a time that depends on the seed and the job, and writes down the order in which jobs began.
struct Seeded {
    seed: u64,
    began: Mutex<Vec<String>>,
}

impl Runner for Seeded {
    fn run(&self, job: &Job) -> Result<Outcome> {
        let who = format!(
            "{}|{}",
            job.instance.id,
            job.item_name.as_deref().unwrap_or("")
        );
        self.began.lock().unwrap().push(who.clone());
        let pause = sha256_hex(format!("{}{who}", self.seed).as_bytes());
        let micros = u64::from_str_radix(&pause[..3], 16).unwrap_or(0);
        std::thread::sleep(std::time::Duration::from_micros(micros));
        let seen: Vec<&str> = job
            .inputs
            .iter()
            .flat_map(|(_, v)| v.iter().map(|a| a.sha256.as_str()))
            .collect();
        let text = format!("{}|{}", job.instance.step_ref(), seen.join(","));
        let outputs = job
            .step
            .outs
            .iter()
            .map(|o| {
                let a = Artifact {
                    sha256: sha256_hex(format!("{text}|{}", o.name).as_bytes()),
                    bytes: text.len() as u64,
                    ty: o.ty,
                    label: None,
                };
                (o.name.to_string(), a)
            })
            .collect();
        Ok(Outcome {
            outputs,
            ms: 0.5,
            ..Outcome::default()
        })
    }
}

fn record(seed: u64, workers: usize) -> (String, Vec<String>) {
    let r = Seeded {
        seed,
        began: Mutex::new(Vec::new()),
    };
    let mut run = execute(&plan(), &registry(), &r, workers).unwrap();
    for a in &mut run.activities {
        a.ms = 0.0;
        a.heap_peak_bytes = None;
    }
    let agent = Agent {
        name: "mzLab".into(),
        version: "x".into(),
        commit: None,
        platform: "native",
    };
    (
        prov_json(&agent, Some("p"), &run),
        r.began.into_inner().unwrap(),
    )
}

#[test]
fn any_seed_and_any_number_of_workers_give_the_same_record() {
    let (base, _) = record(0, 1);
    for seed in 0..10 {
        for workers in [1, 4] {
            assert_eq!(
                record(seed, workers).0,
                base,
                "seed {seed}, {workers} workers"
            );
        }
    }
}

#[test]
fn a_file_moves_on_before_the_others_are_through_the_step_before() {
    let (_, began) = record(3, 1);
    let at = |s: &str| began.iter().position(|x| x == s).unwrap();
    assert!(at("idx|a.raw") < at("conv|c.raw"), "{began:?}");
    assert!(at("tab|") > at("idx|c.raw"));
}

#[test]
fn outputs_are_keyed_by_the_name_of_the_input() {
    let r = Seeded {
        seed: 0,
        began: Mutex::new(Vec::new()),
    };
    let run = execute(&plan(), &registry(), &r, 2).unwrap();
    assert_eq!(
        run.activities[0].generated,
        ["conv.mz[a.raw]", "conv.mz[b.raw]", "conv.mz[c.raw]"]
    );
    assert_eq!(
        run.activities[2].used,
        ["idx.index[a.raw]", "idx.index[b.raw]", "idx.index[c.raw]"]
    );
}
