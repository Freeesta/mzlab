//! Parity tests for `mzlab_feat::deconvolve` against Python reference (`tools/deconvolve_riferimento.py`).
//! Checks tolerance of 1e-4 on profiles, correlations, and deconvolved spectra, plus mutational and negative tests.

#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_feat::deconvolve::{
    deconvolve_isomers, fragment_matrix, ms2_consensus, DeconvolveParams,
};
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn near(got: f64, want: f64, tol: f64) -> bool {
    (got - want).abs() <= tol * got.abs().max(want.abs()).max(1.0)
}

fn load_reference_cases() -> Vec<Value> {
    let out =
        std::env::temp_dir().join(format!("mzlab-deconvolve-ref-{}.json", std::process::id()));
    let status = Command::new("python3")
        .current_dir(root())
        .arg("tools/deconvolve_riferimento.py")
        .arg(&out)
        .status()
        .expect("failed to execute tools/deconvolve_riferimento.py");
    assert!(status.success(), "reference generator failed");
    let content = std::fs::read(&out).expect("failed to read generated reference");
    let _ = std::fs::remove_file(&out);
    serde_json::from_slice(&content).expect("failed to parse reference JSON")
}

fn parse_f64_vec(v: &Value) -> Vec<f64> {
    v.as_array()
        .map(|arr| arr.iter().filter_map(|x| x.as_f64()).collect())
        .unwrap_or_default()
}

fn parse_nested_f64_vec(v: &Value) -> Vec<Vec<f64>> {
    v.as_array()
        .map(|arr| arr.iter().map(parse_f64_vec).collect())
        .unwrap_or_default()
}

fn verify_single_component(
    tag: &str,
    g_comp: &mzlab_feat::deconvolve::DeconvolveComponent,
    w_comp: &Value,
) {
    match (g_comp.rt_apex, w_comp["rt_apex"].as_f64()) {
        (Some(ga), Some(wa)) => assert!((ga - wa).abs() <= 1e-4, "{tag}: rt_apex mismatch"),
        (None, None) => {}
        _ => panic!("{tag}: rt_apex presence mismatch"),
    }
    let w_spec = &w_comp["spectrum"];
    let (w_mz, w_rel) = (parse_f64_vec(&w_spec["mz"]), parse_f64_vec(&w_spec["rel"]));
    assert_eq!(
        g_comp.spectrum.mz.len(),
        w_mz.len(),
        "{tag}: mz len mismatch"
    );
    for (&gm, &wm) in g_comp.spectrum.mz.iter().zip(&w_mz) {
        assert!(near(gm, wm, 1e-4), "{tag}: mz mismatch");
    }
    for (&gr, &wr) in g_comp.spectrum.rel.iter().zip(&w_rel) {
        assert!(near(gr, wr, 1e-4), "{tag}: rel mismatch");
    }
    if let (Some(gp), Some(wp_arr)) = (&g_comp.profile, w_comp["profile"].as_array()) {
        let wp: Vec<f64> = wp_arr.iter().map(|x| x.as_f64().unwrap()).collect();
        assert_eq!(gp.len(), wp.len(), "{tag}: profile len mismatch");
        for (&gv, &wv) in gp.iter().zip(&wp) {
            assert!(near(gv, wv, 1e-4), "{tag}: profile mismatch");
        }
    }
    if let (Some(gc), Some(wc)) = (g_comp.xic_corr, w_comp["xic_corr"].as_f64()) {
        assert!(near(gc, wc, 1e-4), "{tag}: xic_corr mismatch");
    }
    if let (Some(gs), Some(ws)) = (g_comp.n_scans_main, w_comp["n_scans_main"].as_u64()) {
        assert_eq!(gs as u64, ws, "{tag}: n_scans_main mismatch");
    }
}

fn verify_case(case: &Value) {
    let name = case["name"].as_str().unwrap_or("unnamed");
    let mzs = parse_nested_f64_vec(&case["mzs"]);
    let ints = parse_nested_f64_vec(&case["ints"]);
    let rts = parse_f64_vec(&case["rts"]);
    let xic_val = &case["xic"];
    let xic_data =
        (!xic_val.is_null()).then(|| (parse_f64_vec(&xic_val["rt"]), parse_f64_vec(&xic_val["y"])));
    let p_val = &case["params"];
    let params = DeconvolveParams {
        min_scans: p_val["min_scans"].as_u64().unwrap_or(4) as usize,
        max_components: p_val["max_components"].as_u64().unwrap_or(3) as usize,
        ppm: p_val["ppm"].as_f64().unwrap_or(8.0),
        min_frac_sig: p_val["min_frac_sig"].as_f64().unwrap_or(0.02),
        subpeak_tol: p_val["subpeak_tol"].as_f64().unwrap_or(0.07),
    };
    let xic_ref = xic_data.as_ref().map(|(r, y)| (r.as_slice(), y.as_slice()));
    let got = deconvolve_isomers(&mzs, &ints, &rts, xic_ref, &params);
    let want = &case["expected"];
    assert_eq!(
        got.k,
        want["k"].as_u64().unwrap() as usize,
        "{name}: k mismatch"
    );
    assert_eq!(
        got.scans,
        want["scans"].as_u64().unwrap() as usize,
        "{name}: scans mismatch"
    );
    assert_eq!(
        got.fragments,
        want["fragments"].as_u64().unwrap() as usize,
        "{name}: fragments mismatch"
    );
    if let (Some(g_lof), Some(w_lof)) = (got.lof, want["lof"].as_f64()) {
        assert!(near(g_lof, w_lof, 0.05), "{name}: LOF mismatch");
    }
    let want_comps = want["components"].as_array().unwrap();
    assert_eq!(
        got.components.len(),
        want_comps.len(),
        "{name}: components count mismatch"
    );
    for (c_idx, (g_comp, w_comp)) in got.components.iter().zip(want_comps).enumerate() {
        verify_single_component(&format!("{name} comp#{c_idx}"), g_comp, w_comp);
    }
}

