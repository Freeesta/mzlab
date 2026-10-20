//! The defaults of the identification descriptors are the defaults of the engine: `ReviewSettings` and `AnnotationSettings` (the keys of
//! `campaign.toml`), `InSilicoOpts`, `SearchOpts` and `formula_id::Opts`. A default that moves in the code fails here until the descriptor follows.

use mzlab_core::step::{Param, Step};
use mzlab_feat::attributio::annotate::AnnotationSettings;
use mzlab_feat::attributio::ms2_engine::insilico::InSilicoOpts;
use mzlab_feat::attributio::ms2_engine::matcher::SearchOpts;
use mzlab_feat::attributio::ms2_engine::rearrange::{PENALTY, SHIFTS};
use mzlab_feat::attributio::review::settings::ReviewSettings;
use mzlab_feat::steps::attributio;
use mzlab_feat::untargeted::formula_id::Opts;

fn step(id: &str) -> Step {
    attributio().into_iter().find(|s| s.id == id).unwrap()
}

fn param(s: &Step, name: &str) -> Param {
    s.params
        .iter()
        .chain(s.algorithms.iter().flat_map(|a| a.params.iter()))
        .find(|p| p.name == name)
        .unwrap_or_else(|| panic!("{} has no {name}", s.id))
        .clone()
}

fn num(id: &str, name: &str) -> f64 {
    param(&step(id), name)
        .default_f64()
        .unwrap_or_else(|| panic!("{id} {name} has no number"))
}

#[test]
fn review_settings() {
    let r = ReviewSettings::default();
    for (id, name, want) in [
        ("spectra.chimeric", "chim_rel", r.chim_rel),
        ("spectra.chimeric", "chim_cap", r.chim_cap),
        ("spectra.match", "tol_da", r.tol_da),
        ("spectra.match", "precursor_ppm", r.precursor_ppm),
        ("spectra.match", "top_n", r.top_n as f64),
        ("spectra.match", "analog_window_da", r.analog_window_da),
        (
            "spectra.match",
            "analog_prefilter",
            r.analog_prefilter as f64,
        ),
        (
            "spectra.match",
            "analog_min_shift_da",
            r.analog_min_shift_da,
        ),
        ("spectra.match", "exact_ppm", r.exact_ppm),
        ("spectra.match", "delta_formula_mda", r.delta_formula_mda),
        ("formula.ms2", "ppm_ion", r.ppm_ion),
        ("formula.ms2", "ppm_frag", r.ppm_frag),
        ("formula.ms2", "max_subformulas", r.max_subformulas as f64),
        ("rt.predict", "rt_logp_k", r.rt_logp_k),
        ("rt.predict", "rt_logp_floor", r.rt_logp_floor),
        ("rt.predict", "lc_dwell_min", r.lc_dwell_min),
    ] {
        assert_eq!(num(id, name), want, "{id} {name}");
    }
}

#[test]
fn annotation_settings() {
    let a = AnnotationSettings::default();
    for (name, want) in [
        ("spec_sim_min", a.spec_sim_min),
        ("spec_peaks_min", a.spec_peaks_min),
        ("library_conflict_margin", a.library_conflict_margin),
        ("gnps_mass_tol_ppm", a.gnps_mass_tol_ppm),
        ("mzlib_mass_tol_ppm", a.mzlib_mass_tol_ppm),
        ("cosmic_min", a.cosmic_min),
        ("canopus_min", a.canopus_min),
        ("rt_group_tol_min", a.rt_group_tol_min),
    ] {
        assert_eq!(num("annot.level", name), want, "annot.level {name}");
    }
    let s = step("annot.level");
    assert_eq!(
        param(&s, "library_conflict").default.unwrap().str(),
        Some(a.library_conflict.as_str())
    );
    assert_eq!(
        param(&s, "putative_library_level3").default.unwrap().bool(),
        Some(a.putative_library_level3)
    );
}

#[test]
fn engine_options() {
    let i = InSilicoOpts::default();
    assert_eq!(num("spectra.insilico", "db_ppm"), i.db_ppm);
    assert_eq!(num("spectra.insilico", "frag_ppm"), i.frag_ppm);
    assert_eq!(num("spectra.insilico", "max_bonds"), i.max_bonds as f64);
    let m = SearchOpts::default();
    assert_eq!(num("spectra.match", "min_score"), m.min_score);
    assert_eq!(num("spectra.match", "min_shared"), m.min_shared as f64);
    let f = Opts::default();
    let s = step("formula.ms1");
    assert_eq!(
        param(&s, "ion").default.unwrap().str(),
        Some(f.ion.as_str())
    );
    assert_eq!(
        param(&s, "nitrogen").default.unwrap().str(),
        Some(f.nitrogen.as_str())
    );
    assert_eq!(num("formula.ms1", "tol_ppm"), f.tol_ppm);
    assert_eq!(num("formula.ms1", "rdb_min"), f.rdb_min);
    assert_eq!(num("formula.ms1", "max_results"), f.max_results as f64);
    assert_eq!(num("formula.ms1", "max_charge"), f.finder.max_charge as f64);
    assert_eq!(
        param(&s, "require_c13").default.unwrap().bool(),
        Some(f.finder.require_c13)
    );
    assert!(
        f.z.is_none() && f.elements.is_none(),
        "z and elements are estimated"
    );
}

#[test]
fn constants_named_in_the_text() {
    assert_eq!(SHIFTS, [0, 1, -1, 2, -2]);
    assert_eq!(PENALTY, 0.25);
}
