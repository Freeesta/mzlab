//! `export.table_csv` and `export.mztab` in a plan: a `.mzfeat` written by the real store on invented rows gives the same bytes as the page asks of
//! the table `.wasm` (`table_file`), for each kind of file, and a second run writes them again.

use std::path::Path;

use mzlab_core::budget::sha256_hex;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::samples::{self, propose};
use mzlab_feat::store::{write_table, Origin, Slice};
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};
use mzlab_feat_table_wasm::core::table_file;

const NAMES: [&str; 3] = ["blank.mzML", "s_t0.mzML", "s_t10.mzML"];

fn table() -> Vec<u8> {
    table_in(Recipe::default())
}

fn table_in(recipe: Recipe) -> Vec<u8> {
    let rows: [(f64, f64, [f64; 3]); 3] = [
        (200.0, 5.0, [1e6, 1e6, 2e6]),
        (250.1, 6.0, [0.0, 5e5, 9e5]),
        (300.2, 7.0, [8e5, 9e5, 8e5]),
    ];
    let mut t = Aligned::with_capacity(3, rows.len());
    for (mz, rt, areas) in rows {
        t.mz.push(mz);
        t.rt.push(rt);
        for a in areas {
            t.area.push(a);
            t.height.push((a / 10.0) as f32);
            t.apex_rt.push(rt as f32);
            t.width.push(0.1);
            t.state
                .push(if a > 0.0 { CELL_DETECTED } else { CELL_ABSENT });
        }
    }
    write_table(&t, &recipe, &Origin::default())
}

fn samples_text() -> String {
    let files: Vec<(String, String)> = NAMES
        .iter()
        .map(|n| (n.to_string(), "00".repeat(32)))
        .collect();
    samples::to_json(&propose(&files))
}

fn dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_export_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    std::fs::write(d.join("table.mzfeat"), table()).unwrap();
    std::fs::write(d.join("samples.json"), samples_text()).unwrap();
    d
}

/// Runs one export step of the plan and gives the text of the file it wrote.
fn run(d: &Path, step: &str, params: &str, file: &str) -> Result<String, String> {
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"table.mzfeat","type":"table"}},{{"name":"samples.json","type":"samples"}}],"steps":[
        {{"id":"e","step":"{step}","in":{{"table":"file:table.mzfeat","samples":"file:samples.json"}},"params":{params}}}]}}"#
    );
    let (run, prov) = crate::flow::run_plan(plan.as_bytes(), d, &d.join("work"), 1, None)
        .map_err(|e| e.key.to_string())?;
    if !run.skipped.is_empty() || prov.contains("\"error\"") {
        return Err(prov);
    }
    Ok(std::fs::read_to_string(d.join("work").join(file)).unwrap())
}

fn page_of(t: &[u8], pol: i32, kind: &str) -> String {
    table_file(
        &mut Slice(t),
        t.len() as u64,
        &samples_text(),
        "",
        &sha256_hex(t),
        pol,
        kind,
    )
    .unwrap()
}

fn page(kind: &str) -> String {
    page_of(&table(), 1, kind)
}

#[test]
fn every_kind_of_csv_is_the_file_of_the_page() {
    let d = dir("csv");
    for kind in ["full", "full_mzmine", "quant", "csv_semicolon"] {
        let got = run(
            &d,
            "export.table_csv@1",
            &format!(r#"{{"kind":"{kind}"}}"#),
            "e.csv",
        )
        .unwrap();
        assert_eq!(got, page(kind), "{kind}");
        assert_eq!(
            got,
            run(
                &d,
                "export.table_csv@1",
                &format!(r#"{{"kind":"{kind}"}}"#),
                "e.csv"
            )
            .unwrap()
        );
    }
    // no parameter writes the "full" file
    assert_eq!(
        run(&d, "export.table_csv@1", "{}", "e.csv").unwrap(),
        page("full")
    );
    assert!(page("full").contains("datafile:s_t0:area"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn mztab_is_the_file_of_the_page() {
    let d = dir("mztab");
    let got = run(&d, "export.mztab@1", "{}", "e.mzTab").unwrap();
    assert_eq!(got, page("mztab"));
    assert!(got.contains("MTD\tmzTab-version\t2.0.0-M"), "{got}");
    assert_eq!(got, run(&d, "export.mztab@1", "{}", "e.mzTab").unwrap());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_view_of_a_filter_is_refused_with_its_key() {
    let d = dir("view");
    std::fs::write(
        d.join("table.mzfeat"),
        r#"{"digest":"x","righe":[],"fattori":[],"conteggi":{}}"#,
    )
    .unwrap();
    assert!(run(&d, "export.mztab@1", "{}", "e.mzTab").is_err());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_polarity_of_the_recipe_reaches_the_file() {
    let d = dir("neg");
    let mut recipe = Recipe::default();
    recipe.detect.polarity = Some(-1);
    let t = table_in(recipe);
    std::fs::write(d.join("table.mzfeat"), &t).unwrap();
    let got = run(&d, "export.mztab@1", "{}", "e.mzTab").unwrap();
    assert_eq!(got, page_of(&t, -1, "mztab"));
    assert_ne!(got, page_of(&t, 1, "mztab"));
    let _ = std::fs::remove_dir_all(&d);
}
