//! `export.annotations@1` in a plan: the CSV `flow run` writes is the text of the engine called by hand, `--set` reaches the parameters and an unwired optional port is left out.

use std::path::{Path, PathBuf};

use mzlab_feat::flow_export_annot::{export_made, Inputs, Params};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_DETECTED};

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_exannot_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    d
}

fn table() -> Vec<u8> {
    let mut t = Aligned::with_capacity(1, 2);
    for (mz, rt) in [(200.1, 1.5), (250.2, 2.5)] {
        t.mz.push(mz);
        t.rt.push(rt);
        t.area.push(1e6);
        t.height.push(1e5);
        t.apex_rt.push(rt as f32);
        t.width.push(0.1);
        t.state.push(CELL_DETECTED);
    }
    write_table(&t, &Recipe::default(), &Origin::default())
}

const HITS: &str = r#"{"format":"candidates/1","step":"spectra.match@1","algorithm":"spectral_entropy","hits":[{"query":"1","id":"L","name":"A, b","inchikey":"AAAAAAAAAAAAAA-BBBBBBBBBB-N","smiles":"C","formula":"C2H6","score":0.9,"shared":5,"mass_ppm":1.25,"precursor_delta":0,"source":"library","algorithm":"spectral_entropy"}]}"#;
const ANNOT: &str =
    r#"{"format":"annot/1","rows":[{"id":"1","level":"2a","why":"2a","gates":[]}]}"#;

fn plan(params: &str) -> String {
    format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"t.mzfeat","type":"table"}},{{"name":"hits.json","type":"candidates"}},{{"name":"annot.json","type":"annot"}}],"steps":[{{"id":"x","step":"export.annotations@1","in":{{"table":"file:t.mzfeat","candidates":["file:hits.json"],"annot":"file:annot.json"}},"params":{params}}}]}}"#
    )
}

fn run(d: &Path, params: &str) -> String {
    let (run, prov) =
        crate::flow::run_plan(plan(params).as_bytes(), d, &d.join("work"), 1, None).unwrap();
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    std::fs::read_to_string(d.join("work/x.csv")).unwrap()
}

#[test]
fn the_plan_writes_the_csv_of_the_engine_and_its_parameters_reach_it() {
    let d = dir("plan");
    let t = table();
    std::fs::write(d.join("t.mzfeat"), &t).unwrap();
    std::fs::write(d.join("hits.json"), HITS).unwrap();
    std::fs::write(d.join("annot.json"), ANNOT).unwrap();
    let inputs = Inputs {
        table: &t,
        candidates: &[("hits.json", HITS.as_bytes())],
        annot: Some(("annot.json", ANNOT.as_bytes())),
        ..Inputs::default()
    };
    let by_hand = export_made(&Params::from_params(&[]), &inputs).unwrap().csv;
    assert_eq!(run(&d, "{}"), by_hand);
    assert!(by_hand.contains("\"A, b\""), "{by_hand}");
    let semi = run(&d, r#"{"separator":"semicolon"}"#);
    assert!(semi.starts_with("feature_id;mz;rt_min;"), "{semi}");
    let high = run(&d, r#"{"min_level":"1"}"#);
    assert_eq!(high.lines().count(), 1, "{high}");
}
