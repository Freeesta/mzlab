//! What a feature step of the Flow says of itself besides its bytes: the recipe it runs with (the one of the panel on the profile of the first file, then the
//! values the plan writes) and the values it estimated, with their proofs. `flow run` and the browser (`job_meta`) both ask it, so the record of a run
//! (`prov/1`) names the same estimates in both. Used by `mzlab-cli` (`flow_base.rs`) and the engine `.wasm` (`flow_job_meta`).

use crate::run::{recipe_for_estimated, recipe_of_layers, Layer};
use mzlab_core::error::{Error, Result};
use mzlab_core::flow::sched::{JobId, Sched};
use mzlab_core::flow::{parse, value_text, Ctx, Instance, Job, Plan};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::{Out, Value};
use mzlab_core::step::Step;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::slabs::{rows_of, stima, Stored};
use mzlab_feat::steps::recipe_map;

/// A value the plan leaves out: name, value, proof.
pub type Estimate = (String, Value, Value);

/// What the plan writes for `job`: the algorithm it chose, the values of its preset, then its own parameters (descriptor names).
pub fn written(job: &Job) -> Vec<(String, Value)> {
    let mut v: Vec<(String, Value)> = Vec::new();
    if let Some(a) = job
        .instance
        .algorithm
        .as_deref()
        .filter(|_| job.step.id == "features.detect")
    {
        v.push(("algorithm".to_string(), Value::Str(a.into())));
    }
    let preset = job.instance.preset.as_deref();
    let presets = job.step.presets.iter().filter(|p| Some(p.id) == preset);
    for (k, x) in presets.flat_map(|p| p.params.iter()) {
        v.push((k.to_string(), x.clone()));
    }
    for (k, x) in &job.instance.params {
        v.retain(|(n, _)| n != k);
        v.push((k.clone(), x.clone()));
    }
    v
}

/// The recipe of `job`: `base` with what the plan writes, and the values the plan leaves out that differ from the canonical ones (the estimates).
pub fn recipe(job: &Job, mut base: Recipe) -> Result<(Recipe, Vec<Estimate>)> {
    let w = written(job);
    recipe_map::apply(&mut base, job.step.id, &w)?;
    let resolved = &job.resolved.params;
    let proof = Value::Str("profile of the first file of the series".into());
    let estimates = recipe_map::values(&base, job.step.id)
        .into_iter()
        .filter(|(k, v)| {
            !w.iter().any(|x| x.0 == *k)
                && !job.resolved.noresult.contains(k)
                && resolved.iter().find(|x| x.0 == *k).map(|x| &x.1) != Some(v)
        })
        .map(|(k, v)| {
            let why = match (k.as_str(), base.min_height_basis) {
                ("min_height", Some((mode, sigma))) => Value::Obj(vec![
                    (
                        "basis".into(),
                        Value::Str(
                            "noise lump of the intensities of the first file of the series".into(),
                        ),
                    ),
                    (
                        "factor".into(),
                        Value::Num(mzlab_feat::params_height::MIN_HEIGHT_FACTOR),
                    ),
                    ("mode".into(), Value::Num(mode)),
                    ("sigma".into(), Value::Num(sigma)),
                ]),
                _ => proof.clone(),
            };
            (k, v, why)
        })
        .collect();
    Ok((base, estimates))
}

/// What a job says of itself: the values it estimated and, for the alignment and the gap filling, the memory it asks for and the way it takes.
pub struct Meta {
    pub estimates: Vec<Estimate>,
    pub mem: Option<(f64, &'static str)>,
}

const LAYERED: [&str; 3] = ["features.detect", "features.align", "features.gapfill"];

/// The parameters the feature steps in the line of `inst` wrote before it, in the order of the plan: what the lineage of its files carries in `flow run`.
fn upstream(ctx: &Ctx, plan: &Plan, inst: &Instance) -> Vec<Layer> {
    let mut line: Vec<&str> = Vec::new();
    let mut todo = inst.sources();
    while let Some(id) = todo.pop() {
        if let Some(i) = plan.steps.iter().find(|s| s.id == id) {
            if !line.contains(&id) {
                line.push(id);
                todo.extend(i.sources());
            }
        }
    }
    plan.steps
        .iter()
        .filter(|s| line.contains(&s.id.as_str()) && LAYERED.contains(&s.step.as_str()))
        .map(|s| {
            let job = ctx.job(&JobId::new(&s.id, None), Vec::new());
            (job.step.id.to_string(), written(&job))
        })
        .collect()
}

/// The meta of `instance` of the plan: `own` is the index of the file a detection is about, `first` the first index of the series, `stored` the `.mzfeatf` an
/// alignment reads; `budget_mb` the memory of the alignment. The same recipe as `flow run` builds (`recipe`), so the same estimates and memory.
pub fn job_meta<R: ReadAt>(
    plan_text: &[u8],
    registry: &[Step],
    instance: &str,
    (first, own): (&mut Index<R>, &mut Index<R>),
    stored: Option<&mut dyn Stored>,
    budget_mb: f64,
) -> Result<Meta> {
    let plan = parse(plan_text)?;
    let sched = Sched::new(&plan, registry, env!("CARGO_PKG_VERSION"), &[]);
    let ctx = Ctx::new(&plan, registry, &sched)?;
    let inst = plan
        .steps
        .iter()
        .find(|s| s.id == instance)
        .ok_or_else(|| Error::new("err.flow.lineage"))?;
    let job = ctx.job(&JobId::new(instance, None), Vec::new());
    let detect = job.step.id == "features.detect";
    let base = if detect {
        recipe_for_estimated("", own)?
    } else {
        recipe_of_layers(first, &upstream(&ctx, &plan, inst))?
    };
    let (mut recipe, estimates) = recipe(&job, base)?;
    let fill = job.step.id == "features.gapfill";
    recipe.fill_gaps = fill;
    let mem = match stored {
        Some(src) if !detect => {
            let way = stima(&rows_of(src)?, &recipe, (budget_mb * 1048576.0) as usize);
            Some((way.bytes as f64 / 1048576.0, way.via.name()))
        }
        _ => None,
    };
    Ok(Meta { estimates, mem })
}

/// `job_meta` as JSON: `{"estimates":[{"name","value","proof"}],"mem":{"mb","via"}|null}`.
pub fn meta_json(m: &Meta) -> String {
    let mut o = Out::default();
    o.raw("{\"estimates\":[");
    for (i, (name, value, proof)) in m.estimates.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).raw("{\"name\":").str(name);
        o.raw(",\"value\":").raw(&value_text(value));
        o.raw(",\"proof\":").raw(&value_text(proof)).raw("}");
    }
    o.raw("],\"mem\":");
    match m.mem {
        Some((mb, via)) => o.raw("{\"mb\":").num(mb).raw(",\"via\":").str(via).raw("}"),
        None => o.raw("null"),
    };
    o.raw("}").0.clone()
}

