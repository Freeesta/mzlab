//! Plans of `spectra.match` beyond the first algorithms (unit-resolution window, decoys on an `.mzlib`, name filter) on the synthetic fixtures of `flow_ident_tests`.

use crate::flow_ident_tests::{fixture, hits, mix_fixture, plan, run};

/// Unit-resolution query: the precursor is 0.3 Da off the library one, so the ppm window finds nothing and `precursor_da` finds Alpha, with the error in ppm.
#[test]
fn precursor_da_matches_a_unit_resolution_query() {
    let dir = fixture("lr");
    let mgf = std::fs::read_to_string(dir.join("q.mgf")).unwrap();
    std::fs::write(
        dir.join("q.mgf"),
        mgf.replace("PEPMASS=300.1235", "PEPMASS=300.4"),
    )
    .unwrap();
    for alg in ["spectral_entropy", "cosine"] {
        run(&dir, alg, r#"{"tol_da":0.5}"#);
        assert!(!hits(&dir).get("hits").unwrap().arr().iter().any(|x| x
            .get("query")
            .unwrap()
            .str()
            == Some("q0")));
        run(&dir, alg, r#"{"tol_da":0.5,"precursor_da":0.5}"#);
        let h = hits(&dir);
        let best = h
            .get("hits")
            .unwrap()
            .arr()
            .iter()
            .find(|x| x.get("query").unwrap().str() == Some("q0"))
            .unwrap();
        assert_eq!(best.get("name").unwrap().str(), Some("Alpha"), "{alg}");
        let ppm = best.get("mass_ppm").unwrap().f64().unwrap();
        assert!(
            (ppm - (300.1234 - 300.4) / 300.4 * 1e6).abs() < 1.0,
            "{alg} {ppm}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A `.mzlib` library takes decoys too: it is read back to spectra, so the q-values exist and no decoy is listed.
#[test]
fn decoys_work_on_an_mzlib_library() {
    use mzlab_lib::build::{build_in_memory, BuildOpts};
    let dir = mix_fixture("mzlib");
    let msp = std::fs::read(dir.join("lib.msp")).unwrap();
    let targets = crate::flow_match::read_spectra(("lib.msp", &msp)).unwrap();
    std::fs::write(
        dir.join("lib.mzlib"),
        build_in_memory(&targets, BuildOpts::default()).unwrap().0,
    )
    .unwrap();
    let on = |lib: &str| {
        let p = plan("spectral_entropy", r#"{"decoys":"spectrum"}"#).replace("lib.msp", lib);
        crate::flow::run_plan(p.as_bytes(), &dir, &dir.join("work"), 1, None).unwrap();
        std::fs::read_to_string(dir.join("work/m.json")).unwrap()
    };
    let (text, packed) = (on("lib.msp"), on("lib.mzlib"));
    assert!(
        packed.contains("\"fdr_note\"") && packed.contains("\"decoy_q\":0"),
        "{}",
        &packed[..300.min(packed.len())]
    );
    assert!(!packed.contains("DECOY:"));
    // same library, same decoys: the same candidates (scores may differ in the last digits by the quantisation of the file)
    assert_eq!(
        text.matches("\"query\":").count(),
        packed.matches("\"query\":").count()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// `name_contains` keeps only the library entries whose name has the text, in any case.
#[test]
fn name_contains_filters_the_candidates() {
    let dir = fixture("name");
    run(
        &dir,
        "cosine",
        r#"{"precursor_da":150,"tol_da":0.5,"top_n":10}"#,
    );
    let names = |h: &mzlab_core::json::Value| -> Vec<String> {
        h.get("hits")
            .unwrap()
            .arr()
            .iter()
            .map(|x| x.get("name").unwrap().str().unwrap().to_string())
            .collect()
    };
    assert!(names(&hits(&dir)).len() > 1);
    run(
        &dir,
        "cosine",
        r#"{"precursor_da":150,"tol_da":0.5,"top_n":10,"name_contains":"ALPH"}"#,
    );
    let only = names(&hits(&dir));
    assert!(
        !only.is_empty() && only.iter().all(|n| n == "Alpha"),
        "{only:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// `adduct_contains`, `instrument_contains` and `ce_contains` keep only the library entries whose field has the text, in any case; they combine with each other.
#[test]
fn metadata_filters_keep_the_matching_entries() {
    let dir = fixture("meta");
    let msp = std::fs::read_to_string(dir.join("lib.msp")).unwrap();
    // Alpha gets an instrument and a collision energy, Beta another adduct, instrument and energy
    let (alpha, rest) = msp.split_once("Name: Beta\n").unwrap();
    let alpha = alpha.replace(
        "Name: Alpha\n",
        "Name: Alpha\nInstrument_type: Orbitrap\nCollision_energy: 30 HCD\n",
    );
    let rest = rest.replacen(
        "Precursor_type: [M+H]+",
        "Precursor_type: [M+Na]+\nInstrument_type: QTOF\nCollision_energy: 20",
        1,
    );
    let msp = format!("{alpha}Name: Beta\n{rest}");
    std::fs::write(dir.join("lib.msp"), msp).unwrap();
    let names = |extra: &str| -> Vec<String> {
        let p = format!(r#"{{"precursor_da":150,"tol_da":0.5,"top_n":10{extra}}}"#);
        run(&dir, "cosine", &p);
        let mut n: Vec<String> = hits(&dir)
            .get("hits")
            .unwrap()
            .arr()
            .iter()
            .map(|x| x.get("name").unwrap().str().unwrap().to_string())
            .collect();
        n.sort();
        n.dedup();
        n
    };
    assert!(names("").len() >= 3);
    assert_eq!(names(r#","adduct_contains":"[m+NA]""#), ["Beta"]);
    assert_eq!(names(r#","instrument_contains":"orbi""#), ["Alpha"]);
    assert_eq!(names(r#","ce_contains":"HCD""#), ["Alpha"]);
    assert!(names(r#","instrument_contains":"orbi","adduct_contains":"na""#).is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

/// `rt_tol` drops the library entries whose retention time is farther than the tolerance from the query's; entries without one stay, and `rt_delta_min` is written when both are known.
#[test]
fn rt_tol_excludes_entries_by_retention_time() {
    let dir = fixture("rt");
    let msp = std::fs::read_to_string(dir.join("lib.msp")).unwrap();
    let msp = msp
        .replace("Name: Alpha\n", "Name: Alpha\nRT: 5.0\n")
        .replace("Name: Beta\n", "Name: Beta\nRT: 9.0\n");
    std::fs::write(dir.join("lib.msp"), msp).unwrap();
    let mgf = std::fs::read_to_string(dir.join("q.mgf")).unwrap();
    std::fs::write(
        dir.join("q.mgf"),
        mgf.replacen("BEGIN IONS\n", "BEGIN IONS\nRTINSECONDS=306\n", 1),
    )
    .unwrap();
    let run_rt = |extra: &str| {
        run(
            &dir,
            "cosine",
            &format!(r#"{{"precursor_da":150,"tol_da":0.5,"top_n":10{extra}}}"#),
        );
        let h = hits(&dir);
        let mut v: Vec<(String, Option<f64>)> = h
            .get("hits")
            .unwrap()
            .arr()
            .iter()
            .filter(|x| x.get("query").unwrap().str() == Some("q0"))
            .map(|x| {
                (
                    x.get("name").unwrap().str().unwrap().to_string(),
                    x.get("rt_delta_min").and_then(|d| d.f64()),
                )
            })
            .collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        v
    };
    let off = run_rt("");
    assert_eq!(off.len(), 3);
    assert!(off
        .iter()
        .any(|(n, d)| n == "Beta" && d.is_some_and(|d| (d + 3.9).abs() < 1e-9)));
    let on = run_rt(r#","rt_tol":0.5"#);
    let names: Vec<&str> = on.iter().map(|(n, _)| n.as_str()).collect();
    // Alpha is 0.1 min away, Beta 3.9 min away, Gamma has no retention time
    assert_eq!(names, ["Alpha", "Gamma"], "{on:?}");
    assert!(
        (on[0].1.unwrap() - 0.1).abs() < 1e-9 && on[1].1.is_none(),
        "{on:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// `tree`: a graph `msngrafo/1` against an `msnlib/1` library gives the `msncerca/1` text of `mzlab-cli msn-lib search`, byte for byte.
#[test]
fn tree_writes_the_search_of_the_msn_library() {
    let dir = std::env::temp_dir().join(format!("mzlab_match_tree_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mzml = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/ref/msntree_rotta/msn.mzML");
    let cli = |a: &[&str]| crate::execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    let (lib, graph) = (dir.join("lib.json"), dir.join("g.json"));
    let (l, f) = (lib.to_str().unwrap(), mzml.to_str().unwrap());
    let ion = ["--formula", "C14H15N3O5+"];
    let add = ["msn-lib", "add", l, f, "--id", "STD-1"];
    assert_eq!(cli(&[&add[..], &ion[..]].concat()).code, 0);
    let g = cli(&[&["msn-graph", f][..], &ion[..]].concat());
    std::fs::write(&graph, &g.stdout).unwrap();
    std::fs::create_dir_all(dir.join("work")).unwrap();
    let direct = cli(&["msn-lib", "search", l, graph.to_str().unwrap()]);
    let p = r#"{"format":"mzflow/2","inputs":[{"name":"g.json","type":"report"},{"name":"lib.json","type":"library"}],"steps":[
        {"id":"m","step":"spectra.match@1","in":{"query":"file:g.json","library":"file:lib.json"},"algorithm":"tree","params":{"precursor_ppm":10,"tol_da":0.01,"top_n":10}}]}"#;
    let (run, prov) =
        crate::flow::run_plan(p.as_bytes(), &dir, &dir.join("work"), 1, None).unwrap();
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    let made = std::fs::read(dir.join("work/m.json")).unwrap();
    assert_eq!(made, direct.stdout);
    assert!(String::from_utf8(made)
        .unwrap()
        .contains("\"id\":\"STD-1\""));
    let _ = std::fs::remove_dir_all(&dir);
}

/// `tree_output=candidates` writes the `candidates/1` of the other algorithms; `name_contains` and `precursor_da` apply to the tree too.
#[test]
fn tree_candidates_carry_the_scores_apart_and_obey_the_filters() {
    let dir = std::env::temp_dir().join(format!("mzlab_match_tree_c_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("work")).unwrap();
    let mzml = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/ref/msntree_rotta/msn.mzML");
    let cli = |a: &[&str]| crate::execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    let (lib, graph) = (dir.join("lib.json"), dir.join("g.json"));
    let (l, f) = (lib.to_str().unwrap(), mzml.to_str().unwrap());
    let ion = ["--formula", "C14H15N3O5+"];
    let add = ["msn-lib", "add", l, f, "--id", "STD-1"];
    assert_eq!(cli(&[&add[..], &ion[..]].concat()).code, 0);
    std::fs::write(
        &graph,
        &cli(&[&["msn-graph", f][..], &ion[..]].concat()).stdout,
    )
    .unwrap();
    let run = |params: &str| {
        let p = format!(
            r#"{{"format":"mzflow/2","inputs":[{{"name":"g.json","type":"report"}},{{"name":"lib.json","type":"library"}}],"steps":[
            {{"id":"m","step":"spectra.match@1","in":{{"query":"file:g.json","library":"file:lib.json"}},"algorithm":"tree","params":{params}}}]}}"#
        );
        let (_, prov) =
            crate::flow::run_plan(p.as_bytes(), &dir, &dir.join("work"), 1, None).unwrap();
        assert!(!prov.contains("\"error\""), "{prov}");
        std::fs::read_to_string(dir.join("work/m.json")).unwrap()
    };
    let out = run(r#"{"tree_output":"candidates","tol_da":0.01}"#);
    assert!(out.starts_with("{\"format\":\"candidates/1\""), "{out}");
    assert!(
        out.contains("\"id\":\"STD-1\"") && out.contains("\"tree\":{\"ion\""),
        "{out}"
    );
    assert!(out.contains("\"algorithm\":\"tree\""), "{out}");
    // a name that no entry has leaves no hit
    assert!(run(r#"{"tree_output":"candidates","name_contains":"zzz"}"#).contains("\"hits\":[]"));
    assert!(run(r#"{"tree_output":"candidates","name_contains":"std-1"}"#).contains("STD-1"));
    // the entry is the unknown itself, so a window in Da cannot drop it: the window echoed by msncerca/1 shows the conversion
    let narrow = run(r#"{"precursor_da":0.5}"#);
    assert!(
        narrow.contains("STD-1") && !narrow.contains("\"ppm_prec\":10,"),
        "{narrow}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
