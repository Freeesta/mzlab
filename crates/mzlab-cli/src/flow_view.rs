//! A `table.filter` view read by a step that wants a `.mzfeat`: before the step runs, every view on one of its `table` ports becomes the table it stands
//! for (`mzlab_ops::run::table::core::table_of_view`, the function the `table` `.wasm` exports), written once per view in the working folder with the lineage of
//! the table it was made from. The exports read the view itself (they write the rows it keeps from the whole table). Called by `flow::dispatch`; declared in `flow_export`.

use std::io::Read;
use std::path::Path;

use crate::flow::{io, FileRunner};
use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;
use mzlab_ops::run::table::core::{is_view, table_of_view, view_origin};

/// Number of the next temporary file, so that two threads never write the same one.
static PART: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The steps that read a view as a view.
const READ_VIEWS: &[&str] = &["export.table_csv", "export.mztab"];

/// `job` with its views replaced by their tables; `None` when it reads no view.
pub(crate) fn resolve<'a>(r: &FileRunner, job: &Job<'a>) -> Result<Option<Job<'a>>> {
    if READ_VIEWS.contains(&job.step.id) {
        return Ok(None);
    }
    let table_port = |name: &str| {
        job.step.ins.iter().any(|p| {
            p.name == name && (p.ty == DataType::Table || p.accepts.contains(&DataType::Table))
        })
    };
    let mut inputs = job.inputs.clone();
    let mut changed = false;
    for (port, list) in inputs.iter_mut().filter(|(p, _)| table_port(p)) {
        for a in list.iter_mut().filter(|a| a.ty == DataType::Table) {
            let path = r.path(a);
            // other texts of type `table` (a `deriva/1`, a `filtri/1`) stay as they are: the step refuses them with its own key
            let Some(view) = view_text(&path) else {
                continue;
            };
            *a = table_artifact(r, &view).map_err(|e| e.with("port", port.as_str()))?;
            changed = true;
        }
    }
    Ok(changed.then(|| Job {
        inputs,
        item_name: job.item_name.clone(),
        item_root: job.item_root.clone(),
        ..*job
    }))
}

/// The text of the file when it is a view (it begins with `{` and names the table and the rules it was made from); one byte is read from a `.mzfeat`.
fn view_text(path: &Path) -> Option<String> {
    let mut b = [0u8; 1];
    std::fs::File::open(path)
        .and_then(|mut f| f.read_exact(&mut b))
        .ok()?;
    if !is_view(&b) {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    view_origin(&text).is_ok().then_some(text)
}

/// The table of `view`, made once per view and kept as a file of the run.
fn table_artifact(r: &FileRunner, view: &str) -> Result<Artifact> {
    let bad = || Error::new("err.flow.export.view");
    let (sha, _) = view_origin(view)?;
    let at = crate::flow_export::table_by_sha(r, &sha).ok_or_else(bad)?;
    let out = r.work.join(format!(
        "view-{}.mzfeat",
        &mzlab_core::budget::sha256_hex(view.as_bytes())[..16]
    ));
    if !out.exists() {
        // two steps may read the same view at once: each writes its own file and the rename is atomic
        let table = std::fs::read(&at).map_err(io)?;
        let n = PART.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let part = out.with_extension(format!("{}-{n}.part", std::process::id()));
        std::fs::write(&part, table_of_view(&table, view)?).map_err(io)?;
        std::fs::rename(&part, &out).map_err(io)?;
    }
    let a = r.keep(out, DataType::Table)?;
    let origin = Artifact {
        sha256: sha,
        bytes: 0,
        ty: DataType::Table,
        label: None,
    };
    if let Some(l) = r.lineage(&origin) {
        r.trace(&a, l);
    }
    Ok(a)
}

#[cfg(test)]
#[path = "flow_pairs_tests.rs"]
mod pairs_tests;
#[cfg(test)]
#[path = "flow_view_tests.rs"]
mod tests;
