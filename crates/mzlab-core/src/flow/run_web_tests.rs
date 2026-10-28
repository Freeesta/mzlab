//! `record_jobs`: the reports of jobs run elsewhere give the record of `execute` on the same jobs, byte for byte, with labels and failures as the runner names them.

use super::*;
use crate::flow::{execute, prov_json, Job, Runner};
use crate::json::Out;
use crate::step::Port;
use std::sync::Mutex;

fn leak(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

fn step(id: &str, per_item: bool, ins: &str, outs: &str) -> Step {
    let many = |n: &str| {
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
    s.ins = vec![many(ins)];
    s.outs = vec![many(outs)];
    s
}

fn registry() -> Vec<Step> {
    vec![
        step("t.index", true, "raw*", "index*"),
        step("t.table", false, "index*", "table"),
    ]
}

const FILES: [&str; 3] = ["a.mzML", "b.mzML", "c.mzML"];

fn text() -> String {
    let inputs: Vec<String> = FILES
        .iter()
        .map(|f| format!("{{\"name\":\"{f}\",\"sha256\":\"sha-{f}\",\"bytes\":9}}"))
        .collect();
    let refs: Vec<String> = FILES.iter().map(|f| format!("\"file:{f}\"")).collect();
    format!(
        "{{\"format\":\"mzflow/2\",\"inputs\":[{}],\"steps\":[\
{{\"id\":\"idx\",\"step\":\"t.index@1\",\"in\":{{\"raw\":[{}]}}}},\
{{\"id\":\"tab\",\"step\":\"t.table@1\",\"in\":{{\"index\":[\"idx.index\"]}}}}]}}",
        inputs.join(","),
        refs.join(",")
    )
}

/// Makes each output from what the job reads and writes the report of the job as the page does; the second step fails when `tab_fails`.
struct Reporting {
    reports: Mutex<Vec<String>>,
    tab_fails: bool,
}

impl Runner for Reporting {
    fn run(&self, job: &Job) -> Result<Outcome> {
        let id = &job.instance.id;
        let mut o = Out::default();
        o.raw("{\"instance\":").str(id).raw(",\"item\":");
        match &job.item_name {
            Some(n) => o.str(n),
            None => o.raw("null"),
        };
        if job.instance.id == "tab" && self.tab_fails {
            o.raw(",\"error_key\":\"err.flow.sample\"}");
            self.reports.lock().unwrap().push(o.0.clone());
            return Err(Error::new("err.flow.sample"));
        }
        let seen: Vec<&str> = job
            .inputs
            .iter()
            .flat_map(|(_, v)| v.iter().map(|a| a.sha256.as_str()))
            .collect();
        let sha = crate::budget::sha256_hex(format!("{id}|{}", seen.join(",")).as_bytes());
        let out = &job.step.outs[0];
        let (ext, ty) = if *id == "idx" {
            ("mzidx", DataType::Mzidx)
        } else {
            ("mzfeat", DataType::Table)
        };
        let label = match &job.item_root {
            Some(r) => format!("{id}-{r}.{ext}"),
            None => format!("{id}.{ext}"),
        };
        o.raw(",\"outputs\":[{\"port\":").str(out.name);
        o.raw(",\"ext\":").str(ext).raw(",\"type\":").str(ty.name());
        o.raw(",\"sha256\":").str(&sha).raw(",\"bytes\":42}],");
        o.raw("\"estimates\":[{\"name\":\"ppm\",\"value\":5,\"proof\":\"profile\"}],");
        o.raw("\"ms\":0.5,\"mem\":{\"mb\":3.9,\"via\":\"memory\"},\"cached\":false}");
        self.reports.lock().unwrap().push(o.0.clone());
        let a = Artifact {
            sha256: sha,
            bytes: 42,
            ty,
            label: Some(label),
        };
        Ok(Outcome {
            outputs: vec![(out.name.to_string(), a)],
            estimates: vec![("ppm".into(), Value::Num(5.0), Value::Str("profile".into()))],
            ms: 0.5,
            mem: Some(Mem {
                mb: 3.9,
                via: "memory",
            }),
            ..Outcome::default()
        })
    }
}

fn agent() -> Agent {
    Agent {
        name: "mzLab".into(),
        version: "x".into(),
        commit: None,
        platform: "native",
    }
}

fn both(tab_fails: bool) -> (String, String) {
    let text = text();
    let r = Reporting {
        reports: Mutex::new(Vec::new()),
        tab_fails,
    };
    let native = execute(&parse(text.as_bytes()).unwrap(), &registry(), &r, 1).unwrap();
    // reports arrive in any order
    let mut reports = r.reports.into_inner().unwrap();
    reports.reverse();
    let (web, _) = record_jobs(
        text.as_bytes(),
        &registry(),
        &format!("[{}]", reports.join(",")),
    )
    .unwrap();
    (
        prov_json(&agent(), Some("p"), &native),
        prov_json(&agent(), Some("p"), &web),
    )
}

#[test]
fn the_reports_of_the_jobs_give_the_record_of_execute() {
    let (native, web) = both(false);
    assert!(
        native.contains("idx-a.mzidx") && native.contains("\"mem_mb\":3.9"),
        "{native}"
    );
    assert_eq!(web, native);
}

#[test]
fn a_failed_job_and_a_missing_report_are_recorded_as_in_execute() {
    let (native, web) = both(true);
    assert!(native.contains("err.flow.sample"), "{native}");
    assert_eq!(web, native);
    let (none, _) = record_jobs(text().as_bytes(), &registry(), "[]").unwrap();
    assert!(prov_json(&agent(), None, &none).contains("err.flow.worker"));
}

#[test]
fn the_route_refuses_what_it_cannot_read() {
    let r = Reporting {
        reports: Mutex::new(Vec::new()),
        tab_fails: false,
    };
    execute(&parse(text().as_bytes()).unwrap(), &registry(), &r, 1).unwrap();
    let jobs = format!("[{}]", r.reports.into_inner().unwrap().join(","));
    let mut reg = Out::default();
    reg.raw("[]");
    let body = format!(
        "{{\"plan\":{},\"registry\":{},\"jobs\":{jobs}}}",
        Out::default().str(&text()).0,
        reg.0
    );
    // an unknown registry is an error, not a record
    assert!(route("prov", &body, &agent()).is_err());
    assert!(route("nothing", "{}", &agent()).is_err());
}
