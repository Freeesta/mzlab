//! Plan of `spectra.insilico` on a synthetic one-spectrum MGF and a three-row CSV list (no lab data).

use std::path::PathBuf;

fn fixture(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_insilico_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(dir.join("work")).unwrap();
    // an ester of mass 150.0681 ([M+H]+ 151.0754) with the fragments of its acylium and phenyl ions, and a decoy peak
    let mgf = "BEGIN IONS\nTITLE=q0\nPEPMASS=151.0754\nCHARGE=1+\nIONMODE=Positive\n105.0335 1000\n77.0386 400\n123.0441 300\n60.0000 50\nEND IONS\n";
    std::fs::write(dir.join("q.mgf"), mgf).unwrap();
    let csv = "Identifier,CompoundName,SMILES,MolecularFormula,MonoisotopicMass\n\
        A1,Candidate A,CCOC(=O)c1ccccc1,C9H10O2,150.068080\n\
        A2,Candidate B,COC(=O)Cc1ccccc1,C9H10O2,150.068080\n\
        Z9,Far away,CCCCCCCC,C8H18,114.140850\n";
    std::fs::write(dir.join("list.csv"), csv).unwrap();
    dir
}

fn run(dir: &std::path::Path, params: &str) -> mzlab_core::json::Value {
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"q.mgf","type":"spectra"}},{{"name":"list.csv","type":"candidates"}}],"steps":[
        {{"id":"i","step":"spectra.insilico@1","in":{{"spectra":"file:q.mgf","suspects":"file:list.csv"}},"params":{params}}}]}}"#
    );
    let (run, prov) =
        crate::flow::run_plan(plan.as_bytes(), dir, &dir.join("work"), 1, None).unwrap();
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    mzlab_core::json::parse(&std::fs::read(dir.join("work/i.json")).unwrap()).unwrap()
}

/// The candidates inside the mass window are ranked with their explained peaks, the far one is left out, the bytes are the same twice, `top` and `score_terms` obeyed.
#[test]
fn insilico_ranks_the_suspects_inside_the_mass_window() {
    let dir = fixture("rank");
    let h = run(&dir, "{}");
    let hits = h.get("hits").unwrap().arr();
    assert!(!hits.is_empty() && hits.len() <= 2, "{}", hits.len());
    assert!(hits
        .iter()
        .all(|x| x.get("id").unwrap().str() != Some("Z9")));
    let best = &hits[0];
    assert_eq!(best.get("source").unwrap().str(), Some("insilico"));
    assert!(best.get("score").unwrap().f64().unwrap() > 0.0);
    assert!(best.get("shared").unwrap().f64().unwrap() >= 1.0);
    let first = std::fs::read(dir.join("work/i.json")).unwrap();
    run(&dir, "{}");
    assert_eq!(first, std::fs::read(dir.join("work/i.json")).unwrap());
    let top1 = run(&dir, r#"{"top":1,"score_terms":true}"#);
    let one = top1.get("hits").unwrap().arr();
    assert_eq!(one.len(), 1);
    assert!(one[0].get("score_terms").is_some());
    let _ = std::fs::remove_dir_all(&dir);
}
