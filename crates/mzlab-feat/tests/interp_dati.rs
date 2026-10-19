//! The full `interp` pass over the real series of the private repository (`dati/HRMS/Timolol/`): the mzML files are indexed, the table is built and the
//! rows are interpreted without error. Skipped without the data repository (`MZLAB_DATI`). The literature products and sites in the private repository
//! are hypotheses found by hand, never the truth: nothing here asserts agreement with them (`tools/valida/tp_timololo.py` measures it). The metrics go
//! to stdout as `METRIC name=value`, and to the file in `MZLAB_MISURE_OUT` when set.
#![cfg(feature = "interp")]

use std::path::PathBuf;

use mzlab_core::elements::{mass, parse_formula};
use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare;
use mzlab_feat::famalign::align_families;
use mzlab_feat::family::{analyse_families, FamParams};
use mzlab_feat::interp::filtri::{decoy_masses, DECOY_PPM};
use mzlab_feat::interp::run::run_to;
use mzlab_feat::pipeline::{analyse, Steps};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{read_table, write_table, Origin, Slice};
use mzlab_feat::tp::{TpBlock, NO_PARENT, ST_PARENT, ST_TP};
use mzlab_feat::trace::Params;

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

const PARENT: &str = r#"{"neutral":"C13H24N4O3S"}"#;
const PROTON: f64 = 1.007_276;
// the net changes the rules of the registry would propose for the parent: +O, +2O, -CH2, -2H
const RULES: [&str; 4] = ["C13H24N4O4S", "C13H24N4O5S", "C12H22N4O3S", "C13H22N4O3S"];

fn files() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = dati_percorsi::percorso("dati/HRMS/Timolol")
        .into_iter()
        .flat_map(|d| std::fs::read_dir(d).into_iter().flatten().flatten())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "mzML"))
        .collect();
    v.sort();
    v
}

fn index_of(path: &PathBuf) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap();
    let mut src = bytes.clone();
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    let p = prepare(&mut src, bytes.len() as u64, &name).unwrap();
    build_mzml_to_vec(&p.table, &bytes, 256 << 20).unwrap().0
}

fn registry() -> (String, Vec<f64>) {
    let mz: Vec<f64> = RULES
        .iter()
        .map(|f| mass(&parse_formula(f).unwrap()) + PROTON)
        .collect();
    let items: Vec<String> = RULES.iter().zip(&mz).enumerate().map(|(i, (f, m))| {
        format!(r#"{{"id":{},"label":"r{i}","formula":"{f}","mz":{m},"level":3,"sources":["regola"]}}"#, i + 1)
    }).collect();
    (format!(r#"{{"hypotheses":[{}]}}"#, items.join(",")), mz)
}

/// The metrics as JSON: counts of the pass, the decoys of `filtri/1` and one line per interpreted row (m/z, RT, change, confidence of the position).
fn report(t: &mzlab_feat::table::Aligned, sink: &[u8]) -> String {
    use mzlab_core::json::parse;
    use mzlab_feat::interp::store;
    let size = sink.len() as u64;
    let (meta, ids) = store::read_meta(&mut Slice(sink), size)
        .unwrap()
        .expect("the block is there");
    let decoys = meta
        .get("filtri")
        .and_then(|f| f.get("decoys"))
        .map_or("null".into(), |d| {
            format!(
                "{{\"rules\":{},\"survived\":{}}}",
                d.get("rules").and_then(|v| v.f64()).unwrap_or(0.0),
                d.get("survived").and_then(|v| v.f64()).unwrap_or(0.0)
            )
        });
    let mut rows = vec![];
    for id in ids {
        let text = store::read_row(&mut Slice(sink), size, id)
            .unwrap()
            .unwrap();
        let v = parse(text.as_bytes()).unwrap();
        let pick = |a: &str, b: &str| {
            v.get(a)
                .and_then(|x| x.get(b))
                .and_then(|x| x.str())
                .unwrap_or("")
                .to_string()
        };
        let region = v
            .get("site")
            .and_then(|x| x.get("region"))
            .and_then(|x| x.get("name"))
            .and_then(|x| x.str())
            .unwrap_or("")
            .to_string();
        let r = (0..t.len()).find(|&r| t.id(r) == id).unwrap();
        rows.push(format!(
            "{{\"id\":{id},\"mz\":{:.5},\"rt\":{:.3},\"delta\":\"{}\",\"confidence\":\"{}\",\"region\":\"{region}\"}}",
            t.mz[r],
            t.rt[r],
            pick("change", "delta"),
            pick("site", "confidence")
        ));
    }
    format!("{{\"rows\":[{}],\"decoys\":{decoys}}}", rows.join(","))
}

#[test]
fn the_full_pass_runs_on_the_real_series() {
    let files = files();
    if files.is_empty() {
        eprintln!(
            "SKIP interp_dati: no data repository (MZLAB_DATI) or no mzML in dati/HRMS/Timolol"
        );
        return;
    }
    let idx: Vec<Vec<u8>> = files.iter().map(index_of).collect();
    let open = |k: usize| Index::open(idx[k].clone(), idx[k].len() as u64);
    let feats: Vec<_> = (0..idx.len())
        .map(|k| {
            let mut ix = open(k).unwrap();
            let (mut f, links) = analyse(&mut ix, &Params::default(), &Steps::default()).unwrap();
            analyse_families(
                &mut ix,
                &mut f,
                &links,
                &Params::default(),
                &FamParams::default(),
            )
            .unwrap();
            f
        })
        .collect();
    let mut t =
        align_families(&feats.iter().collect::<Vec<_>>(), &Recipe::default().align).unwrap();
    let (registry, rule_mz) = registry();
    let parent_mz = mass(&parse_formula("C13H24N4O3S").unwrap()) + PROTON;
    let Some(parent) = (0..t.len()).find(|&r| (t.mz[r] - parent_mz).abs() / parent_mz * 1e6 <= 5.0)
    else {
        eprintln!("SKIP interp_dati: the parent ion is not in the table");
        return;
    };
    t.tp = TpBlock {
        status: vec![ST_TP; t.len()],
        notes: vec![0; t.len()],
        via: vec![0; t.len()],
        kinship: vec![0; t.len()],
        parent_row: parent as u32,
        ..TpBlock::default()
    };
    t.tp.status[parent] = ST_PARENT;
    assert_ne!(t.tp.parent_row, NO_PARENT);
    let bytes = write_table(&t, &Recipe::default(), &Origin::default());
    let mut sink = Vec::new();
    run_to(
        &mut Slice(&bytes),
        bytes.len() as u64,
        PARENT,
        &registry,
        1,
        &mut |k| open(k),
        None::<&mut Index<Vec<u8>>>,
        &mut sink,
    )
    .unwrap();
    let (back, _) = read_table(&sink).unwrap();
    assert_eq!(back.to_bytes(), t.to_bytes(), "the table reads as before");
    // each decoy lies beyond 10 ppm of every rule
    let decoys = decoy_masses(&rule_mz);
    for d in &decoys {
        assert!(rule_mz.iter().all(|r| (d - r).abs() / r * 1e6 > DECOY_PPM));
    }
    assert!(!decoys.is_empty());
    let json = report(&t, &sink);
    println!("METRICS {json}");
    if let Some(p) = std::env::var_os("MZLAB_MISURE_OUT") {
        std::fs::write(p, json).unwrap();
    }
}
