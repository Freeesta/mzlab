//! `structure.reactivity@1` in a plan: ethanol (a textbook molecule) gives the `reatt/1` of the engine byte for byte, twice the same, and the options of the
//! step (solvent, conformers, seed) reach the engine as the `.wasm` `chem` would read them.

use std::path::Path;

use mzlab_chem::reatt::reactivity_json;

const ETHANOL: &str = "CCO";

fn dir(tag: &str, parent: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_reactivity_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    std::fs::write(d.join("parent.txt"), parent).unwrap();
    d
}

fn run(d: &Path, params: &str) -> Result<String, String> {
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"parent.txt","type":"structure"}}],"steps":[
        {{"id":"reat","step":"structure.reactivity@1","in":{{"parent":"file:parent.txt"}},"params":{params}}}]}}"#
    );
    let (done, prov) = crate::flow::run_plan(plan.as_bytes(), d, &d.join("work"), 1, None)
        .map_err(|e| e.key.to_string())?;
    if !done.skipped.is_empty() || prov.contains("\"error\"") {
        return Err(prov);
    }
    Ok(std::fs::read_to_string(d.join("work/reat.json")).unwrap())
}

#[test]
fn the_plan_gives_the_bytes_of_the_engine() {
    let d = dir("bytes", ETHANOL);
    let got = run(&d, "{}").unwrap();
    assert!(got.contains("\"atoms\""), "{got}");
    assert_eq!(got, reactivity_json(ETHANOL, "").unwrap());
    assert_eq!(got, run(&d, "{}").unwrap());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn every_option_of_the_step_reaches_the_engine() {
    let d = dir("opts", ETHANOL);
    let dry = run(&d, r#"{"solvent":"none"}"#).unwrap();
    assert_eq!(
        dry,
        reactivity_json(ETHANOL, r#"{"solvent":"none"}"#).unwrap()
    );
    assert_ne!(dry, run(&d, "{}").unwrap());
    let one = run(&d, r#"{"conformers":1,"seed":7}"#).unwrap();
    assert_eq!(
        one,
        reactivity_json(ETHANOL, r#"{"conformers":1,"seed":7}"#).unwrap()
    );
    let ph = run(&d, r#"{"protomer":"ph7"}"#).unwrap();
    assert_eq!(
        ph,
        reactivity_json(ETHANOL, r#"{"protomer":"ph7"}"#).unwrap()
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_parent_without_a_structure_is_refused() {
    let d = dir("nosmiles", "C2H6O");
    assert!(run(&d, "{}").is_err());
    let _ = std::fs::remove_dir_all(&d);
}
