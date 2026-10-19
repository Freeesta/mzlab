//! Chemical classes from rules (`classi/1`): every rule recomputed against literature values, synthetic spectra (positive, negative, ambiguous) and noise.

use mzlab_core::json;
use mzlab_feat::classi::{evaluate, to_json, Kind, Polarity, Role, Rule, Table, MIN_REL};

const PRECURSOR: f64 = 450.1234;
const PPM: f64 = 5.0;

/// (rule, m/z or Δm from the literature, 4 decimals).
const LITERATURE: &[(&str, f64)] = &[
    ("ph_149", 149.0233),
    ("gl_loss", 176.0321),
    ("gl_175", 175.0248),
    ("gl_113", 113.0244),
    ("gl_85", 85.0295),
    ("su_loss", 79.9568),
    ("su_hso4", 96.9601),
    ("su_so3", 79.9574),
    ("sa_so3", 79.9574),
    ("sa_so2", 63.9619),
    ("hx_loss", 162.0528),
    ("dh_loss", 146.0579),
    ("pe_loss", 132.0423),
    ("hn_204", 204.0867),
    ("hn_loss", 203.0794),
    ("hn_168", 168.0655),
    ("hn_138", 138.0550),
    ("ph_h2po4", 96.9696),
    ("ph_po3", 78.9591),
    ("ph_h3po4", 97.9769),
    ("ph_hpo3", 79.9663),
    ("pc_184", 184.0733),
    ("pc_86", 86.0964),
    ("sf_156", 156.0114),
    ("sf_108", 108.0444),
    ("sf_92", 92.0495),
    ("sf_so2", 63.9619),
    ("pf_cf3", 68.9958),
    ("pf_c2f5", 118.9926),
    ("pf_c3f7", 168.9894),
    ("pf_fso3", 98.9558),
    ("pf_cf2", 49.9968),
    ("pf_hf", 20.0062),
    ("pf_co2", 43.9898),
    ("gs_129", 129.0426),
    ("gs_272", 272.0888),
    ("gs_75", 75.0320),
    ("gs_307", 307.0838),
    ("ta_124", 124.0074),
    ("ta_80", 79.9574),
];

fn table() -> Table {
    Table::bundled().unwrap()
}

fn pol_of(r: &Rule, other: Polarity) -> Polarity {
    r.polarity.unwrap_or(other)
}

/// Peak that a rule should match, exactly on the theoretical value.
fn peak_for(r: &Rule, offset_tol: f64) -> f64 {
    match r.kind {
        Kind::Frag => r.value * (1.0 + offset_tol * PPM * 1e-6),
        Kind::Loss => PRECURSOR - r.value + offset_tol * PRECURSOR * PPM * 1e-6,
    }
}

fn run(peaks: &[(f64, f64)], pol: Polarity, ppm: f64) -> Vec<(String, &'static str, Vec<String>)> {
    let r = evaluate(&table(), peaks, PRECURSOR, pol, ppm).unwrap();
    r.classes
        .into_iter()
        .map(|c| (c.id, c.level, c.ambiguous_with))
        .collect()
}

