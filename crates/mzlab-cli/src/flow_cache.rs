//! The cache of a run folder: what each job made, under its `flow::cache_key` (inputs by SHA-256, step, algorithm, canonical parameters) plus the
//! instance and the version of the program (the file of an item is in its inputs). A job whose key is in `cache/index.json` and whose files are still there with the same SHA-256 is
//! not run again: its outcome says `cached`. Used by the `FileRunner` of `flow.rs` (`flow run`, the route `flow/run` of the desktop app).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use mzlab_core::error::Result;
use mzlab_core::flow::{cache_key, value_text, Artifact, Job};
use mzlab_core::json::{parse, Value};
use mzlab_core::step::DataType;

use crate::flow::{io, FileRunner, Lineage, Made};

/// One job as it was made: its output file and what the next steps need of it.
#[derive(Clone)]
struct Entry {
    port: String,
    file: String,
    sha256: String,
    ty: DataType,
    estimates: Vec<(String, Value, Value)>,
    warnings: Vec<String>,
    said: Option<Value>,
    lineage: Option<Lineage>,
    /// The second output, when the step has one: (port, file, SHA-256, type).
    extra: Option<(String, String, String, DataType)>,
}

/// The second output of a cached job, kept again and checked against its SHA-256: `Some(None)` when the entry has none, `None` when a missing or changed file fails the whole entry.
fn take_extra(
    r: &FileRunner,
    job: &Job,
    work: &Path,
    extra: &Option<(String, String, String, DataType)>,
) -> Option<Option<(&'static str, Artifact)>> {
    let Some((port, file, sha, ty)) = extra else {
        return Some(None);
    };
    let a = r
        .keep(work.join(file), *ty)
        .ok()
        .filter(|a| a.sha256 == *sha)?;
    Some(Some((
        job.step.outs.iter().find(|p| p.name == *port)?.name,
        a,
    )))
}

/// Deletes the files of a cache entry that is no longer used.
fn forget(work: &Path, e: &Entry) {
    for file in std::iter::once(&e.file).chain(e.extra.iter().map(|x| &x.1)) {
        let _ = std::fs::remove_file(work.join(file));
    }
}

fn s(v: &str) -> Value {
    Value::Str(v.into())
}

/// The entries read from the folder and the ones of this run (taken again or made).
pub(crate) struct Cache {
    work: PathBuf,
    old: HashMap<String, Entry>,
    now: Mutex<Vec<(String, Entry)>>,
}

impl Cache {
    /// The cache of `work`; `fresh` forgets what is there (every job runs, the index is written again).
    pub fn open(work: &Path, fresh: bool) -> Cache {
        let text = std::fs::read(index_path(work)).ok().filter(|_| !fresh);
        let entries = text.and_then(|t| parse(&t).ok());
        let old = entries.map_or_else(HashMap::new, |v| {
            let list = v.get("entries").map(Value::arr).unwrap_or_default();
            list.iter().filter_map(|e| read_entry(work, e)).collect()
        });
        Cache {
            work: work.to_path_buf(),
            old,
            now: Mutex::new(Vec::new()),
        }
    }

    /// The key of `job`.
    pub fn key(job: &Job) -> String {
        let inputs: Vec<String> = job
            .inputs
            .iter()
            .flat_map(|(port, v)| v.iter().map(move |a| format!("{port}={}", a.sha256)))
            .collect();
        let who = format!(
            "{} {} {} {}",
            job.instance.step_ref(),
            job.instance.id,
            env!("CARGO_PKG_VERSION"),
            option_env!("MZLAB_COMMIT").unwrap_or("")
        );
        let algorithm = job
            .resolved
            .algorithm
            .as_ref()
            .map(|(id, v)| format!("{id}@{v}"));
        cache_key(
            &inputs,
            &who,
            algorithm.as_deref(),
            &job.resolved.params_sha256(),
        )
    }

