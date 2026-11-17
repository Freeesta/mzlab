//! What a feature step of the Flow says of itself besides its bytes: the recipe it runs with (the one of the panel on the profile of the first file, then the
//! values the plan writes) and the values it estimated, with their proofs. `flow run` and the browser (`job_meta`) both ask it, so the record of a run
//! (`prov/1`) names the same estimates in both. The lineage it reads comes from `mzlab_core::flow::lineage` (the `rotte` `.wasm` in the browser). Used by
//! `mzlab-cli` (`flow_base.rs`) and the feature `.wasm` (`flow_job_meta`).

use super::{recipe_for_estimated, recipe_of_layers};
use mzlab_core::error::Result;
use mzlab_core::flow::{lineage_from_text, value_text, Job};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::{Out, Value};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::slabs::{rows_of, stima, Stored};
use mzlab_feat::steps::recipe_map;

pub use mzlab_core::flow::{job_layers_text, job_lineage_text, written};

/// A value the plan leaves out: name, value, proof.
pub type Estimate = (String, Value, Value);

/// The recipe of `job`: `base` with what the plan writes, and the values the plan leaves out that differ from the canonical ones (the estimates).
pub fn recipe(job: &Job, base: Recipe) -> Result<(Recipe, Vec<Estimate>)> {
    let r = &job.resolved;
    recipe_of(job.step.id, &written(job), (&r.params, &r.noresult), base)
}

/// `recipe` of a step `step` with what the plan writes `w`, its canonical values and the names that do not change the result.
fn recipe_of(
    step: &str,
    w: &[(String, Value)],
    (resolved, noresult): (&[(String, Value)], &[String]),
    mut base: Recipe,
) -> Result<(Recipe, Vec<Estimate>)> {
    recipe_map::apply(&mut base, step, w)?;
    let proof = Value::Str("profile of the first file of the series".into());
    let estimates = recipe_map::values(&base, step)
        .into_iter()
        .filter(|(k, v)| {
            !w.iter().any(|x| x.0 == *k)
                && !noresult.contains(k)
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
    /// The preset the files decided (`lr` for a detection on unit-resolution files), as `flow run` records it.
    pub preset: Option<&'static str>,
}

/// The meta of the job of `lineage` (the text of `job_lineage_text`): `own` is the index of the file a detection is about, `first` the first index of the series,
/// `stored` the `.mzfeatf` an alignment reads; `budget_mb` the memory of the alignment. The same recipe as `flow run` builds (`recipe`), so the same estimates
/// and memory.
pub fn job_meta<R: ReadAt>(
    lineage: &str,
    (first, own): (&mut Index<R>, &mut Index<R>),
    stored: Option<&mut dyn Stored>,
    budget_mb: f64,
) -> Result<Meta> {
    let l = lineage_from_text(lineage)?;
    let detect = l.step == "features.detect";
    let base = if detect {
        recipe_for_estimated("", own)?
    } else {
        recipe_of_layers(first, &l.layers)?
    };
    let (mut recipe, estimates) = recipe_of(&l.step, &l.params, (&l.resolved, &l.noresult), base)?;
    recipe.fill_gaps = l.step == "features.gapfill";
    let mem = match stored {
        Some(src) if !detect => {
            let way = stima(&rows_of(src)?, &recipe, (budget_mb * 1048576.0) as usize);
            Some((way.bytes as f64 / 1048576.0, way.via.name()))
        }
        _ => None,
    };
    let preset = (detect && recipe.detect.axis.is_da()).then_some("lr");
    Ok(Meta {
        estimates,
        mem,
        preset,
    })
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

/// `job_meta` as JSON: `{"estimates":[{"name","value","proof"}],"mem":{"mb","via"}|null,"preset":"lr"?}`.
pub fn meta_json(m: &Meta) -> String {
    let mut o = Out::default();
    o.raw("{\"estimates\":[");
    for (i, (name, value, proof)) in m.estimates.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"name\":")
            .str(name);
        o.raw(",\"value\":").raw(&value_text(value));
        o.raw(",\"proof\":").raw(&value_text(proof)).raw("}");
    }
    o.raw("],\"mem\":");
    match m.mem {
        Some((mb, via)) => o.raw("{\"mb\":").num(mb).raw(",\"via\":").str(via).raw("}"),
        None => o.raw("null"),
    };
    if let Some(p) = m.preset {
        o.raw(",\"preset\":").str(p);
    }
    o.raw("}").0.clone()
}

#[cfg(test)]
#[path = "run_meta_tests.rs"]
mod tests;
