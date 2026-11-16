//! The meta of a job from its lineage text (`job_lineage_text`, the `rotte` `.wasm`, then `job_meta` of the feature `.wasm`) is the meta of the job read on the
//! plan itself (its step, what it writes, the layers of its line, as `flow_base.rs` carries them), over the branches, the preset and the gap filling of `PLAN`.

use super::branches::PLAN;
use crate::flow_run_tests::fixture;
use mzlab_core::error::Result;
use mzlab_core::flow::sched::{JobId, Sched};
use mzlab_core::flow::{parse, Ctx, Layer};
use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare;
use mzlab_feat::slabs::{rows_of, stima, Stored};
use mzlab_ops::run::feat::run;
use mzlab_ops::run::feat::run::meta::{
    job_layers_text, job_lineage_text, job_meta, meta_json, recipe, written, Meta,
};

struct Held(Vec<Vec<u8>>);

impl Stored for Held {
    fn count(&self) -> usize {
        self.0.len()
    }
    fn bytes(&mut self, k: usize) -> Result<Vec<u8>> {
        Ok(self.0[k].clone())
    }
}

const BUDGET_MB: f64 = 256.0;

/// The meta of `inst` read on the plan: `up` the instances of its line before it, in order.
fn on_plan(inst: &str, up: &[&str], idx: &[Vec<u8>], peaks: &[Vec<u8>]) -> String {
    let reg = crate::flow::steps();
    let plan = parse(PLAN.as_bytes()).unwrap();
    let sched = Sched::new(&plan, &reg, env!("CARGO_PKG_VERSION"), &[]);
    let ctx = Ctx::new(&plan, &reg, &sched).unwrap();
    let job = ctx.job(&JobId::new(inst, None), Vec::new());
    let layers: Vec<Layer> = up
        .iter()
        .map(|u| {
            let j = ctx.job(&JobId::new(u, None), Vec::new());
            (j.step.id.to_string(), written(&j))
        })
        .collect();
    let open = |k: usize| Index::open(idx[k].clone(), idx[k].len() as u64).unwrap();
    let detect = job.step.id == "features.detect";
    let base = if detect {
        run::recipe_for_estimated("", &mut open(1)).unwrap()
    } else {
        run::recipe_of_layers(&mut open(0), &layers).unwrap()
    };
    let (mut r, estimates) = recipe(&job, base).unwrap();
    r.fill_gaps = job.step.id == "features.gapfill";
    let mem = (!detect).then(|| {
        let way = stima(
            &rows_of(&mut Held(peaks.to_vec())).unwrap(),
            &r,
            (BUDGET_MB * 1048576.0) as usize,
        );
        (way.bytes as f64 / 1048576.0, way.via.name())
    });
    let preset = (detect && r.detect.axis.is_da()).then_some("lr");
    meta_json(&Meta {
        estimates,
        mem,
        preset,
    })
}

/// The meta of `inst` from its lineage text.
fn on_text(inst: &str, idx: &[Vec<u8>], peaks: &[Vec<u8>]) -> String {
    let reg = crate::flow::steps();
    let text = job_lineage_text(PLAN.as_bytes(), &reg, inst).unwrap();
    let open = |k: usize| Index::open(idx[k].clone(), idx[k].len() as u64).unwrap();
    let mut held = Held(peaks.to_vec());
    let m = job_meta(
        &text,
        (&mut open(0), &mut open(1)),
        Some(&mut held),
        BUDGET_MB,
    )
    .unwrap();
    meta_json(&m)
}

#[test]
fn the_meta_of_the_lineage_text_is_the_meta_of_the_plan() {
    let (dir, files) = fixture("lineage_meta");
    let idx: Vec<Vec<u8>> = files
        .iter()
        .map(|(name, b)| {
            let mut src = b.clone();
            let pr = prepare(&mut src, b.len() as u64, name).unwrap();
            build_mzml_to_vec(&pr.table, b, 64 << 20).unwrap().0
        })
        .collect();
    let reg = crate::flow::steps();
    let peaks = |det: &str| -> Vec<Vec<u8>> {
        let dt = job_layers_text(PLAN.as_bytes(), &reg, det, false).unwrap();
        (0..files.len())
            .map(|k| {
                let mut ix = Index::open(idx[k].clone(), idx[k].len() as u64).unwrap();
                let r = run::recipe_for(&dt, &mut ix).unwrap();
                run::detect(&mut ix, &files[k].0, &r).unwrap()
            })
            .collect()
    };
    let (pa, pb) = (peaks("deta"), peaks("detb"));
    for (inst, up, p) in [
        ("deta", &[][..], &pa),
        ("detb", &[], &pb),
        ("alia", &["deta"], &pa),
        ("alib", &["detb"], &pb),
        ("gapa", &["deta", "alia"], &pa),
    ] {
        let (want, got) = (on_plan(inst, up, &idx, p), on_text(inst, &idx, p));
        assert_eq!(got, want, "{inst}");
        assert!(
            got.contains("\"mem\":") && (up.is_empty() || !got.contains("\"mem\":null")),
            "{inst}: {got}"
        );
    }
    // the preset and the written values reach the meta: a detection of branch a estimates other values than one of branch b
    assert_ne!(on_text("deta", &idx, &pa), on_text("detb", &idx, &pb));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_lineage_text_keeps_the_order_the_plan_writes() {
    let reg = crate::flow::steps();
    let text = job_lineage_text(PLAN.as_bytes(), &reg, "gapa").unwrap();
    let l = mzlab_core::flow::lineage_from_text(&text).unwrap();
    assert_eq!(l.step, "features.gapfill");
    let steps: Vec<&str> = l.layers.iter().map(|x| x.0.as_str()).collect();
    assert_eq!(steps, ["features.detect", "features.align"], "{text}");
    assert!(!l.resolved.is_empty(), "{text}");
    assert!(mzlab_core::flow::lineage_from_text("{\"step\":1}").is_err());
}
