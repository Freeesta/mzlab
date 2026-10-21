//! `tp.generate@1` in a plan: the registry of the hypotheses of a parent is the one `mzlab_feat::hypo::generate` gives on the rules of the campaign with the
//! same inputs and parameters, from a formula and from a SMILES (whose groups choose the rules), byte for byte, and a second run gives the same bytes.

use std::path::Path;

use mzlab_feat::chem::grafo::analyse_json;
use mzlab_feat::hypo::{generate, HypoParams, Inputs, Rules, Structure};

const RULES: &[u8] = include_bytes!("../../../TP_Mine/data/regole_tp_2.json");
const FORMULA: &str = "C13H24N4O3S";
const SMILES: &str = "CC(C)(C)NCC(O)COc1nsnc1N1CCOCC1";

fn dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_tpgen_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    d
}

fn run(d: &Path, params: &str) -> Result<String, String> {
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"parent.txt","type":"structure"}}],"steps":[
        {{"id":"g","step":"tp.generate@1","in":{{"parent":"file:parent.txt"}},"params":{params}}}]}}"#
    );
    let (run, prov) = crate::flow::run_plan(plan.as_bytes(), d, &d.join("work"), 1, None)
        .map_err(|e| e.key.to_string())?;
    if !run.skipped.is_empty() || prov.contains("\"error\"") {
        return Err(prov);
    }
    Ok(std::fs::read_to_string(d.join("work/g.json")).unwrap())
}

/// The registry the page asks for: the structure (or none) and the formula of the parent.
fn engine(smiles: Option<&str>, p: &HypoParams) -> String {
    let structure = smiles.map(|s| analyse_json(s).unwrap()).unwrap_or_default();
    let st = Structure::parse(&structure).unwrap();
    let inp = Inputs {
        parent: FORMULA,
        suspects: None,
        families: &[],
        imported: &[],
        fragments: &st.fragments,
        groups: st.groups.as_deref(),
    };
    let rules = Rules::parse_profile(RULES, st.profile.as_deref()).unwrap();
    generate(&rules, &inp, p).unwrap().to_json()
}

#[test]
fn the_plan_gives_the_registry_of_the_engine() {
    let d = dir("eng");
    std::fs::write(d.join("parent.txt"), format!("{FORMULA}\n")).unwrap();
    let got = run(&d, "{}").unwrap();
    assert!(got.starts_with("{\"format\":\"ipotesi/1\""), "{got}");
    assert_eq!(got, engine(None, &HypoParams::default()));
    assert_eq!(got, run(&d, "{}").unwrap());
    // the parameters reach the engine
    let neg = run(&d, r#"{"polarity":-1,"max_steps":1,"min_mz":100}"#).unwrap();
    let p = HypoParams {
        polarity: -1,
        max_steps: 1,
        min_mz: 100.0,
        ..HypoParams::default()
    };
    assert_eq!(neg, engine(None, &p));
    assert_ne!(neg, got);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_smiles_gives_the_groups_and_the_fragments_of_the_page() {
    let d = dir("smi");
    std::fs::write(d.join("parent.txt"), SMILES).unwrap();
    assert_eq!(
        run(&d, "{}").unwrap(),
        engine(Some(SMILES), &HypoParams::default())
    );
    // the same parent as JSON, with the formula written by hand
    std::fs::write(
        d.join("parent.txt"),
        format!(r#"{{"name":"timolol","formula":"{FORMULA}","smiles":"{SMILES}"}}"#),
    )
    .unwrap();
    assert_eq!(
        run(&d, "{}").unwrap(),
        engine(Some(SMILES), &HypoParams::default())
    );
    // a formula that is not the structure is refused with both
    std::fs::write(
        d.join("parent.txt"),
        format!(r#"{{"formula":"C2H6O","smiles":"{SMILES}"}}"#),
    )
    .unwrap();
    assert!(run(&d, "{}").is_err());
    std::fs::write(d.join("parent.txt"), "\n").unwrap();
    assert!(run(&d, "{}").is_err());
    let _ = std::fs::remove_dir_all(&d);
}
