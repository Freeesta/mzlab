//! The `mass_network` of the Dig on a hand-made Dig of invented products of the timolol formula, against the answer saved before `rete/arcs.rs` took
//! the shared functions out of `legacy/network.rs` (`tests/data/mass_network_timolol.json`): the bytes must not change. `MZLAB_WRITE_GOLDEN=1` rewrites it.

use mzlab_core::elements::parse_formula;
use mzlab_feat::hypo::Rules;
use mzlab_feat::legacy::dig::{Cand, CandMs2, Dig};
use mzlab_feat::legacy::kinetics::Descriptors;
use mzlab_feat::legacy::ms2::Consensus;
use mzlab_feat::legacy::network::mass_network;
use mzlab_feat::legacy::series::{Hyp, Ident, Parent};
use mzlab_feat::legacy::similarity::Components;
use mzlab_feat::rtmap::RtMap;

const PROTON: f64 = 1.007276;
const GOLDEN: &str = "tests/data/mass_network_timolol.json";

fn formula(s: &str) -> Vec<(String, u64)> {
    parse_formula(s).unwrap()
}

fn kin(tmax: f64) -> (Descriptors, bool) {
    let d = Descriptors {
        onset: 0.0,
        tmax,
        residual_fraction: 0.0,
        unimodal_residual: 0.0,
        class: vec![],
        profile: vec![],
        times: vec![],
    };
    (d, true)
}

fn ms2(peaks: &[f64], shifted: usize, cos: f64) -> CandMs2 {
    CandMs2 {
        cons: Consensus {
            mz: peaks.to_vec(),
            rel: vec![100.0, 80.0, 60.0, 40.0, 30.0][..peaks.len()].to_vec(),
            n_found: vec![3; peaks.len()],
            n_scans: 3,
        },
        sim: Components {
            delta: 15.9949,
            cos: [0.2, cos, 0.0],
            ent: [0.0; 3],
            n: [2, shifted, 0],
            pairs: vec![],
        },
    }
}

fn cand(row: usize, neutral: &str, rt: f64, tmax: f64, ms2: Option<CandMs2>) -> Cand {
    let mz = mzlab_core::elements::mass(&formula(neutral)) + PROTON;
    Cand {
        row,
        mz,
        rt,
        areas: vec![],
        area_max: 1e6,
        formula: Some(neutral.into()),
        ppm: Some(0.5),
        delta: None,
        hyp: Some(Hyp {
            label: format!("P{row}"),
            formula: neutral.into(),
            mz,
            level: 4,
            sources: vec![],
            alternatives: vec![],
        }),
        kin: Some(kin(tmax)),
        ms2_scans: 3,
        ms2,
        score: 50.0 + row as f64,
        comps: [0.0; 10],
        ident: Ident {
            row,
            hypothesis: None,
            formula: Some(neutral.into()),
            total: 1.0,
            proofs: vec![],
        },
        flags: vec![],
        filled: vec![],
        limits: vec![],
        hidden: None,
    }
}

fn dig() -> Dig {
    let neutral = formula("C13H24N4O3S");
    let mut ion = neutral.clone();
    ion.iter_mut().find(|(e, _)| e == "H").unwrap().1 += 1;
    let peaks = [100.0, 116.0, 130.0, 146.0, 160.0];
    Dig {
        files: vec![],
        parent: Parent {
            name: "timolol".into(),
            neutral,
            ion,
            mz: 317.1642,
            adduct: "[M+H]+".into(),
            rt: Some(5.0),
            smiles: None,
        },
        parent_row: Some(0),
        parent_rt: Some(5.0),
        parent_ms2: None,
        rt_map: RtMap::default(),
        isf_hidden: vec![],
        n_features: vec![],
        cands: vec![
            cand(1, "C13H24N4O4S", 4.5, 10.0, Some(ms2(&peaks, 2, 0.5))),
            cand(2, "C13H24N4O5S", 4.0, 40.0, Some(ms2(&peaks, 3, 0.6))),
            cand(3, "C13H22N4O3S", 5.5, 20.0, None),
            cand(4, "C10H18N4O3S", 3.0, 30.0, Some(ms2(&peaks, 1, 0.3))),
            // no mass difference to anything, but an MS2 like that of the first product
            cand(5, "C9H8N2O2", 6.0, 30.0, Some(ms2(&peaks, 0, 0.0))),
            cand(6, "C13H24N4O3S", 5.0, 0.0, None),
        ],
        weights: [0.0; 10],
        funnel: vec![],
        warnings: vec![],
    }
}

#[test]
fn mass_network_does_not_change() {
    let rules = Rules::parse(include_bytes!("../../../TP_Mine/data/regole_tp.json")).unwrap();
    let got = mass_network(&dig(), &rules);
    assert!(got.contains("\"edges\""), "{got}");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(GOLDEN);
    if std::env::var("MZLAB_WRITE_GOLDEN").is_ok() {
        std::fs::write(&path, &got).unwrap();
    }
    assert_eq!(got, std::fs::read_to_string(&path).unwrap());
}
