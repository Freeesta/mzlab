//! What `flow check` accepts, `flow run` runs: for every wire a producer of the registry can make into a consumer of the same type, a plan built from the
//! descriptors (the base chain over the synthetic identity series, every other port from a file or a step of its type) is checked, then run. A pair is a
//! mismatch when the check passes and the consumer runs on the canonical input of the port but not on this one; the ones of today are `KNOWN`. Sources
//! at the edge: a `table.filter` view, a table with no row, a table of one file. Skipped without python3 and numpy (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use mzlab_core::samples::{proponi, to_json};
use mzlab_core::step::{DataType, Step};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_DETECTED};

use crate::flow_link_tests::{hypotheses, series, FORMULA};

const NAMES: [&str; 6] = [
    "SIN_blank",
    "SIN_dark",
    "SIN_t0",
    "SIN_t05min",
    "SIN_t10min",
    "SIN_t20min",
];

/// The steps of the base chain, in every plan: the canonical index, peaks and table.
const BASE: &[(&str, &str)] = &[
    (
        "idx",
        r#"{"id":"idx","step":"index.mzidx@1","in":{"source":[SRC]}}"#,
    ),
    (
        "det",
        r#"{"id":"det","step":"features.detect@1","in":{"index":["idx.index"]}}"#,
    ),
    (
        "ali",
        r#"{"id":"ali","step":"features.align@1","in":{"peaks":["det.peaks"]}}"#,
    ),
    (
        "gap",
        r#"{"id":"gap","step":"features.gapfill@1","in":{"table":"ali.table","index":["idx.index"]}}"#,
    ),
];

/// Where a value of a type comes from: a file of the plan or an output of a step (with the steps it needs).
#[derive(Clone, Debug)]
struct Source {
    ty: DataType,
    /// What a port writes to read it (`file:x` or `inst.port`).
    wire: String,
    /// The instances it needs, in order (id, JSON).
    needs: Vec<(String, String)>,
    /// `producer.port`, or the file and what is special about it.
    label: String,
}

/// A port of a step that reads `ty` (its own type or one it accepts).
fn reads(p: &mzlab_core::step::Port, ty: DataType) -> bool {
    p.ty == ty || p.accepts.contains(&ty)
}

struct Fixture {
    dir: PathBuf,
    /// The plan inputs, as JSON objects.
    inputs: Vec<String>,
    /// The same inputs with their hashes and facets, read once: every plan of the harness lists them all.
    filled: Vec<mzlab_core::flow::FileInput>,
    reg: Vec<Step>,
}

impl Fixture {
    fn plan(&self, steps: &[(String, String)]) -> String {
        let mut seen = BTreeSet::new();
        let list: Vec<&str> = steps
            .iter()
            .filter(|(id, _)| seen.insert(id.clone()))
            .map(|(_, j)| j.as_str())
            .collect();
        format!(
            r#"{{"format":"mzflow/2","inputs":[{}],"steps":[{}]}}"#,
            self.inputs.join(","),
            list.join(",")
        )
    }

    /// The problems of the check, empty when the plan passes.
    fn check(&self, text: &str) -> Vec<String> {
        let mut plan = match mzlab_core::flow::parse(text.as_bytes()) {
            Ok(p) => p,
            Err(e) => return vec![e.key.to_string()],
        };
        plan.inputs = self.filled.clone();
        match crate::flow_run::prepare(plan, &self.reg) {
            Ok(Ok(_)) => Vec::new(),
            Ok(Err(p)) => p.iter().map(crate::flow_run::problem_text).collect(),
            Err(e) => vec![e.key.to_string()],
        }
    }

    /// Runs the plan with the cache of the working folder; per instance, Ok or the key of its error (`skipped` when it did not run).
    fn run(&self, text: &str) -> Result<BTreeMap<String, Result<(), String>>, String> {
        let work = self.dir.join("work");
        let (run, prov) = crate::flow_out::run_plan_cached(
            text.as_bytes(),
            &self.dir,
            &work,
            2,
            None,
            Some(false),
        )
        .map_err(|e| e.key.to_string())?;
        let prov = mzlab_core::json::parse(prov.as_bytes()).unwrap();
        let mut out = BTreeMap::new();
        for (id, act) in prov.get("activity").map(|a| a.entries()).unwrap_or(&[]) {
            let got = match act.get("error") {
                Some(e) => Err(e
                    .get("error_key")
                    .and_then(|k| k.str())
                    .unwrap_or("?")
                    .to_string()),
                None => Ok(()),
            };
            out.insert(id.clone(), got);
        }
        for s in &run.skipped {
            out.entry(s.clone())
                .or_insert_with(|| Err("skipped".into()));
        }
        Ok(out)
    }
}

