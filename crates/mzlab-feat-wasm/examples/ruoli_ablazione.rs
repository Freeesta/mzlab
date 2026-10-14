//! Ablation of the roles of the files in the view of the products (`tp/1`): how many rows the view keeps, and how many of them are not in the truth, when a
//! kind of file (t0, blank, dark, infusion) is taken out of the run. The recipe, the rules and every threshold are the defaults, the same in every configuration.
//!
//!     cargo run --release -p mzlab-feat-wasm --example ruoli_ablazione -- SPEC.json [--ruoli]
//!
//! `SPEC.json`: `parent` {`formula` (neutral), `mz`, `rt` (the row of the table nearest to them, within `tol`, is the parent), `smiles`?}, `rules` (path of regole_tp.json), `tol` {`ppm` | `da`, `rt`} (matching with the truth),
//! `files` [{`path` (.mzidx), `role` (treated | t0 | blank | dark | infusion), `minutes` (treated)}], `truth`? (a `verita_tp/1` document or the `verita.json` of
//! the synthetic series: `ioni`), `probes`? [{`mz`, `rt`}] (status of the rows around a point). Prints one JSON document with a line per configuration.
//! The configurations: `completa` (every file as given: the infusion counts as a control, as the Pipeline panel does), `senza_t0`, `senza_bianco`,
//! `senza_buio`, `senza_infusione`, `solo_tempi` (t0 and treated only); one whose role is absent from the files is `assente`.
//! `--ruoli`: the run gets the roles of the files (`campioni/2`, confirmed times) instead of the minutes alone: gate 1 reads each reference with its
//! reason (`tp::build_by_role`) and the infusion is never a control; each line also gives the rows taken out by the blank, the t0 and the dark.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;

use mzlab_core::campioni::Campione;
use mzlab_core::idxread::Index;
use mzlab_core::json::{self, Value};
use mzlab_feat::store::read_table;
use mzlab_feat::tp::{TpRoles, ST_CONTROL, ST_ISF, ST_PARENT, ST_TP, ST_UNEXPLAINED};
use mzlab_feat_wasm::run;

const ROLES: [&str; 5] = ["treated", "t0", "blank", "dark", "infusion"];
const CONFIGS: [(&str, &str); 6] = [
    ("completa", ""),
    ("senza_t0", "t0"),
    ("senza_bianco", "blank"),
    ("senza_buio", "dark"),
    ("senza_infusione", "infusion"),
    ("solo_tempi", "*"),
];
/// Roles of the synthetic truth that are products; the rest of its ions are inherited from the parent, decoys or background.
const TP_ROLES: [&str; 3] = ["tp1", "tp2", "isomero"];
const INHERITED: [&str; 5] = ["isotopo", "addotto", "isf", "chimerica", "ereditato"];

struct FileSpec {
    path: String,
    role: String,
    minutes: f64,
}

/// An ion of the truth.
struct Entry {
    mz: f64,
    rts: Vec<f64>,
    cat: String,
    product: bool,
}

#[derive(Clone)]
struct Tol {
    ppm: f64,
    da: f64,
    rt: f64,
}

fn text(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::str).unwrap_or("").to_string()
}

fn num(v: &Value, k: &str, default: f64) -> f64 {
    v.get(k).and_then(Value::f64).unwrap_or(default)
}

