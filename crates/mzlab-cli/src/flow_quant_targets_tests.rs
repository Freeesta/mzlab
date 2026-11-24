//! `quant.targets@1` through the plan runner on synthetic indexes: the table and the log are the bytes `mzlab-cli quant targets` writes for the
//! same files and parameters, and a second run takes both from the cache unchanged.

use std::path::{Path, PathBuf};

use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare;

#[path = "../../mzlab-feat/tests/common_quant/mod.rs"]
#[allow(dead_code)]
mod common_quant;
use common_quant::{mzml, sample, Ion, TARGETS};

fn index(ions: &[Ion], seed: u64) -> Vec<u8> {
    let x = mzml(ions, seed);
    let mut src = x.clone();
    let p = prepare(&mut src, x.len() as u64, "x.mzML").unwrap();
    build_mzml_to_vec(&p.table, &x, 64 << 20).unwrap().0
}

fn dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_quant_targets_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    std::fs::write(d.join("S_01.mzidx"), index(&sample(), 1)).unwrap();
    std::fs::write(
        d.join("Blank_01.mzidx"),
        index(&[Ion(200.0, 5.0, 3000.0, 0.1)], 2),
    )
    .unwrap();
    std::fs::write(d.join("targets.csv"), TARGETS).unwrap();
    d
}

fn run(d: &Path) -> String {
    let plan = r#"{"format":"mzflow/2","inputs":[{"name":"Blank_01.mzidx","type":"mzidx"},{"name":"S_01.mzidx","type":"mzidx"},{"name":"targets.csv","type":"report"}],"steps":[
        {"id":"q","step":"quant.targets@1","in":{"raw":["file:Blank_01.mzidx","file:S_01.mzidx"],"targets":"file:targets.csv"},"params":{"offset":"auto","blank_pattern":"blank*","min_sn":5}}]}"#;
    let (run, prov) =
        crate::flow::run_plan_cached(plan.as_bytes(), d, &d.join("work"), 1, None, Some(false))
            .unwrap();
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    prov
}

#[test]
fn the_step_writes_the_bytes_of_the_cli_and_is_cached() {
    let d = dir();
    run(&d);
    let table = std::fs::read(d.join("work/q.csv")).unwrap();
    let log = std::fs::read(d.join("work/q.log.json")).unwrap();
    let at = |f: &str| d.join(f).to_string_lossy().into_owned();
    let args: Vec<String> = vec![
        "--targets".into(),
        at("targets.csv"),
        at("Blank_01.mzidx"),
        at("S_01.mzidx"),
        "--offset".into(),
        "auto".into(),
        "--blank-pattern".into(),
        "blank*".into(),
        "--min-sn".into(),
        "5".into(),
        "--log".into(),
        at("cli.log.json"),
    ];
    let cli = crate::quant_targets_cmd::run(args);
    assert_eq!(cli.code, 0, "{}", cli.stderr);
    assert_eq!(
        cli.stdout, table,
        "the CLI and the step write the same table"
    );
    assert_eq!(std::fs::read(d.join("cli.log.json")).unwrap(), log);
    assert!(String::from_utf8_lossy(&table).contains("S_01.mzidx,clean,"));
    let again = run(&d);
    assert!(again.contains("\"cached\":true"), "{again}");
    assert_eq!(std::fs::read(d.join("work/q.csv")).unwrap(), table);
    let _ = std::fs::remove_dir_all(&d);
}
