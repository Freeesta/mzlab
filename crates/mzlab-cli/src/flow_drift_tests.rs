//! `table.drift@1` in a plan: the frozen series of `tests/golden/feat/deriva.json` (invented numbers, answers of the reference script) written as a `.mzfeat`
//! with its samples and batches gives the factors of the reference and the text of the engine called by hand; method and span reach the engine, a design
//! without QC files at the ends of a batch is refused, and a second run writes the same bytes.

use std::path::Path;

use mzlab_core::campioni::{self, Campione};
use mzlab_core::disegno::{self, Disegno, Lotto};
use mzlab_core::json::{parse, Value};
use mzlab_feat::drift::{batches_of, correct, Method};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};

/// A file of the series: kind, order of injection, batch.
type File = (String, u32, String);

fn golden() -> Value {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/feat/deriva.json");
    parse(&std::fs::read(p).unwrap()).unwrap()
}

/// Name and SHA-256 of the file `k`.
fn ident(k: usize) -> (String, String) {
    (format!("f{k}.mzML"), format!("{k:02x}").repeat(32))
}

/// The files of the series as (kind, order, batch) and the areas of its rows.
fn series() -> (Vec<File>, Vec<Vec<f64>>) {
    let g = golden();
    let files = g.get("files").unwrap().arr();
    let text = |f: &Value, k: &str| f.get(k).and_then(Value::str).unwrap().to_string();
    let files = files
        .iter()
        .map(|f| {
            let order = f.get("order").and_then(Value::f64).unwrap() as u32;
            (text(f, "kind"), order, text(f, "batch"))
        })
        .collect();
    let areas = g
        .get("areas")
        .unwrap()
        .arr()
        .iter()
        .map(|r| r.arr().iter().map(|x| x.f64().unwrap()).collect())
        .collect();
    (files, areas)
}

fn table(areas: &[Vec<f64>]) -> Vec<u8> {
    let n = areas[0].len();
    let mut t = Aligned::with_capacity(n, areas.len());
    for (r, row) in areas.iter().enumerate() {
        t.mz.push(200.0 + r as f64);
        t.rt.push(5.0);
        for &a in row {
            t.area.push(a);
            t.height.push((a / 10.0) as f32);
            t.apex_rt.push(5.0);
            t.width.push(0.1);
            t.state
                .push(if a > 0.0 { CELL_DETECTED } else { CELL_ABSENT });
        }
    }
    let origin = Origin {
        inputs: (0..n).map(ident).collect(),
        ..Origin::default()
    };
    write_table(&t, &Recipe::default(), &origin)
}

fn campioni_2(files: &[File]) -> Vec<Campione> {
    files
        .iter()
        .enumerate()
        .map(|(k, (kind, order, _))| {
            let (name, sha) = ident(k);
            let mut c = Campione::new(&name, &sha, *order);
            c.kind = kind.clone();
            c
        })
        .collect()
}

fn design(files: &[File]) -> Disegno {
    Disegno {
        conditions: Vec::new(),
        reference: None,
        comparisons: Vec::new(),
        factors: Vec::new(),
        batches: files
            .iter()
            .enumerate()
            .map(|(k, f)| Lotto {
                file: ident(k).0,
                sha256: ident(k).1,
                batch: f.2.clone(),
            })
            .collect(),
    }
}

fn dir(tag: &str, files: &[File], areas: &[Vec<f64>]) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_drift_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    std::fs::write(d.join("table.mzfeat"), table(areas)).unwrap();
    std::fs::write(d.join("design.json"), disegno::to_json(&design(files))).unwrap();
    std::fs::write(
        d.join("samples.json"),
        campioni::to_json(&campioni_2(files)),
    )
    .unwrap();
    d
}

fn plan(params: &str) -> String {
    format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"table.mzfeat","type":"table"}},{{"name":"samples.json","type":"samples"}},{{"name":"design.json","type":"design"}}],"steps":[
        {{"id":"d","step":"table.drift@1","in":{{"table":"file:table.mzfeat","samples":"file:samples.json","design":"file:design.json"}},"params":{params}}}]}}"#
    )
}

fn run(d: &Path, params: &str) -> String {
    let (run, prov) =
        crate::flow::run_plan(plan(params).as_bytes(), d, &d.join("work"), 1, None).unwrap();
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    std::fs::read_to_string(d.join("work/d.json")).unwrap()
}