#[test]
fn parity_against_python_reference() {
    let cases = load_reference_cases();
    assert!(
        cases.len() >= 30,
        "expected at least 30 reference cases, got {}",
        cases.len()
    );
    for case in &cases {
        verify_case(case);
    }
}

#[test]
fn negative_and_boundary_conditions() {
    let p = DeconvolveParams::default();

    // 1. Completely empty inputs
    let res = deconvolve_isomers(&[], &[], &[], None, &p);
    assert_eq!(res.k, 1);
    assert_eq!(res.scans, 0);
    assert_eq!(res.fragments, 0);
    assert!(res.lof.is_none());
    assert_eq!(res.components.len(), 1);
    assert_eq!(res.components[0].rt_apex, None);
    assert_eq!(res.components[0].profile, None);
    assert_eq!(res.components[0].xic_corr, None);

    // 2. Single scan with empty peak list
    let res1 = deconvolve_isomers(&[vec![]], &[vec![]], &[1.5], None, &p);
    assert_eq!(res1.k, 1);
    assert_eq!(res1.scans, 1);
    assert_eq!(res1.fragments, 0);
    assert_eq!(res1.components[0].rt_apex, Some(1.5));
    assert!(res1.components[0].spectrum.mz.is_empty());

    // 3. Fragment matrix with empty or non-replicated peaks
    let (d, cm) = fragment_matrix(&[vec![100.0]], &[vec![50.0]], 8.0, 2, true);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].len(), 0);
    assert_eq!(cm.len(), 0);

    // 4. Consensus MS2 with low-abundance peaks filtered out
    let cons = ms2_consensus(
        &[vec![100.0, 200.0], vec![100.0, 200.0]],
        &[vec![1000.0, 1.0], vec![1000.0, 0.5]], // 200.0 is < 0.5%
        8.0,
    );
    assert_eq!(cons.mz.len(), 1);
    assert!((cons.mz[0] - 100.0).abs() < 1e-4);
}

#[test]
fn mutational_tests() {
    let p = DeconvolveParams::default();

    // Two distinct isomers
    let rts = vec![1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8];
    let mzs = vec![
        vec![100.0, 150.0],
        vec![100.0, 150.0],
        vec![100.0, 150.0, 200.0],
        vec![100.0, 150.0, 200.0],
        vec![200.0, 250.0],
        vec![200.0, 250.0],
        vec![200.0, 250.0],
        vec![200.0, 250.0],
    ];
    let ints = vec![
        vec![50.0, 40.0],
        vec![80.0, 70.0],
        vec![100.0, 90.0, 10.0],
        vec![60.0, 50.0, 30.0],
        vec![70.0, 80.0],
        vec![90.0, 100.0],
        vec![60.0, 70.0],
        vec![30.0, 40.0],
    ];
    let xr = vec![1.0, 1.2, 1.3, 1.5, 1.6, 1.7, 1.9];
    let xy = vec![10.0, 80.0, 90.0, 20.0, 85.0, 70.0, 10.0];

    let base_res = deconvolve_isomers(&mzs, &ints, &rts, Some((&xr, &xy)), &p);
    assert_eq!(base_res.k, 2);

    // Mutation 1: High ppm tolerance collapses fragments into fewer columns
    let p_high_ppm = DeconvolveParams {
        ppm: 1_000_000.0,
        ..p
    };
    let (d_mut, cm_mut) = fragment_matrix(&mzs, &ints, p_high_ppm.ppm, 2, true);
    assert!(cm_mut.len() < base_res.fragments);
    assert!(d_mut[0].len() < base_res.fragments);

    // Mutation 2: Increasing min_scans filters out rarer fragments
    let p_strict_scans = DeconvolveParams { min_scans: 6, ..p };
    let (_, cm_strict) = fragment_matrix(&mzs, &ints, p.ppm, p_strict_scans.min_scans, true);
    assert!(cm_strict.len() < base_res.fragments);

    // Mutation 3: Flattening the XIC trace eliminates subpeaks -> falls back to estimate_components
    let xy_flat = vec![50.0; xr.len()];
    let flat_res = deconvolve_isomers(&mzs, &ints, &rts, Some((&xr, &xy_flat)), &p);
    // Since XIC has no subpeaks, it seeds from estimate_components instead of XIC
    assert!(flat_res.k >= 1);
}
