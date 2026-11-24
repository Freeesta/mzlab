//! `mzlab-cli file convert --dry-run`: what would be converted (input, output, reader, bytes) without writing anything, and the refusal of an mzML named
//! as an input. Split from `convert.rs` for the size limit.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::convert_entries::Entry;
use crate::convert_report::Planned;
use crate::Outcome;

/// `err.file.alreadyMzml` (exit 4) for the first input that is an mzML file.
pub(crate) fn refuse_mzml(inputs: &[PathBuf]) -> Option<Outcome> {
    let e = inputs
        .iter()
        .find_map(|p| crate::sample_select::refuse_mzml(p).err())?;
    Some(
        Outcome::fail_error(&e)
            .with_hint("an mzML is read as it is; `mzlab-cli file validate` checks it"),
    )
}

/// The dry run as an outcome: the `convert/1` document or one line per mzML on stdout, the count on stderr, exit 0.
pub(crate) fn outcome(list: &[Entry], out: &Path, json: bool) -> Outcome {
    let plan = planned(list, out);
    let stdout = if json {
        crate::convert_report::dry_run(&plan, out)
    } else {
        dry_text(&plan)
    };
    Outcome {
        code: 0,
        stdout: stdout.into_bytes(),
        stderr: format!(
            "dry run: {} mzML would be written to {}\n",
            plan.len(),
            out.display()
        ),
        error: None,
    }
}

/// What `--dry-run` reports: per mzML to write, the reader that takes the dataset and the bytes it holds (all its members).
fn planned(list: &[Entry], out: &Path) -> Vec<Planned> {
    let mut seen: HashMap<PathBuf, (Option<&'static str>, u64)> = HashMap::new();
    list.iter()
        .map(|e| {
            let (reader, input_bytes) = *seen.entry(e.input.clone()).or_insert_with(|| {
                let Ok(fs) = crate::flow_set::file_set(&e.input) else {
                    return (None, 0);
                };
                let bytes = fs.members.iter().map(|m| m.dim).sum();
                let reader =
                    mzlab_lettori::open_listing(fs, &mzlab_core::readers::Limits::desktop())
                        .ok()
                        .map(|l| l.format().id());
                (reader, bytes)
            });
            let sample_id = e.label.rsplit_once('@').and_then(|(_, id)| id.parse().ok());
            Planned {
                input: e.input.display().to_string(),
                output: out.join(&e.out).display().to_string(),
                sample_id,
                reader,
                input_bytes,
                empty: e.empty,
                rename: e.note.clone(),
            }
        })
        .collect()
}

/// The text of `--dry-run` without `--json`: one line per mzML, `INPUT -> OUTPUT (reader, N bytes)`.
fn dry_text(plan: &[Planned]) -> String {
    plan.iter()
        .map(|p| {
            format!(
                "{} -> {} ({}, {} bytes)\n",
                p.input,
                p.output,
                p.reader.unwrap_or("no reader"),
                p.input_bytes
            )
        })
        .collect()
}