/// The engine called by hand on the same series: its summary and the factors of every cell.
fn engine(files: &[File], areas: &[Vec<f64>], method: Method) -> String {
    let (_, meta) = mzlab_feat::store::read_table(&table(areas)).unwrap();
    let inputs = mzlab_feat::store::origin_from_meta(&meta).inputs;
    let json = campioni::to_json(&campioni_2(files));
    let all = crate::flow_stats::samples_of(json.as_bytes()).unwrap();
    let cols = crate::flow_stats::columns(&inputs, &all);
    let b = batches_of(&cols, &design(files));
    let (t, _) = mzlab_feat::store::read_table(&table(areas)).unwrap();
    let d = correct(&t, &cols, &b, method).unwrap();
    let mut text = d.summary_json(&cols);
    text.pop();
    let rows: Vec<String> = (0..t.len())
        .map(|r| {
            let f: Vec<String> = d.factor[r * t.n_files..(r + 1) * t.n_files]
                .iter()
                .map(|x| {
                    if x.is_finite() {
                        format!("{x}")
                    } else {
                        "null".into()
                    }
                })
                .collect();
            format!("[{}]", f.join(","))
        })
        .collect();
    format!("{text},\"factor\":[{}]}}", rows.join(","))
}

#[test]
fn the_plan_gives_the_factors_of_the_reference_and_the_engine_text() {
    let (files, areas) = series();
    let d = dir("engine", &files, &areas);
    let got = run(&d, "{}");
    assert_eq!(got, engine(&files, &areas, Method::Linear));
    let doc = parse(got.as_bytes()).unwrap();
    assert_eq!(doc.get("schema").and_then(Value::str), Some("deriva/1"));
    let want = golden();
    let case = &want.get("cases").unwrap().arr()[0];
    for (r, row) in case.get("factor").unwrap().arr().iter().enumerate() {
        for (k, w) in row.arr().iter().enumerate() {
            let g = doc.get("factor").unwrap().arr()[r].arr()[k].f64();
            match w.f64() {
                Some(w) => assert!((g.unwrap() - w).abs() <= 1e-9 * w.abs().max(1.0), "{r} {k}"),
                None => assert!(g.is_none(), "{r} {k}"),
            }
        }
    }
    assert_eq!(got, run(&d, "{}"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn method_and_span_reach_the_engine() {
    let (files, areas) = series();
    let d = dir("loess", &files, &areas);
    let loess = run(&d, r#"{"method":"loess","span":0.75}"#);
    assert_eq!(loess, engine(&files, &areas, Method::Loess(0.75)));
    assert!(loess.contains("\"method\":\"loess\""), "{loess}");
    assert_ne!(loess, run(&d, "{}"));
    let wide = run(&d, r#"{"method":"loess","span":1}"#);
    assert_eq!(wide, engine(&files, &areas, Method::Loess(1.0)));
    assert_ne!(wide, loess);
    let _ = std::fs::remove_dir_all(&d);
}

/// The library function the browser module calls (`mzlab_feat::flow_drift::drift_text`, bytes in, text out) writes the bytes of the file the plan writes.
#[test]
fn the_library_text_is_the_file_of_the_plan() {
    let (files, areas) = series();
    let d = dir("lib", &files, &areas);
    let (table, samples, design) = (
        std::fs::read(d.join("table.mzfeat")).unwrap(),
        std::fs::read(d.join("samples.json")).unwrap(),
        std::fs::read(d.join("design.json")).unwrap(),
    );
    for given in ["{}", r#"{"method":"loess","span":0.8}"#] {
        let from_plan = run(&d, given);
        let params = mzlab_feat::flow_ms2::json_params(given).unwrap();
        let from_lib =
            mzlab_feat::flow_drift::drift_text(&params, &table, &samples, &design).unwrap();
        assert_eq!(from_plan.as_bytes(), from_lib.as_bytes());
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// The error key the provenance records for the step when the correction is refused, and no output file.
fn refused(tag: &str, edit: impl Fn(&mut Vec<File>)) -> String {
    let (mut files, areas) = series();
    edit(&mut files);
    let d = dir(tag, &files, &areas);
    let (_, prov) =
        crate::flow::run_plan(plan("{}").as_bytes(), &d, &d.join("work"), 1, None).unwrap();
    assert!(!d.join("work/d.json").exists());
    let _ = std::fs::remove_dir_all(&d);
    prov
}

#[test]
fn a_batch_without_enough_qc_or_qc_at_its_ends_is_refused() {
    let few = refused("few", |f| f[0].0 = "sample".into());
    assert!(few.contains("error.drift.tooFewQc"), "{few}");
    let ends = refused("ends", |f| {
        f[0].0 = "sample".into();
        f[1].0 = "qc".into();
    });
    assert!(ends.contains("error.drift.qcNotAtEnds"), "{ends}");
}
