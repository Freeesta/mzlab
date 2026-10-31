//! `table.filter@1` in a plan: a `.mzfeat` written by the real store on invented rows (a standard, a product, a contaminant in the blank, noise seen
//! once) gives the same view as the page asks of the table `.wasm` (`view_json` with the spec the page builds), and a second run comes from the cache.

use std::path::Path;

use mzlab_core::campioni;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::samples::{self, propose};
use mzlab_feat::store::{write_table, Origin, Slice};
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};
use mzlab_feat_table_wasm::core::view_json;

const NAMES: [&str; 4] = ["blank.mzML", "s_t0.mzML", "s_t10.mzML", "s_t20.mzML"];

fn table() -> Vec<u8> {
    let rows: [(f64, f64, [f64; 4]); 4] = [
        (200.0, 5.0, [1e6, 1e6, 2e6, 4e6]),
        (250.1, 6.0, [0.0, 0.0, 5e5, 9e5]),
        (300.2, 7.0, [8e5, 9e5, 8e5, 9e5]),
        (350.3, 8.0, [0.0, 0.0, 0.0, 1e3]),
    ];
    let mut t = Aligned::with_capacity(4, rows.len());
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
    write_table(&t, &Recipe::default(), &Origin::default())
}

fn files() -> Vec<(String, String)> {
    NAMES
        .iter()
        .map(|n| (n.to_string(), "00".repeat(32)))
        .collect()
}

fn dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_tfilter_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    d
}

fn run(d: &Path, params: &str) -> Result<String, String> {
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"table.mzfeat","type":"table"}},{{"name":"samples.json","type":"samples"}}],"steps":[
        {{"id":"f","step":"table.filter@1","in":{{"table":"file:table.mzfeat","samples":"file:samples.json"}},"params":{params}}}]}}"#
    );
    let (run, prov) = crate::flow::run_plan(plan.as_bytes(), d, &d.join("work"), 1, None)
        .map_err(|e| e.key.to_string())?;
    if !run.skipped.is_empty() || prov.contains("\"error\"") {
        return Err(prov);
    }
    Ok(std::fs::read_to_string(d.join("work/f.json")).unwrap())
}

fn page(table: &[u8], samples: &str, spec: &str) -> String {
    view_json(&mut Slice(table), table.len() as u64, samples, spec).unwrap()
}

