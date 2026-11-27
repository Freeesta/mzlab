//! `mzlab-cli table slots`: the fixed slots of the feature view of one row (`feature-slots/1`): per slot filled, empty or na, the source of the evidence or the step and
//! command that fill it. The bytes are those of the browser (`table_slots`) and of the app (`feat/table` `slots`): one engine, `mzlab_ops::run::table::core`.

use mzlab_core::error::Result;
use mzlab_ops::run::table::core;

use super::table_row_cmd::{read, samples_of};
use crate::extract;
use crate::Outcome;

pub const HELP: &str = "mzlab-cli table slots: what fills each slot of the feature view of one row (feature-slots/1).

usage: mzlab-cli table slots TABLE.mzfeat --id N [--samples FILE.json] [--json] [--out PATH]

  TABLE.mzfeat  a feature table (`features`, `flow run`, `quant`): the `id` of the first column of its CSV is --id
  --id N        the row; an id the table does not have is exit 5
  --samples F   a samples/1 document (`sample table`): types and times of the files; without it they are proposed from the file names (noted on stderr)
  --json        accepted: the answer is always JSON
  --out PATH    write there instead of the standard output

Slots in a fixed order: structure, xic, kinetics, ms2, isotopes, uv, sim. Each has `state` (filled: the table holds evidence; empty: nothing yet; na: this table cannot hold it, `reason` says why),
`evidence` (the numbers the table has, else null), `source` (output, step that writes it, input files with SHA-256) when filled, and `fill` (the Flow step and the command) when not.
Exit codes: 0 ok; 2 usage (no --id); 4 table or samples missing or unreadable; 5 the id is not in the table; 6 --out cannot be written.";

struct Job {
    table: std::path::PathBuf,
    id: u32,
    samples: Option<std::path::PathBuf>,
    out: Option<std::path::PathBuf>,
}

fn job(args: Vec<String>) -> std::result::Result<std::result::Result<Job, &'static str>, String> {
    let Some(p) = extract::parse(args, &["--id", "--samples", "--out"], &[])? else {
        return Ok(Err(HELP));
    };
    let table = match p.pos.as_slice() {
        [t] => std::path::PathBuf::from(t),
        [] => return Err("table slots needs a table".into()),
        _ => return Err("table slots takes one table".into()),
    };
    let id = p
        .get("--id")
        .ok_or("table slots needs --id")?
        .parse::<u32>()
        .map_err(|_| "--id needs a row id (a whole number)")?;
    Ok(Ok(Job {
        table,
        id,
        samples: p.get("--samples").map(Into::into),
        out: p.out(),
    }))
}

fn answer(j: &Job) -> Result<(String, bool)> {
    let table = read(&j.table)?;
    let (samples, proposed) = samples_of(j.samples.as_deref(), &table)?;
    let name = j
        .table
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let text = core::slots_json(
        &mut table.clone(),
        table.len() as u64,
        &samples,
        j.id,
        &name,
    )?;
    Ok((text + "\n", proposed))
}

pub fn run(args: Vec<String>) -> Outcome {
    let j = match job(args) {
        Ok(Ok(j)) => j,
        Ok(Err(h)) => return Outcome::ok(format!("{h}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    match answer(&j) {
        Err(e) => Outcome::fail_error(&e),
        Ok((t, proposed)) => {
            let mut o = crate::emit(t, &j.out);
            if proposed && o.code == 0 {
                o.stderr += "note.table.slots.samples: no --samples, the types and times come from the file names\n";
            }
            o
        }
    }
}

#[cfg(test)]
#[path = "table_slots_cmd_tests.rs"]
mod tests;