fn names_origin() -> Origin {
    Origin {
        inputs: NAMES
            .iter()
            .map(|n| (format!("{n}.mzML"), String::new()))
            .collect(),
        ..Origin::default()
    }
}

/// A table of `files` files and `rows` rows written by the store.
fn small_table(files: usize, rows: usize, origin: &Origin) -> Vec<u8> {
    let mut t = Aligned::with_capacity(files, rows);
    for r in 0..rows {
        t.mz.push(250.0 + 17.0 * r as f64);
        t.rt.push(2.0 + r as f64);
        for k in 0..files {
            let a = 1e5 * (1.0 + k as f64);
            t.area.push(a);
            t.height.push((a / 10.0) as f32);
            t.apex_rt.push(2.0 + r as f32);
            t.width.push(0.1);
            t.state.push(CELL_DETECTED);
        }
    }
    write_table(&t, &Recipe::default(), origin)
}

const MSP: &str = "NAME: invented one\nPRECURSORMZ: 450.2155\nPRECURSORTYPE: [M+H]+\nFORMULA: C22H31N3O4S\nIONMODE: Positive\nNum Peaks: 3\n120.0808 100\n235.1441 40\n332.1675 25\n\nNAME: invented two\nPRECURSORMZ: 466.2104\nPRECURSORTYPE: [M+H]+\nFORMULA: C22H31N3O5S\nIONMODE: Positive\nNum Peaks: 2\n136.0757 100\n348.1624 30\n\n";