    /// What the cache holds for `key`, when its file is still the one it made; the file is registered in `r` with its lineage.
    pub fn take(&self, r: &FileRunner, job: &Job, key: &str) -> Option<Made> {
        let e = self.old.get(key)?;
        let a: Artifact = r.keep(self.work.join(&e.file), e.ty).ok()?;
        if a.sha256 != e.sha256 {
            return None;
        }
        if let Some(l) = &e.lineage {
            // the position of the file in the plan may have changed since the entry was made
            let mut l = l.clone();
            l.item = job.item;
            r.trace(&a, l);
        }
        let port = job.step.outs.iter().find(|p| p.name == e.port)?.name;
        self.now
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push((key.to_string(), e.clone()));
        Some(Made {
            port,
            artifact: a,
            estimates: e.estimates.clone(),
            warnings: e.warnings.clone(),
            file: e.said.clone(),
            mem: None,
            extra: take_extra(r, job, &self.work, &e.extra)?,
        })
    }
}

/// Where the index of the cache of `work` is.
fn index_path(work: &Path) -> PathBuf {
    work.join("cache").join("index.json")
}

impl Cache {
    /// Keeps what `made` is under `key`.
    pub fn put(&self, key: String, made: &Made, lineage: Option<Lineage>) {
        let file = made.artifact.label.clone().unwrap_or_default();
        let e = Entry {
            port: made.port.to_string(),
            file,
            sha256: made.artifact.sha256.clone(),
            ty: made.artifact.ty,
            estimates: made.estimates.clone(),
            warnings: made.warnings.clone(),
            said: made.file.clone(),
            lineage,
            extra: made.extra.as_ref().map(|(p, a)| {
                (
                    p.to_string(),
                    a.label.clone().unwrap_or_default(),
                    a.sha256.clone(),
                    a.ty,
                )
            }),
        };
        self.now
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push((key, e));
    }

    /// Writes the index of this run; the files of the old entries this run did not use are removed (they were made by an earlier run here).
    pub fn save(&self) -> Result<()> {
        let mut now = self.now.lock().unwrap_or_else(|p| p.into_inner()).clone();
        mzlab_core::sortc::slice_by(&mut now, |a, b| a.0.cmp(&b.0));
        for (k, e) in &self.old {
            if !now.iter().any(|(n, x)| n == k || x.file == e.file) {
                forget(&self.work, e);
            }
        }
        let entries: Vec<Value> = now
            .iter()
            .map(|(k, e)| write_entry(&self.work, k, e))
            .collect();
        let doc = Value::Obj(vec![
            ("format".into(), s("mzflow-cache/1")),
            ("entries".into(), Value::Arr(entries)),
        ]);
        let path = index_path(&self.work);
        std::fs::create_dir_all(path.parent().unwrap_or(&self.work)).map_err(io)?;
        std::fs::write(path, value_text(&doc)).map_err(io)
    }
}

/// A path of the run folder as written in the index: relative to the folder.
fn rel(work: &Path, p: &Path) -> Value {
    s(&p.strip_prefix(work).unwrap_or(p).to_string_lossy())
}

fn paths(work: &Path, v: Option<&Value>) -> Vec<PathBuf> {
    v.map(Value::arr)
        .unwrap_or_default()
        .iter()
        .filter_map(Value::str)
        .map(|p| work.join(p))
        .collect()
}

fn pairs(v: &[(String, Value)]) -> Value {
    Value::Obj(v.to_vec())
}

fn write_lineage(work: &Path, l: &Lineage) -> Value {
    let layers = l
        .layers
        .iter()
        .map(|(step, p)| Value::Arr(vec![s(step), pairs(p)]))
        .collect();
    let list = |v: &[PathBuf]| Value::Arr(v.iter().map(|p| rel(work, p)).collect());
    Value::Obj(vec![
        ("name".into(), s(&l.name)),
        ("instance".into(), s(&l.instance)),
        (
            "item".into(),
            l.item.map_or(Value::Null, |k| Value::Num(k as f64)),
        ),
        (
            "first".into(),
            l.first.as_ref().map_or(Value::Null, |p| rel(work, p)),
        ),
        ("layers".into(), Value::Arr(layers)),
        ("index".into(), list(&l.index)),
        ("peaks".into(), list(&l.peaks)),
    ])
}