fn load(path: &str) -> Value {
    json::parse(&std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"))).expect("json")
}

fn open(path: &str) -> Index<File> {
    let f = File::open(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let size = f.metadata().expect("size").len();
    Index::open(f, size).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn files_of(spec: &Value) -> Vec<FileSpec> {
    let list = spec.get("files").map(Value::arr).unwrap_or(&[]);
    list.iter()
        .map(|f| {
            let role = text(f, "role");
            assert!(ROLES.contains(&role.as_str()), "role {role}");
            FileSpec {
                path: text(f, "path"),
                minutes: num(f, "minutes", f64::NAN),
                role,
            }
        })
        .collect()
}

/// Treatment time of the run as the percorso TP reads it: the t0 at 0, the treated at their minutes, every other file NaN (a control).
fn minutes_of(f: &FileSpec) -> f64 {
    match f.role.as_str() {
        "t0" => 0.0,
        "treated" => f.minutes,
        _ => f64::NAN,
    }
}

fn numbers(v: Option<&Value>) -> Vec<f64> {
    match v {
        Some(Value::Num(n)) => vec![*n],
        Some(a) => a.arr().iter().filter_map(Value::f64).collect(),
        None => Vec::new(),
    }
}

/// The ions of the truth: `ioni` (synthetic series) or `tps` + `ereditati` + `esche` (the format of the real ones).
fn truth_of(doc: &Value) -> Vec<Entry> {
    let mut out = Vec::new();
    for i in doc.get("ioni").map(Value::arr).unwrap_or(&[]) {
        if num(i, "presente", 1.0) == 0.0 {
            continue;
        }
        let cat = text(i, "ruolo");
        let product = TP_ROLES.contains(&cat.as_str());
        let (mz, rts) = (num(i, "mz", f64::NAN), numbers(i.get("rt_obs")));
        out.push(Entry {
            mz,
            rts,
            cat,
            product,
        });
    }
    for (key, cat, product) in [("tps", "tp", true), ("ereditati", "ereditato", false)] {
        for t in doc.get(key).map(Value::arr).unwrap_or(&[]) {
            let rts = numbers(t.get(if product { "rt_obs" } else { "rt_atteso" }));
            let seen = product && t.get("rilevabile").and_then(Value::bool) == Some(false);
            if !seen {
                out.push(Entry {
                    mz: num(t, "mz", f64::NAN),
                    rts,
                    cat: cat.into(),
                    product,
                });
            }
        }
    }
    out
}

fn tol_of(spec: &Value) -> Tol {
    let t = spec.get("tol").cloned().unwrap_or(Value::Null);
    Tol {
        ppm: num(&t, "ppm", 0.0),
        da: num(&t, "da", 0.0),
        rt: num(&t, "rt", 0.2),
    }
}

fn near(t: &Tol, mz: f64, rt: f64, e_mz: f64, rts: &[f64]) -> bool {
    let window = if t.da > 0.0 {
        t.da
    } else {
        e_mz * t.ppm * 1e-6
    };
    (mz - e_mz).abs() <= window && (rts.is_empty() || rts.iter().any(|r| (rt - r).abs() <= t.rt))
}

fn status_name(s: u8) -> &'static str {
    match s {
        ST_TP => "tp",
        ST_CONTROL => "control",
        ST_ISF => "isf",
        ST_UNEXPLAINED => "unexplained",
        ST_PARENT => "parent",
        _ => "other",
    }
}

fn object(m: &BTreeMap<String, usize>) -> String {
    let items: Vec<String> = m.iter().map(|(k, v)| format!("\"{k}\":{v}")).collect();
    format!("{{{}}}", items.join(","))
}

/// What the view says about one configuration.
#[derive(Default)]
struct Report {
    rows: usize,
    by_status: BTreeMap<String, usize>,
    /// Category of the truth of the rows by status (`none` = not in the truth).
    by_cat: BTreeMap<String, BTreeMap<String, usize>>,
    products_seen: BTreeSet<usize>,
    inherited_in_view: usize,
    probes: Vec<String>,
    /// With `--ruoli`: the rows taken out by each reference and whether the t0 is missing.
    reasons: String,
}

fn classify(rep: &mut Report, table: &mzlab_feat::table::Aligned, truth: &[Entry], t: &Tol) {
    rep.rows = table.len();
    if let Some(c) = &table.tp.roles {
        rep.reasons = format!(
            ",\"scartate_per\":{{\"bianco\":{},\"t0\":{},\"buio\":{}}},\"senza_t0\":{}",
            c.by_blank, c.by_t0, c.by_dark, c.no_t0
        );
    }
    for r in 0..table.len() {
        let name = status_name(table.tp.status[r]);
        *rep.by_status.entry(name.into()).or_default() += 1;
        let hits: Vec<usize> = (0..truth.len())
            .filter(|&k| near(t, table.mz[r], table.rt[r], truth[k].mz, &truth[k].rts))
            .collect();
        let hit = hits
            .iter()
            .copied()
            .find(|&k| truth[k].product)
            .or(hits.first().copied());
        let cat = hit.map_or("none".to_string(), |k| truth[k].cat.clone());
        *rep.by_cat
            .entry(name.into())
            .or_default()
            .entry(cat.clone())
            .or_default() += 1;
        if table.tp.status[r] == ST_TP {
            rep.products_seen
                .extend(hits.iter().copied().filter(|&k| truth[k].product));
            rep.inherited_in_view += usize::from(
                INHERITED.contains(&cat.as_str()) && hit.is_some_and(|k| !truth[k].product),
            );
        }
    }
}

fn probe(rep: &mut Report, table: &mzlab_feat::table::Aligned, probes: &[Value], t: &Tol) {
    for p in probes {
        let (mz, rt) = (num(p, "mz", f64::NAN), num(p, "rt", f64::NAN));
        let states: Vec<String> = (0..table.len())
            .filter(|&r| near(t, table.mz[r], table.rt[r], mz, &[rt]))
            .map(|r| format!("\"{}\"", status_name(table.tp.status[r])))
            .collect();
        rep.probes.push(format!(
            "{{\"mz\":{mz},\"rt\":{rt},\"rows\":[{}]}}",
            states.join(",")
        ));
    }
}

/// Runs the pipeline on the chosen files (indices into the stored detections) and reads back the view.
fn run_config(ctx: &Ctx, keep: &[usize]) -> Result<mzlab_feat::table::Aligned, String> {
    let refs: Vec<&[u8]> = keep.iter().map(|&k| ctx.stored[k].as_slice()).collect();
    let (feats, origin) = run::read_files(&refs).map_err(|e| e.to_string())?;
    let table = run::align(&feats, &origin, &ctx.recipe, |k| {
        Ok(open(&ctx.files[keep[k]].path))
    })
    .map_err(|e| e.to_string())?;
    let mut ix: Vec<_> = keep.iter().map(|&k| open(&ctx.files[k].path)).collect();
    let fr: Vec<_> = feats.iter().collect();
    let rules = std::fs::read(&ctx.rules).expect("rules");
    let h = run::hypotheses(
        &mut ix,
        &fr,
        &rules,
        &ctx.structure,
        &ctx.formula,
        Some(ctx.parent.1),
        &ctx.recipe,
    )
    .map_err(|e| e.to_string())?;
    let mins: Vec<f64> = keep.iter().map(|&k| minutes_of(&ctx.files[k])).collect();
    let parent = parent_row(&table, ctx).unwrap_or(ctx.parent);
    let roles = ctx.by_role.then(|| roles_of(ctx, keep));
    let mut ev = Vec::new();
    let budget = mzlab_core::budget::MemBudget::mb(mzlab_core::budget::NATIVE_BUDGET_MB as f64);
    let mut at = mzlab_feat::store::Slice(&table);
    let size = table.len() as u64;
    run::evidence_roles_to(
        &mut at,
        size,
        &h,
        &mins,
        roles.as_ref(),
        Some(parent),
        &ctx.recipe,
        &budget,
        &mut ev,
    )
    .map_err(|e| e.to_string())?;
    let (t, _) = read_table(&ev).map_err(|e| e.to_string())?;
    let (treated, others) = match &roles {
        Some(ro) => (
            ro.treated.len(),
            ro.t0.len() + ro.blank.len() + ro.dark.len(),
        ),
        None => (
            mins.iter().filter(|&&m| m > 0.0).count(),
            mins.iter().filter(|&&m| m <= 0.0 || m.is_nan()).count(),
        ),
    };
    if treated == 0 || others == 0 {
        return Err("error.tpNoControls".into());
    }
    if t.tp.is_empty() {
        let (pmz, prt) = ctx.parent;
        let rows =
            (0..t.len()).filter(|&r| (t.mz[r] - pmz).abs() < 1.0 && (t.rt[r] - prt).abs() < 0.5);
        for r in rows {
            eprintln!("no parent: nearest row m/z {} RT {}", t.mz[r], t.rt[r]);
        }
        return Err("error.tpNoParent".into());
    }
    Ok(t)
}

/// The roles of the files of the run as `campioni/2` gives them to the TP path (times confirmed, as after the loading table).
fn roles_of(ctx: &Ctx, keep: &[usize]) -> TpRoles {
    let cs: Vec<Campione> = keep
        .iter()
        .enumerate()
        .map(|(i, &k)| {
            let f = &ctx.files[k];
            let mut c = Campione::new(f.path.rsplit('/').next().unwrap_or(""), "", i as u32 + 1);
            c.kind = if f.role == "treated" {
                "sample".into()
            } else {
                f.role.clone()
            };
            match f.role.as_str() {
                "treated" => c.set_time(Some(f.minutes), "min"),
                "t0" => c.set_time(Some(0.0), "min"),
                _ => {}
            }
            c.time_confirmed = true;
            c
        })
        .collect();
    TpRoles::of(&cs)
}

/// The row of the table closest to the parent given (within the matching window), as the page picks it: its m/z moves a little with the files of the run.
fn parent_row(table: &[u8], ctx: &Ctx) -> Option<(f64, f64)> {
    let (t, _) = read_table(table).ok()?;
    let (mz, rt) = ctx.parent;
    (0..t.len())
        .filter(|&r| near(&ctx.tol, t.mz[r], t.rt[r], mz, &[rt]))
        .min_by(|&a, &b| (t.mz[a] - mz).abs().total_cmp(&(t.mz[b] - mz).abs()))
        .map(|r| (t.mz[r], t.rt[r]))
}

struct Ctx {
    tol: Tol,
    files: Vec<FileSpec>,
    stored: Vec<Vec<u8>>,
    recipe: mzlab_feat::recipe::Recipe,
    rules: String,
    structure: String,
    formula: String,
    parent: (f64, f64),
    /// `--ruoli`: the run gets the roles of the files.
    by_role: bool,
}

fn render(name: &str, keep: &[usize], ctx: &Ctx, out: Result<Report, String>) -> String {
    let names: Vec<String> = keep
        .iter()
        .map(|&k| format!("\"{}\"", ctx.files[k].path.rsplit('/').next().unwrap_or("")))
        .collect();
    let head = format!("{{\"config\":\"{name}\",\"files\":[{}]", names.join(","));
    match out {
        Err(e) => format!("{head},\"error\":\"{e}\"}}"),
        Ok(r) => {
            let cats: Vec<String> = r
                .by_cat
                .iter()
                .map(|(s, m)| format!("\"{s}\":{}", object(m)))
                .collect();
            format!(
                "{head},\"rows\":{},\"status\":{},\"truth_by_status\":{{{}}},\"products_in_view\":{},\"inherited_in_view\":{},\"probes\":[{}]{}}}",
                r.rows,
                object(&r.by_status),
                cats.join(","),
                r.products_seen.len(),
                r.inherited_in_view,
                r.probes.join(","),
                r.reasons
            )
        }
    }
}

fn selection(files: &[FileSpec], drop: &str) -> Vec<usize> {
    (0..files.len())
        .filter(|&k| match drop {
            "" => true,
            "*" => files[k].role == "t0" || files[k].role == "treated",
            role => files[k].role != role,
        })
        .collect()
}

fn main() {
    let path = std::env::args().nth(1).expect("SPEC.json");
    let spec = load(&path);
    let files = files_of(&spec);
    let parent = spec.get("parent").expect("parent");
    let rules = text(&spec, "rules");
    let truth = spec
        .get("truth")
        .and_then(Value::str)
        .map(|p| truth_of(&load(p)))
        .unwrap_or_default();
    let probes = spec.get("probes").map(Value::arr).unwrap_or(&[]);
    let tol = tol_of(&spec);
    let recipe = run::recipe_for("", &mut open(&files[0].path)).expect("recipe");
    let stored = files
        .iter()
        .map(|f| {
            run::detect(
                &mut open(&f.path),
                f.path.rsplit('/').next().unwrap_or(""),
                &recipe,
            )
            .expect("detect")
        })
        .collect();
    let structure = match text(parent, "smiles").as_str() {
        "" => String::new(),
        s => mzlab_feat::chem::grafo::analyse_json(s).expect("smiles"),
    };
    let ctx = Ctx {
        files,
        stored,
        recipe,
        rules,
        structure,
        formula: text(parent, "formula"),
        parent: (num(parent, "mz", f64::NAN), num(parent, "rt", f64::NAN)),
        tol: tol.clone(),
        by_role: std::env::args().any(|a| a == "--ruoli"),
    };
    let mut lines = Vec::new();
    for (name, drop) in CONFIGS {
        let keep = selection(&ctx.files, drop);
        let absent = !drop.is_empty() && drop != "*" && keep.len() == ctx.files.len();
        if absent {
            lines.push(format!("{{\"config\":\"{name}\",\"assente\":true}}"));
            continue;
        }
        let out = run_config(&ctx, &keep).map(|t| {
            let mut rep = Report::default();
            classify(&mut rep, &t, &truth, &tol);
            probe(&mut rep, &t, probes, &tol);
            rep
        });
        lines.push(render(name, &keep, &ctx, out));
    }
    let total = truth.iter().filter(|e| e.product).count();
    println!(
        "{{\"products_in_truth\":{total},\"configs\":[\n{}\n]}}",
        lines.join(",\n")
    );
}
