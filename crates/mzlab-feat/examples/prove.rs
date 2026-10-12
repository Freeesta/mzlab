//! The graph of the proofs on a series of `.mzidx` files: analysis, families, alignment per family, hypotheses of the parent, kinetics, `evid/1`.
//!
//!     cargo run --release -p mzlab-feat --example prove -- <file.mzidx>... --parent-formula FORMULA [--truth truth.json] [--out FILE.mzfeat] [--twice]
//!
//! The file names give the times (`tNNNmin` = treated, `dark` = control). With a truth file (`parent {mz, rt_obs}`, `tps [{id, mz, rt_obs[]}]`) it
//! prints, for the candidates (rows that confirm a hypothesis), the rank of the true products ordered by the largest area and ordered by the graph, and
//! how many candidates are needed to recover 90 % of those found. `--twice` builds the `.mzfeat` a second time and compares the bytes.

use std::path::{Path, PathBuf};

use mzlab_core::idxread::Index;
use mzlab_feat::evid::*;
use mzlab_feat::family::{analyse_families, FamParams, Families};
use mzlab_feat::hypo::{generate, HypoParams, Inputs as HypoInputs, Registry, Rules};
use mzlab_feat::kinetic::{parent_kinetics, ChainBank, FormBank, Setup, K_PARENT_MIN};
use mzlab_feat::pipeline::analyse;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::series::{align_and_fill, source_sha256};
use mzlab_feat::store::{read_evid, write_table_with_evid, Origin};
use mzlab_feat::table::{Aligned, FileFeatures};
use serde_json::Value;

type Idx = Index<std::fs::File>;