fn read_lineage(work: &Path, v: &Value) -> Lineage {
    let text = |k: &str| v.get(k).and_then(Value::str).unwrap_or("").to_string();
    let layers = v.get("layers").map(Value::arr).unwrap_or_default();
    let layer = |x: &Value| {
        let step = x.at(0).and_then(Value::str)?.to_string();
        match x.at(1)? {
            Value::Obj(kv) => Some((step, kv.clone())),
            _ => None,
        }
    };
    Lineage {
        name: text("name"),
        instance: text("instance"),
        item: v.get("item").and_then(Value::f64).map(|k| k as usize),
        first: v.get("first").and_then(Value::str).map(|p| work.join(p)),
        layers: layers.iter().filter_map(layer).collect(),
        index: paths(work, v.get("index")),
        peaks: paths(work, v.get("peaks")),
    }
}

fn write_entry(work: &Path, key: &str, e: &Entry) -> Value {
    let estimates = e
        .estimates
        .iter()
        .map(|(n, v, p)| Value::Arr(vec![s(n), v.clone(), p.clone()]))
        .collect();
    Value::Obj(vec![
        ("key".into(), s(key)),
        ("port".into(), s(&e.port)),
        ("file".into(), s(&e.file)),
        ("sha256".into(), s(&e.sha256)),
        ("type".into(), s(e.ty.name())),
        ("estimates".into(), Value::Arr(estimates)),
        (
            "warnings".into(),
            Value::Arr(e.warnings.iter().map(|w| s(w)).collect()),
        ),
        ("file_info".into(), e.said.clone().unwrap_or(Value::Null)),
        (
            "extra".into(),
            e.extra.as_ref().map_or(Value::Null, |(p, f, h, t)| {
                Value::Obj(vec![
                    ("port".into(), s(p)),
                    ("file".into(), s(f)),
                    ("sha256".into(), s(h)),
                    ("type".into(), s(t.name())),
                ])
            }),
        ),
        (
            "lineage".into(),
            e.lineage
                .as_ref()
                .map_or(Value::Null, |l| write_lineage(work, l)),
        ),
    ])
}

fn read_entry(work: &Path, v: &Value) -> Option<(String, Entry)> {
    let text = |k: &str| v.get(k).and_then(Value::str).map(str::to_string);
    let estimates = v.get("estimates").map(Value::arr).unwrap_or_default();
    let estimate = |x: &Value| {
        Some((
            x.at(0)?.str()?.to_string(),
            x.at(1)?.clone(),
            x.at(2)?.clone(),
        ))
    };
    let lineage = v
        .get("lineage")
        .filter(|l| !l.is_null())
        .map(|l| read_lineage(work, l));
    let e = Entry {
        port: text("port")?,
        file: text("file")?,
        sha256: text("sha256")?,
        ty: DataType::from_name(&text("type")?)?,
        estimates: estimates.iter().filter_map(estimate).collect(),
        warnings: v
            .get("warnings")
            .map(Value::arr)
            .unwrap_or_default()
            .iter()
            .filter_map(|w| w.str().map(str::to_string))
            .collect(),
        said: v.get("file_info").filter(|f| !f.is_null()).cloned(),
        lineage,
        extra: v.get("extra").filter(|x| !x.is_null()).and_then(|x| {
            let t = |k: &str| x.get(k).and_then(Value::str).map(str::to_string);
            Some((
                t("port")?,
                t("file")?,
                t("sha256")?,
                DataType::from_name(&t("type")?)?,
            ))
        }),
    };
    Some((text("key")?, e))
}