/// A sink that keeps nothing: `Hashing` over it only hashes what passes.
struct Discard;

impl mzlab_core::budget::WriteAt for Discard {
    fn write_at(&mut self, _off: u64, _data: &[u8]) -> Result<()> {
        Ok(())
    }
}

/// SHA-256 (hex) of the `size` bytes `r` holds, read in pieces of 1 MiB so that a big index is never loaded whole (the browser hashes the files of a run this way).
pub fn file_sha<R: ReadAt>(r: &mut R, size: u64) -> Result<String> {
    use mzlab_core::budget::{Hashing, WriteAt};
    let (mut out, mut buf, mut at) = (Hashing::new(Discard), vec![0u8; 1 << 20], 0u64);
    while at < size {
        let n = (size - at).min(buf.len() as u64) as usize;
        r.read_at(at, &mut buf[..n])?;
        out.write_at(at, &buf[..n])?;
        at += n as u64;
    }
    Ok(out.finish().2)
}

#[cfg(feature = "detect")]
mod export {
    use js_sys::{Array, Function};
    use wasm_bindgen::prelude::*;

    /// `{"bytes","sha256"}` of file 0 of `read(0, offset, length)` (`file_sha`).
    #[wasm_bindgen]
    pub fn flow_file_sha(read: Function, size: f64) -> std::result::Result<String, JsValue> {
        let mut r = crate::JsReader { read, k: 0 };
        let sha = super::file_sha(&mut r, size as u64).map_err(crate::js_err)?;
        Ok(format!("{{\"bytes\":{size},\"sha256\":\"{sha}\"}}"))
    }

    /// `{"bytes","sha256"}` of the source file an index (file 0 of `read`) was made from: what the plan names as input in the record of a run.
    #[wasm_bindgen]
    pub fn flow_idx_source(read: Function, size: f64) -> std::result::Result<String, JsValue> {
        let ix = crate::open_one(&read, 0, size).map_err(crate::js_err)?;
        let bytes = ix.source.get("size").and_then(|v| v.f64()).unwrap_or(0.0);
        let sha = ix.source.get("sha256").and_then(|v| v.str()).unwrap_or("");
        Ok(format!("{{\"bytes\":{bytes},\"sha256\":\"{sha}\"}}"))
    }

    /// The meta of `instance` (`job_meta`, JSON): `plan` is the `mzflow/2` text, `registry` the `step/1` array. `read(k, offset, length)` reads index 0 (the
    /// first of the series) and 1 (the file of a detection), `sizes` their sizes; `files` the `.mzfeatf` of an alignment (empty for a detection).
    #[wasm_bindgen]
    pub fn flow_job_meta(
        plan: &str,
        registry: &str,
        instance: &str,
        read: Function,
        sizes: Vec<f64>,
        files: Array,
        budget_mb: f64,
    ) -> std::result::Result<String, JsValue> {
        let reg = mzlab_core::json::parse(registry.as_bytes())
            .and_then(|v| mzlab_core::step::registry_from_value(&v))
            .map_err(crate::js_err)?;
        let mut first = crate::open_one(&read, 0, sizes[0]).map_err(crate::js_err)?;
        let mut own = crate::open_one(&read, 1, sizes[1]).map_err(crate::js_err)?;
        let mut src = crate::JsStored { files };
        let m = super::job_meta(
            plan.as_bytes(),
            &reg,
            instance,
            (&mut first, &mut own),
            Some(&mut src),
            budget_mb,
        )
        .map_err(crate::js_err)?;
        Ok(super::meta_json(&m))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Bytes(Vec<u8>);

    impl ReadAt for Bytes {
        fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
            buf.copy_from_slice(&self.0[off as usize..off as usize + buf.len()]);
            Ok(())
        }
    }

    #[test]
    fn the_sha_of_a_file_read_in_pieces_is_the_sha_of_its_bytes() {
        let data: Vec<u8> = (0..(2 << 20) + 17).map(|i| (i * 31 % 251) as u8).collect();
        let want = mzlab_core::budget::sha256_hex(&data);
        let size = data.len() as u64;
        assert_eq!(file_sha(&mut Bytes(data), size).unwrap(), want);
    }
}