#[test]
fn the_plan_gives_the_view_of_the_page() {
    let d = dir("page");
    let (t, s) = (table(), samples::to_json(&propose(&files())));
    std::fs::write(d.join("table.mzfeat"), &t).unwrap();
    std::fs::write(d.join("samples.json"), &s).unwrap();

    let got = run(
        &d,
        r#"{"min_detected":2,"min_area":100000,"blank_ratio":3,"normalize":"sum"}"#,
    )
    .unwrap();
    // the spec the page builds from the same fields (`37_feature_table.js`: detected, area, blank, then the normalisation)
    let spec = r#"{"normalizza":{"metodo":"somma"},"filtra":[{"regola":"min_rilevati","valore":2},{"regola":"min_area","valore":100000},{"regola":"bianco","valore":3}]}"#;
    assert_eq!(got, page(&t, &s, spec));
    assert!(got.contains("\"tenute\":2"), "{got}");
    // no rule and no normalisation keeps every row, as an empty spec does
    let all = run(&d, "{}").unwrap();
    assert_eq!(all, page(&t, &s, ""));
    // the second run reads the same bytes
    assert_eq!(
        got,
        run(
            &d,
            r#"{"min_detected":2,"min_area":100000,"blank_ratio":3,"normalize":"sum"}"#
        )
        .unwrap()
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn campioni_2_gives_the_same_view_and_a_rule_that_cannot_be_read_fails() {
    let d = dir("c2");
    let t = table();
    std::fs::write(d.join("table.mzfeat"), &t).unwrap();
    let named: Vec<_> = files()
        .into_iter()
        .map(|(n, s)| (n, s, String::new()))
        .collect();
    std::fs::write(
        d.join("samples.json"),
        campioni::to_json(&campioni::proponi(&named)),
    )
    .unwrap();
    let got = run(&d, r#"{"blank_ratio":3}"#).unwrap();
    let s1 = samples::to_json(&propose(&files()));
    assert_eq!(
        got,
        page(
            &t,
            &s1,
            r#"{"normalizza":{"metodo":"nessuna"},"filtra":[{"regola":"bianco","valore":3}]}"#
        )
    );
    // a series with no blank cannot evaluate the blank rule: the run ends in error, no silent pass
    let mut s = propose(&files());
    s[0].kind = "sample".into();
    std::fs::write(d.join("samples.json"), samples::to_json(&s)).unwrap();
    assert!(run(&d, r#"{"blank_ratio":3}"#).is_err());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_duplicate_window_keeps_the_strongest_row_and_matches_the_page() {
    let d = dir("dup");
    let (t, s) = (table(), samples::to_json(&propose(&files())));
    std::fs::write(d.join("table.mzfeat"), &t).unwrap();
    std::fs::write(d.join("samples.json"), &s).unwrap();
    // a window wide enough to hold the four rows leaves the row with the largest area
    let got = run(&d, r#"{"dup_ppm":1000000,"dup_rt":10}"#).unwrap();
    let spec = r#"{"filtra":[{"regola":"duplicati","ppm":1000000,"rt_min":10}]}"#;
    assert_eq!(got, page(&t, &s, spec));
    assert!(got.contains("\"tenute\":1"), "{got}");
    // the window alone, with the default RT distance of 0.1 min, drops nothing here
    let few = run(&d, r#"{"dup_ppm":5}"#).unwrap();
    assert!(few.contains("\"tenute\":4"), "{few}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_nts_rules_read_the_batches_of_campioni_3_and_match_the_page() {
    let d = dir("nts");
    let t = table();
    std::fs::write(d.join("table.mzfeat"), &t).unwrap();
    // two blanks (one per batch) and three samples; the samples file is campioni/3 with the batch of each file
    let named: Vec<_> = files()
        .into_iter()
        .map(|(n, s)| (n, s, String::new()))
        .collect();
    let mut cs = campioni::proponi(&named);
    for (c, b) in cs.iter_mut().zip(["B1", "B1", "B2", "B2"]) {
        c.batch = b.into();
    }
    let samples = campioni::to_json3(&campioni::Tabella {
        campioni: cs,
        colonne: Vec::new(),
    });
    std::fs::write(d.join("samples.json"), &samples).unwrap();
    let params = r#"{"blank_ratio":3,"blank_stat":"max","blank_match":"batch","min_replicate_fraction":0.5,"normalize":"istd","istd_id":1,"blank_keep_istd":true}"#;
    let got = run(&d, params).unwrap();
    // the spec the page builds: the rules in the order of the descriptor, the labels of the files read from the same document
    let spec = r#"{"normalizza":{"metodo":"istd","istd":{"id":1}},"filtra":[{"regola":"repliche_min","valore":0.5},{"regola":"bianco","valore":3,"statistica":"massimo","abbinamento":"lotto","istd_esente":true}],"etichette":{"lotto":["B1","B1","B2","B2"],"matrice":["","","",""],"serie":["","","",""],"gruppo":["","","",""]}}"#;
    let want = page(&t, &samples_text_v1(&samples), spec);
    assert_eq!(got, want);
    assert!(got.contains("\"bianco_ripiego\":["), "{got}");
    // the same through the entry the page calls
    let via =
        mzlab_feat_table_wasm::filterspec::spec_from_json_with_samples(params, samples.as_bytes())
            .unwrap();
    assert_eq!(page(&t, &samples_text_v1(&samples), &via), got);
    // defaults: the new parameters off give the bytes of the old plan
    assert_eq!(
        run(&d, r#"{"blank_ratio":3}"#).unwrap(),
        page(
            &t,
            &samples_text_v1(&samples),
            r#"{"normalizza":{"metodo":"nessuna"},"filtra":[{"regola":"bianco","valore":3}]}"#
        )
    );
    let _ = std::fs::remove_dir_all(&d);
}

fn samples_text_v1(doc: &str) -> String {
    mzlab_feat_table_wasm::pol::samples_v1(doc.as_bytes()).unwrap()
}