fn ids(v: &[(String, &'static str, Vec<String>)]) -> Vec<String> {
    v.iter().map(|c| c.0.clone()).collect()
}

#[test]
fn every_rule_matches_the_literature_value() {
    let t = table();
    for r in &t.rules {
        assert!(
            LITERATURE.iter().any(|l| l.0 == r.id),
            "rule {} has no literature row",
            r.id
        );
    }
    for (id, lit) in LITERATURE {
        let r = t
            .rules
            .iter()
            .find(|r| r.id == *id)
            .unwrap_or_else(|| panic!("no rule {id}"));
        assert!(
            (r.value - lit).abs() < 0.00015,
            "{id}: computed {} vs literature {lit}",
            r.value
        );
    }
    assert_eq!(LITERATURE.len(), t.rules.len());
}

#[test]
fn table_is_consistent() {
    let t = table();
    assert!(t.classes.len() >= 12);
    let mut seen = std::collections::HashSet::new();
    for r in &t.rules {
        assert!(seen.insert(r.id.clone()), "duplicate rule id {}", r.id);
        assert!(!r.why.is_empty() && r.why.len() < 140);
    }
    for (ci, (id, _)) in t.classes.iter().enumerate() {
        assert!(
            t.rules
                .iter()
                .any(|r| r.class == ci && r.role == Role::Diagnostic),
            "class {id} has no diagnostic rule"
        );
    }
}

#[test]
fn a_diagnostic_rule_alone_lists_its_class() {
    let t = table();
    for r in t.rules.iter().filter(|r| r.role == Role::Diagnostic) {
        for pol in [Polarity::Pos, Polarity::Neg] {
            if r.polarity.is_some() && r.polarity != Some(pol) {
                continue;
            }
            let got = run(&[(300.0, 100.0), (peak_for(r, 0.0), 100.0)], pol, PPM);
            assert!(
                ids(&got).contains(&t.classes[r.class].0),
                "{} not listed in {pol:?}",
                r.id
            );
        }
    }
}

#[test]
fn a_rule_of_the_other_polarity_is_ignored() {
    let t = table();
    for r in t
        .rules
        .iter()
        .filter(|r| r.kind == Kind::Frag && r.role == Role::Diagnostic)
    {
        let other = if r.polarity == Some(Polarity::Pos) {
            Polarity::Neg
        } else {
            Polarity::Pos
        };
        let got = run(&[(300.0, 100.0), (peak_for(r, 0.0), 100.0)], other, PPM);
        assert!(
            !ids(&got).contains(&t.classes[r.class].0),
            "{} wrongly listed in {other:?}",
            r.id
        );
    }
}

#[test]
fn a_support_rule_alone_never_lists_its_class() {
    let t = table();
    for r in t.rules.iter().filter(|r| r.role == Role::Support) {
        let got = run(
            &[(300.0, 100.0), (peak_for(r, 0.0), 100.0)],
            pol_of(r, Polarity::Neg),
            PPM,
        );
        assert!(
            !ids(&got).contains(&t.classes[r.class].0),
            "{} alone lists {}",
            r.id,
            t.classes[r.class].0
        );
    }
    // The plain losses (H2O, NH3, CO2, CO) do not even make a class.
    for loss in [18.0106, 17.0265, 43.9898, 27.9949] {
        for pol in [Polarity::Pos, Polarity::Neg] {
            assert!(run(&[(300.0, 100.0), (PRECURSOR - loss, 100.0)], pol, PPM).is_empty());
        }
    }
}

#[test]
fn tolerance_edges_do_not_leak() {
    let t = table();
    for r in t.rules.iter().filter(|r| r.role == Role::Diagnostic) {
        let pol = pol_of(r, Polarity::Neg);
        let name = &t.classes[r.class].0;
        for k in [-3.0, -2.0, 2.0, 3.0] {
            let got = run(&[(300.0, 100.0), (peak_for(r, k), 100.0)], pol, PPM);
            assert!(
                !ids(&got).contains(name),
                "{} matched at {k} tolerances",
                r.id
            );
        }
        for k in [-0.5, 0.5] {
            let got = run(&[(300.0, 100.0), (peak_for(r, k), 100.0)], pol, PPM);
            assert!(
                ids(&got).contains(name),
                "{} missed at {k} tolerances",
                r.id
            );
        }
    }
}

#[test]
fn a_weak_peak_is_no_evidence() {
    let r = table()
        .rules
        .into_iter()
        .find(|r| r.id == "ph_149")
        .unwrap();
    let weak = (r.value, 1000.0 * MIN_REL * 0.5);
    assert!(run(&[(300.0, 1000.0), weak], Polarity::Pos, PPM).is_empty());
}

#[test]
fn levels_follow_the_number_of_diagnostic_matches() {
    let t = table();
    let get = |id: &str| t.rules.iter().find(|r| r.id == id).unwrap().clone();
    let one = run(
        &[(300.0, 100.0), (peak_for(&get("gl_loss"), 0.0), 100.0)],
        Polarity::Neg,
        PPM,
    );
    assert_eq!(one[0], ("glucuronide".to_string(), "3b", vec![]));
    let two = run(
        &[
            (300.0, 100.0),
            (peak_for(&get("gl_loss"), 0.0), 100.0),
            (get("gl_175").value, 100.0),
            (get("gl_113").value, 50.0),
        ],
        Polarity::Neg,
        PPM,
    );
    assert_eq!(two[0], ("glucuronide".to_string(), "3a", vec![]));
}

#[test]
fn same_peak_for_two_classes_lists_both() {
    // HSO4- and H2PO4- are 9.5 mDa apart: with a loose tolerance one peak in the middle belongs to both.
    let peaks = [(300.0, 100.0), (96.965, 100.0), (78.9591, 80.0)];
    let got = run(&peaks, Polarity::Neg, 100.0);
    let sulfate = got
        .iter()
        .find(|c| c.0 == "sulfate")
        .expect("sulfate listed");
    let phosphate = got
        .iter()
        .find(|c| c.0 == "phosphate")
        .expect("phosphate listed");
    assert_eq!(sulfate.2, vec!["phosphate".to_string()]);
    assert_eq!(phosphate.2, vec!["sulfate".to_string()]);
    assert_eq!((sulfate.1, phosphate.1), ("3b", "3b"));
    // With the real tolerance the same peak is not listed for either.
    assert!(run(&[(300.0, 100.0), (96.965, 100.0)], Polarity::Neg, PPM).is_empty());
}

#[test]
fn sulfonate_ion_is_shared_with_sulfate_and_taurine_only_as_support() {
    let got = run(&[(300.0, 100.0), (79.9574, 100.0)], Polarity::Neg, PPM);
    assert_eq!(ids(&got), vec!["sulfonic".to_string()]);
    assert_eq!(got[0].1, "3b");
}

#[test]
fn json_answer_has_the_contract() {
    let t = table();
    let get = |id: &str| t.rules.iter().find(|r| r.id == id).unwrap().clone();
    let rep = evaluate(
        &t,
        &[(300.0, 100.0), (get("ph_149").value + 0.0002, 80.0)],
        PRECURSOR,
        Polarity::Pos,
        PPM,
    )
    .unwrap();
    let v = json::parse(to_json(&rep).as_bytes()).unwrap();
    assert_eq!(v.get("format").unwrap().str(), Some("classi/1"));
    let c = v.get("classes").unwrap().at(0).unwrap();
    assert_eq!(c.get("id").unwrap().str(), Some("phthalate"));
    assert_eq!(c.get("level").unwrap().str(), Some("3b"));
    let e = c.get("evidence").unwrap().at(0).unwrap();
    for k in [
        "rule",
        "kind",
        "role",
        "formula",
        "theoretical",
        "observed",
        "peak_mz",
        "ppm",
        "rel_intensity",
        "why",
        "also",
    ] {
        assert!(e.get(k).is_some(), "missing {k}");
    }
    assert_eq!(e.get("theoretical").unwrap().f64(), Some(149.0233));
    assert!(e.get("ppm").unwrap().f64().unwrap().abs() < 5.0);
    assert!(c.get("unmet").unwrap().arr().is_empty());
    assert!(!to_json(&rep).to_lowercase().contains("identified"));
}

#[test]
fn invalid_input_is_an_error() {
    let t = table();
    assert!(evaluate(&t, &[], 0.0, Polarity::Pos, PPM).is_err());
    assert!(evaluate(&t, &[], PRECURSOR, Polarity::Pos, 0.0).is_err());
    assert!(evaluate(&t, &[(f64::NAN, 1.0)], PRECURSOR, Polarity::Pos, PPM).is_err());
    assert!(evaluate(&t, &[], PRECURSOR, Polarity::Pos, PPM)
        .unwrap()
        .classes
        .is_empty());
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Peaks at 2-3 tolerances from EVERY rule at once, sparse around the diagnostic values: no class.
#[test]
fn peaks_just_outside_every_window_list_no_class() {
    let t = table();
    for k in [-3.0, -2.0, 2.0, 3.0] {
        for pol in [Polarity::Pos, Polarity::Neg] {
            let mut peaks = vec![(300.0, 100.0)];
            peaks.extend(t.rules.iter().map(|r| (peak_for(r, k), 100.0)));
            let got = run(&peaks, pol, PPM);
            assert!(got.is_empty(), "{pol:?} at {k} tolerances: {got:?}");
        }
    }
}

/// Random peaks can fall in a window by chance (a loss window is +-{precursor*5e-6} Da wide): the rate stays under 1 spectrum in 100.
#[test]
fn random_noise_rarely_lists_a_class() {
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let (total, mut listed) = (2000, 0);
    for n in 0..total {
        let pol = if n % 2 == 0 {
            Polarity::Pos
        } else {
            Polarity::Neg
        };
        let peaks: Vec<(f64, f64)> = (0..30)
            .map(|_| (50.0 + rng.next() * 500.0, 1.0 + rng.next() * 999.0))
            .collect();
        if !run(&peaks, pol, PPM).is_empty() {
            listed += 1;
        }
    }
    eprintln!("noise spectra with a class: {listed}/{total}");
    assert!(listed * 100 < total, "{listed}/{total}");
}
