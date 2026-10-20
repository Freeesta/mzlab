//! The defaults of `tp.link`, `tp.position` and `tp.filter` are the constants of `interp` (`ms2link`, `sito`, `filtri`, `modifica`), and those of
//! `tp.generate` the fields of `HypoParams::default()`. A constant that moves in the code fails here until the descriptor follows.

use mzlab_core::step::Step;
use mzlab_feat::hypo::HypoParams;
use mzlab_feat::interp::{filtri, modifica, ms2link, sito};
use mzlab_feat::steps::tabui;

fn step(id: &str) -> Step {
    tabui().into_iter().find(|s| s.id == id).unwrap()
}

fn num(id: &str, name: &str) -> f64 {
    let s = step(id);
    s.params
        .iter()
        .chain(s.algorithms.iter().flat_map(|a| a.params.iter()))
        .find(|p| p.name == name)
        .unwrap_or_else(|| panic!("{id} has no {name}"))
        .default_f64()
        .unwrap_or_else(|| panic!("{id} {name} has no number"))
}

#[test]
fn hypotheses() {
    let d = HypoParams::default();
    assert_eq!(num("tp.generate", "polarity"), f64::from(d.polarity));
    assert_eq!(num("tp.generate", "max_steps"), f64::from(d.max_steps));
    assert_eq!(num("tp.generate", "min_mz"), d.min_mz);
    assert_eq!(num("tp.generate", "max_suspects"), d.max_suspects as f64);
}

#[test]
fn filters() {
    for (name, want) in [
        ("tol_min", filtri::TOL_MIN),
        ("tol_max", filtri::TOL_MAX),
        ("decoy_step", filtri::DECOY_STEP),
        ("decoy_k", f64::from(filtri::DECOY_K)),
        ("decoy_ppm", filtri::DECOY_PPM),
    ] {
        assert_eq!(num("tp.filter", name), want, "{name}");
    }
}

#[test]
fn link() {
    for (name, want) in [
        ("select_ppm", ms2link::SELECT_PPM),
        ("purity_ppm", ms2link::PURITY_PPM),
        ("purity_min", ms2link::PURITY_MIN),
        ("match_ppm", modifica::MATCH_PPM),
        ("iso_corr", ms2link::ISO_CORR),
        ("iso_min_scans", ms2link::ISO_MIN_SCANS as f64),
        ("iso_fragments", ms2link::ISO_FRAGMENTS as f64),
    ] {
        assert_eq!(num("tp.link", name), want, "{name}");
    }
}

#[test]
fn position() {
    for (name, want) in [
        ("match_ppm", sito::MATCH_PPM),
        ("min_rel", sito::MIN_REL),
        ("min_informative", sito::MIN_INFORMATIVE as f64),
        ("margin", sito::MARGIN),
        ("ll_support", sito::LL_SUPPORT),
        ("ll_against", sito::LL_AGAINST),
    ] {
        assert_eq!(num("tp.position", name), want, "{name}");
    }
}
