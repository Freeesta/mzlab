//! `mzlab-cli flow record RUNDIR --roles CAMPIONI.json [--out FILE]` and `flow verify FILE`: the `mzrun/1` record of a finished run, and
//! the check of its fingerprints (`mzlab_core::mzrun`, `docs/agenti/mzrun.md`). Exit 0 ok, 3 the record does not match itself, 2 usage.

use std::path::Path;

use mzlab_core::campioni;
use mzlab_core::json::parse;
use mzlab_core::mzrun::{build, verify, Parts};

use crate::Outcome;

fn read(p: &str) -> Result<Vec<u8>, String> {
    std::fs::read(Path::new(p)).map_err(|e| format!("{p}: {e}"))
}

/// The value of the option after `--name`, once.
fn opts(args: Vec<String>, names: &[&str]) -> Result<(Vec<String>, Vec<Option<String>>), String> {
    let (mut pos, mut got) = (Vec::new(), vec![None; names.len()]);
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match names.iter().position(|n| *n == a) {
            Some(i) => got[i] = Some(it.next().ok_or(format!("{a} needs a value"))?),
            None if a.starts_with("--") => return Err(format!("unknown option {a}")),
            None => pos.push(a),
        }
    }
    Ok((pos, got))
}

/// `flow record`: reads `RUNDIR/plan.json` and `prov.json` and the roles; the text of the record to stdout or `--out`.
pub fn record(args: Vec<String>) -> Outcome {
    let go = || -> Result<Vec<u8>, String> {
        let (pos, got) = opts(args, &["--roles", "--out"])?;
        let [dir] = &pos[..] else {
            return Err("one run folder only".into());
        };
        let roles = got[0].as_deref().ok_or("--roles is needed")?;
        let e = |e: mzlab_core::error::Error| e.to_string();
        let prov = parse(&read(&format!("{dir}/prov.json"))?).map_err(e)?;
        let plan = parse(&read(&format!("{dir}/plan.json"))?).map_err(e)?;
        let roles = campioni::from_json(&read(roles)?).map_err(e)?;
        let text = build(&Parts {
            plan: &plan,
            prov: &prov,
            roles: &roles,
            resources: Vec::new(),
            decisions: Vec::new(),
            limits: Vec::new(),
        })
        .map_err(e)?;
        match &got[1] {
            None => Ok(text.into_bytes()),
            Some(p) => std::fs::write(p, text)
                .map(|_| Vec::new())
                .map_err(|x| format!("{p}: {x}")),
        }
    };
    go().map_or_else(|m| Outcome::fail(&m), Outcome::ok)
}

/// `flow verify FILE`: one line per thing that does not match, nothing when all does.
pub fn check(args: Vec<String>) -> Outcome {
    let go = || -> Result<_, String> {
        let (pos, _) = opts(args, &[])?;
        let [file] = &pos[..] else {
            return Err("one record only".into());
        };
        verify(&read(file)?).map_err(|e| e.to_string())
    };
    match go() {
        Err(m) => Outcome::fail(&m),
        Ok(v) if v.problems.is_empty() => Outcome::ok(Vec::new()),
        Ok(v) => Outcome {
            code: 3,
            stdout: (v.problems.join("\n") + "\n").into_bytes(),
            stderr: String::new(),
        },
    }
}
