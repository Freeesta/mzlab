//! `mzlab-cli flow record RUNDIR --roles ROLES.json [--out FILE]` and `flow verify FILE [--json]`: the `mzrun/2` record of a finished run, and
//! the check of its fingerprints (`mzlab_core::mzrun`, `docs/agenti/mzrun.md`). Exit 0 ok, 3 refused (no commit, a role that cannot be matched, a record
//! that does not match itself), 2 usage. Roles without a SHA-256 take it from the run's own inputs, named on stderr.

use std::path::Path;

use mzlab_core::json::{parse, Out, Value};
use mzlab_core::mzrun::{build, verify, Parts};
use mzlab_core::samples;

use crate::convert::Notice;
use crate::Outcome;

/// The full commit this binary was built from (`build.rs`: `MZLAB_COMMIT`, else git); None when the build knew neither.
pub(crate) fn build_commit() -> Option<&'static str> {
    Some(env!("MZLAB_CLI_COMMIT")).filter(|c| !c.is_empty())
}

/// A failure of a command: a usage mistake (exit 2) or a refusal the data caused (exit 3).
enum Fail {
    Usage(String),
    Refused(String),
}

impl From<String> for Fail {
    fn from(m: String) -> Self {
        Fail::Usage(m)
    }
}

impl From<&str> for Fail {
    fn from(m: &str) -> Self {
        Fail::Usage(m.into())
    }
}

fn refused(e: mzlab_core::error::Error) -> Fail {
    Fail::Refused(e.to_string())
}

fn outcome(r: Result<Vec<u8>, Fail>, log: String) -> Outcome {
    let mut o = match r {
        Ok(out) => Outcome::ok(out),
        Err(Fail::Usage(m)) => Outcome::fail(&m),
        Err(Fail::Refused(m)) => Outcome {
            code: 3,
            ..Outcome::fail(&m)
        },
    };
    o.stderr = log + &o.stderr;
    o
}

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

/// The `(name, sha256)` of the files the plan runs on.
fn plan_inputs(plan: &Value) -> Vec<(String, String)> {
    plan.get("inputs")
        .map_or(&[][..], Value::arr)
        .iter()
        .filter_map(|f| {
            Some((
                f.get("name")?.str()?.to_string(),
                f.get("sha256")?.str()?.to_string(),
            ))
        })
        .collect()
}

fn base(name: &str) -> &str {
    name.rsplit(['/', '\\']).next().unwrap_or(name)
}

/// Fills each role without a SHA-256 from the input of the run whose name is the file name of the role (the full name first, then its last path
/// part); each choice goes to `log`. No match or several different ones: the names the run has, in the refusal.
fn fill_roles(roles: &mut [samples::Sample], plan: &Value, log: &mut String) -> Result<(), Fail> {
    let inputs = plan_inputs(plan);
    for r in roles.iter_mut().filter(|r| r.sha256.is_empty()) {
        let by = |same: &dyn Fn(&str) -> bool| -> Vec<&str> {
            let mut v: Vec<&str> = inputs
                .iter()
                .filter(|(n, _)| same(n))
                .map(|(_, h)| h.as_str())
                .collect();
            v.sort_unstable();
            v.dedup();
            v
        };
        let mut hit = by(&|n| n == r.file);
        if hit.is_empty() {
            hit = by(&|n| base(n) == base(&r.file));
        }
        match hit[..] {
            [sha] => {
                let note = Notice {
                    key: "warn.mzrun.roleSha",
                    params: vec![("file", r.file.clone()), ("sha256", sha.to_string())],
                };
                log.push_str(&crate::error_text::warning(&note));
                log.push('\n');
                r.sha256 = sha.to_string();
            }
            _ => {
                let names: Vec<&str> = inputs.iter().map(|(n, _)| n.as_str()).collect();
                let key = if hit.is_empty() {
                    "err.mzrun.roleNone"
                } else {
                    "err.mzrun.roleSeveral"
                };
                let e = mzlab_core::error::Error::new(key)
                    .with("file", r.file.as_str())
                    .with("inputs", names.join(", "));
                return Err(refused(e));
            }
        }
    }
    Ok(())
}

/// `flow record`: reads `RUNDIR/plan.json` and `prov.json` and the roles; the text of the record to stdout or `--out`.
pub fn record(args: Vec<String>) -> Outcome {
    let mut log = String::new();
    let go = |log: &mut String| -> Result<Vec<u8>, Fail> {
        let (pos, got) = opts(args, &["--roles", "--out"])?;
        let [dir] = &pos[..] else {
            return Err("one run folder only".into());
        };
        let roles = got[0].as_deref().ok_or("--roles is needed")?;
        let e = |e: mzlab_core::error::Error| Fail::Usage(e.to_string());
        let prov = parse(&read(&format!("{dir}/prov.json"))?).map_err(e)?;
        let plan = parse(&read(&format!("{dir}/plan.json"))?).map_err(e)?;
        let mut roles = samples::from_json(&read(roles)?).map_err(e)?;
        fill_roles(&mut roles, &plan, log)?;
        let text = build(&Parts {
            plan: &plan,
            prov: &prov,
            roles: &roles,
            resources: Vec::new(),
            decisions: Vec::new(),
            limits: Vec::new(),
        })
        .map_err(refused)?;
        match &got[1] {
            None => Ok(text.into_bytes()),
            Some(p) => std::fs::write(p, text)
                .map(|_| Vec::new())
                .map_err(|x| Fail::Usage(format!("{p}: {x}"))),
        }
    };
    let r = go(&mut log);
    outcome(r, log)
}

/// `flow verify FILE [--json]`: nothing when all matches; else one line per mismatch on stderr, exit 3. `--json` writes the `verify/1` document on
/// stdout instead (`{"schema":"verify/1","ok":..,"self_sha256":..,"problems":[..]}`), same exit codes.
pub fn check(args: Vec<String>) -> Outcome {
    let json = args.iter().any(|a| a == "--json");
    let args: Vec<String> = args.into_iter().filter(|a| a != "--json").collect();
    let go = || -> Result<_, String> {
        let (pos, _) = opts(args, &[])?;
        let [file] = &pos[..] else {
            return Err("one record only".into());
        };
        verify(&read(file)?).map_err(|e| e.to_string())
    };
    let v = match go() {
        Err(m) => return Outcome::fail(&m),
        Ok(v) => v,
    };
    let code = if v.problems.is_empty() { 0 } else { 3 };
    if json {
        let mut o = Out::default();
        o.raw("{\"schema\":\"verify/1\",\"ok\":")
            .raw(if code == 0 { "true" } else { "false" });
        o.raw(",\"self_sha256\":")
            .str(&v.self_sha256)
            .raw(",\"problems\":[");
        for (k, p) in v.problems.iter().enumerate() {
            if k > 0 {
                o.raw(",");
            }
            o.str(p);
        }
        o.raw("]}\n");
        return Outcome {
            code,
            stdout: o.0.into_bytes(),
            stderr: v.problems.iter().map(|p| format!("{p}\n")).collect(),
            error: None,
        };
    }
    Outcome {
        code,
        stdout: Vec::new(),
        stderr: v.problems.iter().map(|p| format!("{p}\n")).collect(),
        error: None,
    }
}