fn open(path: &str) -> Idx {
    let f = std::fs::File::open(path).expect("open");
    let size = f.metadata().expect("size").len();
    Index::open(f, size).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// Minutes of `...t030min...`; `None` for a control.
fn minutes(name: &str) -> Option<f64> {
    let k = name.rfind("_t")? + 2;
    name[k..].split("min").next()?.parse().ok()
}

struct Series {
    names: Vec<String>,
    feats: Vec<FileFeatures>,
    fams: Vec<Families>,
    origin: Origin,
}

fn analyse_all(files: &[String], r: &Recipe) -> Series {
    let mut s = Series {
        names: vec![],
        feats: vec![],
        fams: vec![],
        origin: Origin::default(),
    };
    for path in files {
        let mut ix = open(path);
        let name = path.rsplit('/').next().unwrap_or(path).to_string();
        s.origin.inputs.push((name.clone(), source_sha256(&ix)));
        let (mut f, links) = analyse(&mut ix, &r.detect, &r.steps).expect("analyse");
        let fam = analyse_families(&mut ix, &mut f, &links, &r.detect, &FamParams::default())
            .expect("families");
        s.names.push(name);
        s.feats.push(f);
        s.fams.push(fam);
    }
    s
}

fn row_near(t: &Aligned, mz: f64, rt: f64, rt_tol: f64) -> Option<usize> {
    let tol = mz * 5e-6;
    (0..t.len())
        .filter(|&r| (t.mz[r] - mz).abs() <= tol && (t.rt[r] - rt).abs() <= rt_tol)
        .max_by(|&a, &b| area_of(t, a).total_cmp(&area_of(t, b)))
}

fn area_of(t: &Aligned, r: usize) -> f64 {
    t.area[r * t.n_files..(r + 1) * t.n_files]
        .iter()
        .copied()
        .filter(|a| a.is_finite())
        .fold(0.0, f64::max)
}

struct Truth {
    parent: (f64, f64),
    tps: Vec<(String, f64, Vec<f64>)>,
}

fn load_truth(path: &str) -> Truth {
    let v: Value = serde_json::from_slice(&std::fs::read(path).expect("truth")).expect("json");
    let rts = |x: &Value| -> Vec<f64> {
        match &x["rt_obs"] {
            Value::Array(a) => a.iter().filter_map(Value::as_f64).collect(),
            n => n.as_f64().into_iter().collect(),
        }
    };
    Truth {
        parent: (v["parent"]["mz"].as_f64().unwrap(), rts(&v["parent"])[0]),
        tps: v["tps"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| {
                (
                    x["id"].as_str().unwrap().to_string(),
                    x["mz"].as_f64().unwrap(),
                    rts(x),
                )
            })
            .collect(),
    }
}

fn registry(formula: &str, neg: bool) -> Registry {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let rules =
        Rules::parse(&std::fs::read(root.join("TP_Mine/data/regole_tp.json")).unwrap()).unwrap();
    let inp = HypoInputs {
        parent: formula,
        suspects: None,
        families: &[],
        imported: &[],
        fragments: &[],
    };
    let p = HypoParams {
        polarity: if neg { -1 } else { 1 },
        ..HypoParams::default()
    };
    generate(&rules, &inp, &p).expect("hypotheses")
}

/// Builds the graph of the series; returns the table, the identities and the `.mzfeat` bytes.
fn build(
    files: &[String],
    r: &Recipe,
    formula: &str,
    truth: Option<&Truth>,
) -> (Aligned, Vec<Identity>, Vec<u8>) {
    let s = analyse_all(files, r);
    let (aligned, _) = align_and_fill(&s.feats, r, |k| Ok(open(&files[k]))).expect("align");
    let arcs = arcs_from_families(
        &aligned,
        &s.fams
            .iter()
            .zip(&s.feats)
            .map(|(fam, feats)| FileFamilies { feats, fam })
            .collect::<Vec<_>>(),
    );
    let reg = registry(formula, r.detect.polarity == Some(-1));
    let table = PointsTable::bundled().unwrap();
    let treated: Vec<usize> = (0..files.len())
        .filter(|&k| minutes(&s.names[k]).is_some())
        .collect();
    let controls: Vec<usize> = (0..files.len())
        .filter(|&k| minutes(&s.names[k]).is_none())
        .collect();
    let times: Vec<f64> = treated
        .iter()
        .map(|&k| minutes(&s.names[k]).unwrap())
        .collect();
    let (forms, chain) = (FormBank::new(&times), ChainBank::new(&times, K_PARENT_MIN));
    let parent_at = truth.map(|t| t.parent);
    let y: Vec<f64> = parent_at
        .and_then(|(mz, rt)| row_near(&aligned, mz, rt, 0.3))
        .map(|row| {
            treated
                .iter()
                .map(|&k| aligned.area[row * aligned.n_files + k])
                .collect()
        })
        .unwrap_or_else(|| vec![1.0; times.len()]);
    let setup = Setup {
        times: &times,
        forms: &forms,
        chain: &chain,
        parent: parent_kinetics(&forms, &times, &y, None),
        pseudo: 2e4 * 3.0,
    };
    let inp = Inputs {
        table: &table,
        aligned: &aligned,
        facts: &[],
        arcs: &arcs,
        registry: &reg,
        libs: &[],
        external: &[],
        kinetics: Some(Kinetics {
            setup: &setup,
            treated: &treated,
            controls: &controls,
        }),
        parent: parent_at,
    };
    let ids = build_graph(&inp);
    let json = graph_json(&ids, &table);
    println!(
        "{} rows, {} hypotheses, {} identities, {} arcs, evid/1 {} bytes",
        aligned.len(),
        reg.len(),
        ids.len(),
        arcs.len(),
        json.len()
    );
    let bytes = write_table_with_evid(&aligned, r, &s.origin, json.as_bytes());
    (aligned, ids, bytes)
}

fn report(aligned: &Aligned, ids: &[Identity], truth: &Truth) {
    let cands: Vec<&Identity> = ids.iter().filter(|i| i.hypothesis.is_some()).collect();
    let mut by_area: Vec<u32> = cands.iter().map(|i| i.id).collect();
    by_area.sort_by(|&a, &b| area_of(aligned, b as usize).total_cmp(&area_of(aligned, a as usize)));
    let by_graph: Vec<u32> = cands.iter().map(|i| i.id).collect();
    let pos =
        |list: &[u32], row: usize| list.iter().position(|&x| x as usize == row).map(|p| p + 1);
    let (mut found, mut ra, mut rg) = (0, vec![], vec![]);
    for (id, mz, rts) in &truth.tps {
        let row = rts.iter().find_map(|&rt| row_near(aligned, *mz, rt, 0.3));
        let (a, g) = row.map_or((None, None), |r| (pos(&by_area, r), pos(&by_graph, r)));
        println!("{id:>8}: row {row:?} rank by area {a:?}, by graph {g:?}");
        found += usize::from(row.is_some());
        ra.extend(a);
        rg.extend(g);
    }
    let stat = |mut v: Vec<usize>| {
        v.sort_unstable();
        let n90 = v
            .get(((v.len() * 9).div_ceil(10)).saturating_sub(1))
            .copied();
        (v.get(v.len() / 2).copied(), n90)
    };
    println!(
        "truth: {} products, {found} in the table, {} among {} candidates",
        truth.tps.len(),
        ra.len(),
        cands.len()
    );
    println!(
        "median rank / candidates for 90 %: by area {:?}, by graph {:?}",
        stat(ra),
        stat(rg)
    );
}

fn main() {
    let (mut files, mut out, mut formula, mut truth_path) = (
        Vec::<String>::new(),
        None::<PathBuf>,
        String::new(),
        None::<String>,
    );
    let (mut twice, mut r) = (false, Recipe::default());
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--neg" => r.detect.polarity = Some(-1),
            "--parent-formula" => formula = args.next().expect("formula"),
            "--truth" => truth_path = args.next(),
            "--out" => out = args.next().map(PathBuf::from),
            "--twice" => twice = true,
            _ => files.push(a),
        }
    }
    let truth = truth_path.as_deref().map(load_truth);
    let (aligned, ids, bytes) = build(&files, &r, &formula, truth.as_ref());
    assert_eq!(read_evid(&bytes).expect("evid").is_empty(), false);
    println!("{} bytes of .mzfeat", bytes.len());
    if let Some(t) = &truth {
        report(&aligned, &ids, t);
    }
    if twice {
        let again = build(&files, &r, &formula, truth.as_ref()).2;
        println!(
            "second run: {}",
            if again == bytes {
                "identical bytes"
            } else {
                "DIFFERENT"
            }
        );
    }
    if let Some(p) = out {
        std::fs::write(p, &bytes).expect("write");
    }
}
