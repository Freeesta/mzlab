//! `table.stats@1` in a plan: a `.mzfeat` written by the real store on invented areas (a blank, a reference of three files, a treatment of three) with
//! its samples (`campioni/2` and `campioni/1`) and design gives the `stat/1` of the engine called by hand and of its JSON entry (`stats_json`).

use std::path::Path;

use mzlab_core::campioni::{self, Campione};
use mzlab_core::disegno::{self, Condizione, Confronto, Disegno};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::samples;
use mzlab_feat::stat::{compare, stats_json, write_stats, Layout, StatsOpts};
use mzlab_feat::store::{write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};

const NAMES: [&str; 7] = [
    "blank.mzML",
    "ref_1.mzML",
    "ref_2.mzML",
    "ref_3.mzML",
    "trt_1.mzML",
    "trt_2.mzML",
    "trt_3.mzML",
];

/// Areas of the features over the seven files: a few up in the treatment, one equal, one only in the blank, one barely detected.
fn rows() -> Vec<Vec<f64>> {
    vec![
        vec![0.0, 1.0e6, 1.1e6, 0.9e6, 4.2e6, 3.9e6, 4.4e6],
        vec![0.0, 2.0e6, 2.2e6, 2.1e6, 2.0e6, 2.3e6, 1.9e6],
        vec![5.0e3, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        vec![0.0, 3.0e4, 0.0, 2.0e4, 0.0, 0.0, 0.0],
        vec![0.0, 8.0e5, 7.5e5, 8.5e5, 1.0e5, 1.2e5, 0.9e5],
    ]
}

fn table() -> Vec<u8> {
    let rows = rows();
    let mut t = Aligned::with_capacity(NAMES.len(), rows.len());
    for (i, areas) in rows.iter().enumerate() {
        t.mz.push(200.0 + 10.0 * i as f64);
        t.rt.push(5.0 + i as f64);
        for &a in areas {
            t.area.push(a);
            t.height.push((a / 10.0) as f32);
            t.apex_rt.push(5.0);
            t.width.push(0.1);
            t.state
                .push(if a > 0.0 { CELL_DETECTED } else { CELL_ABSENT });
        }
    }
    let origin = Origin {
        inputs: NAMES
            .iter()
            .map(|n| (n.to_string(), String::new()))
            .collect(),
        ..Origin::default()
    };
    write_table(&t, &Recipe::default(), &origin)
}

fn campioni_2() -> Vec<Campione> {
    NAMES
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let mut c = Campione::new(n, &format!("{i:064x}"), i as u32 + 1);
            match i {
                0 => c.kind = "blank".into(),
                1..=3 => c.series = "ref".into(),
                _ => c.series = "trt".into(),
            }
            c
        })
        .collect()
}

fn design() -> Disegno {
    let condition = |name: &str| Condizione {
        name: name.into(),
        replicates: 3,
        levels: Vec::new(),
    };
    Disegno {
        conditions: vec![condition("ref"), condition("trt")],
        reference: Some("ref".into()),
        comparisons: vec![Confronto::VsReference],
        factors: Vec::new(),
        batches: Vec::new(),
    }
}

fn dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_stats_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    std::fs::write(d.join("table.mzfeat"), table()).unwrap();
    std::fs::write(d.join("design.json"), disegno::to_json(&design())).unwrap();
    std::fs::write(d.join("samples.json"), campioni::to_json(&campioni_2())).unwrap();
    d
}

fn run(d: &Path, params: &str) -> String {
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"table.mzfeat","type":"table"}},{{"name":"samples.json","type":"samples"}},{{"name":"design.json","type":"design"}}],"steps":[
        {{"id":"s","step":"table.stats@1","in":{{"table":"file:table.mzfeat","samples":"file:samples.json","design":"file:design.json"}},"params":{params}}}]}}"#
    );
    let (run, prov) = crate::flow::run_plan(plan.as_bytes(), d, &d.join("work"), 1, None).unwrap();
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    std::fs::read_to_string(d.join("work/s.json")).unwrap()
}

/// The engine called by hand on the same invented areas.
fn engine(pseudo: Option<f64>) -> String {
    let lay = Layout {
        names: vec!["ref".into(), "trt".into()],
        group_of: vec![None, Some(0), Some(0), Some(0), Some(1), Some(1), Some(1)],
        blank: vec![true, false, false, false, false, false, false],
        reference: Some(0),
    };
    let opts = StatsOpts {
        pseudo,
        ..StatsOpts::default()
    };
    write_stats(&compare(&rows(), &lay, &[(0, 1)], &opts).unwrap(), &lay)
}

#[test]
fn the_plan_gives_the_statistics_of_the_engine() {
    let d = dir("engine");
    let got = run(&d, "{}");
    assert!(got.starts_with("{\"schema\":\"stat/1\""), "{got}");
    assert_eq!(got, engine(None));
    // the JSON entry of the step says the same
    let groups = r#"[null,"ref","ref","ref","trt","trt","trt"]"#;
    let matrix: Vec<String> = rows().iter().map(|r| format!("{r:?}")).collect();
    let input = format!(
        r#"{{"matrix":[{}],"groups":{groups},"blank":[true,false,false,false,false,false,false],"reference":"ref"}}"#,
        matrix.join(",")
    );
    assert_eq!(got, stats_json(input.as_bytes()).unwrap());
    // the same bytes on a second run
    assert_eq!(got, run(&d, "{}"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_pseudocount_parameter_reaches_the_engine() {
    let d = dir("pseudo");
    let got = run(&d, r#"{"pseudocount":500}"#);
    assert_eq!(got, engine(Some(500.0)));
    assert_ne!(got, engine(None));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn campioni_1_gives_the_same_statistics() {
    let d = dir("v1");
    let v1: Vec<samples::Sample> = campioni_2()
        .into_iter()
        .map(|c| samples::Sample {
            kind: c.kind.clone(),
            group: c.series.clone(),
            file: c.file,
            sha256: c.sha256,
            time_min: None,
            replicate: 1,
            conc: None,
            unit: None,
            order: c.order,
            istd: false,
            note: String::new(),
        })
        .collect();
    std::fs::write(d.join("samples.json"), samples::to_json(&v1)).unwrap();
    assert_eq!(run(&d, "{}"), engine(None));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_design_with_a_condition_that_is_not_there_fails() {
    let d = dir("bad");
    let mut bad = design();
    bad.comparisons = vec![Confronto::Pair("ref".into(), "nope".into())];
    std::fs::write(d.join("design.json"), disegno::to_json(&bad)).unwrap();
    let plan = r#"{"format":"mzflow/2","inputs":[{"name":"table.mzfeat","type":"table"},{"name":"samples.json","type":"samples"},{"name":"design.json","type":"design"}],"steps":[
    {"id":"s","step":"table.stats@1","in":{"table":"file:table.mzfeat","samples":"file:samples.json","design":"file:design.json"}}]}"#;
    let (_, prov) = crate::flow::run_plan(plan.as_bytes(), &d, &d.join("work"), 1, None).unwrap();
    assert!(prov.contains("error.stats.condition"), "{prov}");
    let _ = std::fs::remove_dir_all(&d);
}