fn fixture(out: &Path) -> Fixture {
    let dir = out.join("pairs");
    std::fs::create_dir_all(dir.join("work")).unwrap();
    let mut inputs = Vec::new();
    let mut add = |name: &str, ty: &str, bytes: &[u8]| {
        std::fs::write(dir.join(name), bytes).unwrap();
        inputs.push(format!(r#"{{"name":"{name}","type":"{ty}"}}"#));
    };
    for n in NAMES {
        let b = std::fs::read(out.join("identita").join(format!("{n}.mzML"))).unwrap();
        add(&format!("{n}.mzML"), "raw", &b);
        // other bytes for the mzML copy: the run names an artifact after the first file with its SHA-256, and the samples name the raw files
        add(&format!("m_{n}.mzML"), "mzml", &[&b[..], b"\n"].concat());
    }
    let files: Vec<_> = NAMES
        .iter()
        .map(|n| (format!("{n}.mzML"), String::new(), String::new()))
        .collect();
    let camp = proponi(&files);
    add("samples.json", "samples", to_json(&camp).as_bytes());
    add(
        "design.json",
        "design",
        mzlab_core::drawing::to_json(&mzlab_core::drawing::propose(&camp)).as_bytes(),
    );
    add(
        "parent.json",
        "structure",
        format!(r#"{{"formula":"{FORMULA}","rt":1.0}}"#).as_bytes(),
    );
    add("hyp.json", "transform", hypotheses().as_bytes());
    add("library.msp", "library", MSP.as_bytes());
    add(
        "empty.mzfeat",
        "table",
        &small_table(NAMES.len(), 0, &names_origin()),
    );
    let one = Origin {
        inputs: vec![("SIN_t20min.mzML".into(), String::new())],
        ..Origin::default()
    };
    add("single.mzfeat", "table", &small_table(1, 3, &one));
    let text = format!(
        r#"{{"format":"mzflow/2","inputs":[{}],"steps":[]}}"#,
        inputs.join(",")
    );
    let mut plan = mzlab_core::flow::parse(text.as_bytes()).unwrap();
    crate::flow_run::fill_inputs(&mut plan, |n| dir.join(crate::flow_set::dataset_name(n)))
        .unwrap();
    Fixture {
        dir,
        inputs,
        filled: plan.inputs,
        reg: crate::flow::steps(),
    }
}

fn base() -> Vec<(String, String)> {
    let src: Vec<String> = NAMES
        .iter()
        .map(|n| format!(r#""file:{n}.mzML""#))
        .collect();
    BASE.iter()
        .map(|(id, j)| (id.to_string(), j.replace("SRC", &src.join(","))))
        .collect()
}

/// The sources the plan has before any step runs: the files and the base chain.
fn seeds() -> Vec<Source> {
    let file = |ty: DataType, name: &str, label: &str| Source {
        ty,
        wire: format!("file:{name}"),
        needs: Vec::new(),
        label: label.into(),
    };
    let step = |ty: DataType, wire: &str| Source {
        ty,
        wire: wire.into(),
        needs: base(),
        label: wire.into(),
    };
    vec![
        step(DataType::Mzidx, "idx.index"),
        step(DataType::Peaks, "det.peaks"),
        step(DataType::Table, "gap.table"),
        step(DataType::Table, "ali.table"),
        file(DataType::Raw, "SIN_t20min.mzML", "raw file"),
        file(DataType::Mzml, "m_SIN_t20min.mzML", "mzML file"),
        file(DataType::Samples, "samples.json", "samples"),
        file(DataType::Design, "design.json", "design"),
        file(DataType::Structure, "parent.json", "parent"),
        file(DataType::Transform, "hyp.json", "hypotheses file"),
        file(DataType::Library, "library.msp", "MSP library"),
        file(DataType::Table, "empty.mzfeat", "table with no row"),
        file(DataType::Table, "single.mzfeat", "table of one sample"),
    ]
}

/// The wire of a port reading `s`: a list for a `many` port (all the raw files for a raw or mzML series).
fn wire(p: &mzlab_core::step::Port, s: &Source) -> String {
    if !p.many {
        return format!(r#""{}""#, s.wire);
    }
    if s.wire == "file:SIN_t20min.mzML" || s.wire == "file:m_SIN_t20min.mzML" {
        let pre = if s.ty == DataType::Mzml { "m_" } else { "" };
        let all: Vec<String> = NAMES
            .iter()
            .map(|n| format!(r#""file:{pre}{n}.mzML""#))
            .collect();
        return format!("[{}]", all.join(","));
    }
    format!(r#"["{}"]"#, s.wire)
}

/// The instance `id` of `step` reading `chosen` on `port` and the canonical source of its type on every other port; None when a required port has none.
fn instance(
    step: &Step,
    id: &str,
    pick: &dyn Fn(DataType) -> Option<Source>,
    chosen: Option<(&str, &Source)>,
) -> Option<(Vec<(String, String)>, String)> {
    let mut needs = Vec::new();
    let mut ins = Vec::new();
    for p in &step.ins {
        let s = match chosen {
            Some((port, s)) if port == p.name => Some(s.clone()),
            _ => pick(p.ty),
        };
        match s {
            Some(s) => {
                ins.push(format!(r#""{}":{}"#, p.name, wire(p, &s)));
                needs.extend(s.needs.clone());
            }
            None if p.optional => {}
            None => return None,
        }
    }
    let json = format!(
        r#"{{"id":"{id}","step":"{}@{}","in":{{{}}}}}"#,
        step.id,
        step.version,
        ins.join(",")
    );
    Some((needs, json))
}

fn safe(id: &str) -> String {
    id.replace('.', "_")
}

/// Steps the harness leaves out: `meta.compare` wraps another step and has no ports of its own.
const LEFT_OUT: &[&str] = &["meta.compare"];

#[path = "flow_pairs_known.rs"]
mod known;
use known::{report_and_assert, Verdict};

/// Round by round, every step that can be built from the pool and runs adds its outputs to `pool` (a step of a later round reads the outputs of the earlier ones).
fn producers(fx: &Fixture, pool: &mut Vec<Source>) -> Vec<(String, Verdict)> {
    let mut tried: BTreeSet<String> = BTreeSet::new();
    let mut notes: Vec<(String, Verdict)> = Vec::new();
    for _round in 0..3 {
        let snapshot = pool.clone();
        let pick = |ty: DataType| snapshot.iter().find(|s| s.ty == ty).cloned();
        let mut batch = Vec::new();
        for step in fx
            .reg
            .iter()
            .filter(|s| !LEFT_OUT.contains(&s.id) && !tried.contains(s.id))
        {
            let id = format!("p_{}", safe(step.id));
            let Some((mut needs, json)) = instance(step, &id, &pick, None) else {
                continue;
            };
            needs.push((id.clone(), json));
            if fx.check(&fx.plan(&needs)).is_empty() {
                batch.push((step, id, needs));
            }
        }
        if batch.is_empty() {
            break;
        }
        let all: Vec<(String, String)> = batch.iter().flat_map(|(_, _, n)| n.clone()).collect();
        let res = fx.run(&fx.plan(&all)).unwrap();
        for (step, id, needs) in batch {
            tried.insert(step.id.to_string());
            if res.get(&id).is_some_and(|r| r.is_ok()) {
                for o in &step.outs {
                    pool.push(Source {
                        ty: o.ty,
                        wire: format!("{id}.{}", o.name),
                        needs: needs.clone(),
                        label: format!("{}.{}", step.id, o.name),
                    });
                }
            } else {
                // on the canonical inputs: what the synthetic files do not hold (no QC, no PDA, ...), its outputs are not sources
                let why = res
                    .get(&id)
                    .cloned()
                    .unwrap_or(Err("missing".into()))
                    .err()
                    .unwrap_or_default();
                notes.push((
                    format!("{} (as a producer)", step.id),
                    Verdict::Uncovered(why),
                ));
            }
        }
    }
    notes
}

/// Every consumer with a canonical instance that passes the check: its instance and one per (port, other source) that passes, all run in one plan.
fn consumers(
    fx: &Fixture,
    pool: &[Source],
    mut verdicts: Vec<(String, Verdict)>,
) -> Vec<(String, Verdict)> {
    // the canonical source of a type is the first one of the pool
    let canonical = pool.to_vec();
    let pick = |ty: DataType| canonical.iter().find(|s| s.ty == ty).cloned();
    // every consumer with a canonical instance that passes the check: its instance and one per (port, other source) that passes, all run in one plan
    let mut all: Vec<(String, String)> = Vec::new();
    let mut cases: Vec<(String, String, String)> = Vec::new();
    for step in fx.reg.iter().filter(|s| !LEFT_OUT.contains(&s.id)) {
        let base_id = format!("c_{}", safe(step.id));
        let Some((mut needs, json)) = instance(step, &base_id, &pick, None) else {
            verdicts.push((
                format!("{} *", step.id),
                Verdict::Uncovered("no input".into()),
            ));
            continue;
        };
        needs.push((base_id.clone(), json));
        let problems = fx.check(&fx.plan(&needs));
        if !problems.is_empty() {
            verdicts.push((
                format!("{} *", step.id),
                Verdict::Uncovered(problems.join(" ")),
            ));
            continue;
        }
        all.extend(needs);
        for (pi, p) in step.ins.iter().enumerate() {
            for (si, s) in pool.iter().enumerate().filter(|(_, s)| reads(p, s.ty)) {
                if pick(p.ty).is_some_and(|c| c.wire == s.wire) {
                    continue;
                }
                let id = format!("{base_id}_{pi}_{si}");
                let Some((mut n, j)) = instance(step, &id, &pick, Some((p.name, s))) else {
                    continue;
                };
                n.push((id.clone(), j));
                let problems = fx.check(&fx.plan(&n));
                let what = format!("{}.{} <- {}", step.id, p.name, s.label);
                if problems.is_empty() {
                    all.extend(n);
                    cases.push((base_id.clone(), id, what));
                } else {
                    verdicts.push((what, Verdict::Refused(problems.join(" "))));
                }
            }
        }
    }
    let res = fx.run(&fx.plan(&all)).unwrap();
    let got = |id: &str| res.get(id).cloned().unwrap_or(Err("missing".into()));
    for (base_id, id, what) in cases {
        let v = match (got(&base_id), got(&id)) {
            (_, Ok(())) => Verdict::Ran,
            (Err(b), Err(_)) => Verdict::Uncovered(b),
            (Ok(()), Err(e)) => Verdict::Mismatch(e),
        };
        verdicts.push((what, v));
    }
    verdicts
}

#[test]
fn what_the_check_accepts_runs() {
    let Some(out) = series("pairs") else { return };
    let fx = fixture(&out);
    let mut pool = seeds();
    let notes = producers(&fx, &mut pool);
    let verdicts = consumers(&fx, &pool, notes);
    report_and_assert(&verdicts);
    let _ = std::fs::remove_dir_all(&out);
}
