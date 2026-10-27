//! The record the browser writes: the reports of the jobs of the base chain (the files of a native run, the estimates and the memory of `job_meta`) give, through
//! the route `record_prov`/`record_corsa`, the `prov.json` and `corsa.json` of `flow run` byte for byte, apart from the times and the heap.

use crate::flow_run_tests::{fixture, flow_run, p, untimed};
use mzlab_core::budget::sha256_hex;
use mzlab_core::error::Result;
use mzlab_core::idxread::Index;
use mzlab_core::json::{Out, Value};
use mzlab_core::step::json::registry_json;
use mzlab_feat::slabs::Stored;
use mzlab_feat_wasm::run_meta::{job_meta, meta_json};
use std::path::Path;

/// The stored files of a series held in memory.
struct Held(Vec<Vec<u8>>);

impl Stored for Held {
    fn count(&self) -> usize {
        self.0.len()
    }
    fn bytes(&mut self, k: usize) -> Result<Vec<u8>> {
        Ok(self.0[k].clone())
    }
}

fn open(out: &Path, name: &str) -> Index<std::fs::File> {
    let f = std::fs::File::open(out.join(name)).unwrap();
    let size = f.metadata().unwrap().len();
    Index::open(f, size).unwrap()
}

/// The report of a job as the page writes it: the file it made and, for a feature step, the meta.
fn report(out: &Path, (instance, item): (&str, Option<&str>), (port, file, ty): (&str, &str, &str), meta: Option<String>) -> String {
    let bytes = std::fs::read(out.join(file)).unwrap();
    let mut o = Out::default();
    o.raw("{\"instance\":").str(instance).raw(",\"item\":");
    match item {
        Some(n) => o.str(n),
        None => o.raw("null"),
    };
    o.raw(",\"outputs\":[{\"port\":").str(port).raw(",\"label\":").str(file);
    o.raw(",\"type\":").str(ty).raw(",\"sha256\":").str(&sha256_hex(&bytes));
    o.raw(",\"bytes\":").int(bytes.len() as i64).raw("}],\"ms\":1");
    if let Some(m) = meta {
        let v = mzlab_core::json::parse(m.as_bytes()).unwrap();
        o.raw(",\"estimates\":").raw(&mzlab_core::flow::value_text(v.get("estimates").unwrap()));
        o.raw(",\"mem\":").raw(&mzlab_core::flow::value_text(v.get("mem").unwrap()));
    }
    o.raw("}").0.clone()
}

#[test]
fn the_reports_of_the_base_chain_give_the_prov_and_the_corsa_of_flow_run() {
    let (dir, _) = fixture("record");
    let out = dir.join("out");
    let o = flow_run(&[p(&dir.join("plan.json")), "--in", p(&dir.join("data")), "--out", p(&out)]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let plan = std::fs::read_to_string(out.join("plan.json")).unwrap();
    let registry = registry_json(&crate::flow::steps());
    let budget = mzlab_core::budget_auto::native_auto_mb();
    let roots = ["s_t000", "s_t010", "s_t030"];
    let (mut jobs, mut peaks) = (Vec::new(), Vec::new());
    for r in roots {
        let item = format!("{r}.mzML");
        jobs.push(report(&out, ("idx", Some(&item)), ("index", &format!("idx-{r}.mzidx"), "mzidx"), None));
        let (mut first, mut own) = (open(&out, "idx-s_t000.mzidx"), open(&out, &format!("idx-{r}.mzidx")));
        let reg = mzlab_core::step::registry_from_value(&mzlab_core::json::parse(registry.as_bytes()).unwrap()).unwrap();
        let m = job_meta(plan.as_bytes(), &reg, "det", (&mut first, &mut own), None, budget).unwrap();
        jobs.push(report(&out, ("det", Some(&item)), ("peaks", &format!("det-{r}.mzfeatf"), "peaks"), Some(meta_json(&m))));
        peaks.push(std::fs::read(out.join(format!("det-{r}.mzfeatf"))).unwrap());
    }
    let reg = mzlab_core::step::registry_from_value(&mzlab_core::json::parse(registry.as_bytes()).unwrap()).unwrap();
    for (id, file, port) in [("ali", "ali.mzfeat", "table"), ("gap", "gap.mzfeat", "table")] {
        let (mut first, mut own) = (open(&out, "idx-s_t000.mzidx"), open(&out, "idx-s_t000.mzidx"));
        let mut held = Held(peaks.clone());
        let m = job_meta(plan.as_bytes(), &reg, id, (&mut first, &mut own), Some(&mut held), budget).unwrap();
        jobs.push(report(&out, (id, None), (port, file, "table"), Some(meta_json(&m))));
    }
    let mut body = Out::default();
    body.raw("{\"plan\":").str(&plan).raw(",\"registry\":").raw(&registry);
    body.raw(",\"jobs\":[").raw(&jobs.join(",")).raw("]}");
    let prov = mzlab_core::flow::sched_route("record_prov", &body.0).unwrap();
    let native = std::fs::read_to_string(out.join("prov.json")).unwrap();
    assert_eq!(untimed(&prov), untimed(&native));
    let mut c = Out::default();
    c.raw("{\"prov\":").str(&prov).raw(",\"uscite\":[]}");
    let corsa = mzlab_core::flow::sched_route("record_corsa", &c.0).unwrap();
    let native = std::fs::read_to_string(out.join("corsa.json")).unwrap();
    assert_eq!(untimed(&corsa), untimed(&native));
    assert!(matches!(mzlab_core::json::parse(prov.as_bytes()), Ok(Value::Obj(_))));
}
