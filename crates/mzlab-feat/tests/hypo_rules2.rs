//! `regole_tp/2`: profiles, rules with preconditions on the functional groups of the parent (from its SMILES) and the rules for when the structure is not known.
//! The parent is the progenitor C13H24N4O3S, written only by its SMILES.

use std::path::{Path, PathBuf};

use mzlab_feat::chem::grafo::Mol;
use mzlab_feat::hypo::*;

const PARENT: &str = "CC(C)(C)NCC(COc1nsnc1N1CCOCC1)O";
const FORMULA: &str = "C13H24N4O3S";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn file() -> Vec<u8> {
    std::fs::read(root().join("TP_Mine/data/regole_tp_2.json")).unwrap()
}

fn run(rules: &Rules, groups: Option<&[String]>) -> Registry {
    let frags: Vec<String> = Vec::new();
    let inp = Inputs {
        parent: FORMULA,
        suspects: None,
        families: &[],
        imported: &[],
        fragments: &frags,
        groups,
    };
    generate(rules, &inp, &HypoParams::default()).unwrap()
}

fn groups() -> Vec<String> {
    Mol::parse(PARENT)
        .unwrap()
        .groups()
        .iter()
        .map(|g| g.to_string())
        .collect()
}

fn find<'a>(r: &'a Registry, formula: &str) -> &'a Hypothesis {
    r.hyps
        .iter()
        .find(|h| h.formula == formula && h.steps == 1)
        .unwrap_or_else(|| panic!("{formula} missing"))
}

#[test]
fn the_file_has_three_profiles_and_the_default_is_photocatalysis() {
    let r = Rules::parse(&file()).unwrap();
    assert_eq!((r.version, r.profile.as_str()), (2, "fotocatalisi"));
    assert!(r.rules.len() > 40);
    assert!(r.rules.iter().all(|x| !x.class.is_empty()));
    let met = Rules::parse_profile(&file(), Some("metabolismo_I_II")).unwrap();
    assert_eq!(
        met.rules.len(),
        r.rules.len() + 5,
        "phase I inherited, phase II added"
    );
    assert!(met
        .rules
        .iter()
        .any(|x| x.id == "glucuronidation" && x.delta_text == "+C6H8O6"));
    assert_eq!(
        Rules::parse_profile(&file(), Some("ozono_cloro"))
            .err()
            .unwrap()
            .key,
        "error.hypo.profileFrozen"
    );
    assert_eq!(
        Rules::parse_profile(&file(), Some("altro"))
            .err()
            .unwrap()
            .key,
        "error.hypo.profile"
    );
}

#[test]
fn a_v1_file_is_still_read_and_says_so() {
    let r =
        Rules::parse(&std::fs::read(root().join("TP_Mine/data/regole_tp.json")).unwrap()).unwrap();
    assert_eq!((r.version, r.profile.as_str()), (1, ""));
    assert!(r
        .rules
        .iter()
        .all(|x| x.groups_all.is_empty() && x.groups_any.is_empty() && !x.fallback));
    assert_eq!(run(&r, None).rules_version, 1);
    assert_eq!(
        Rules::parse_profile(
            &std::fs::read(root().join("TP_Mine/data/regole_tp.json")).unwrap(),
            Some("fotocatalisi")
        )
        .err()
        .unwrap()
        .key,
        "error.hypo.rulesVersion"
    );
}

#[test]
fn with_the_structure_the_groups_decide() {
    let rules = Rules::parse(&file()).unwrap();
    let g = groups();
    let with = run(&rules, Some(&g));
    assert_eq!(with.rules_version, 2);
    // the tert-butyl on N: literature level, by the specific rule; the generic fallback is not used
    let c4 = find(&with, "C9H16N4O3S");
    assert_eq!((c4.label.as_str(), c4.level), ("n_detertbutylation", 2));
    assert!(with.hyps.iter().all(|h| !h.label.contains("loss_of_c4h8")));
    // no methyl, no isopropyl, no carboxylic acid, no halogen, no nitro on this structure
    for no in [
        "n_demethylation",
        "n_deisopropylation",
        "photo_kolbe",
        "dechlorination",
        "denitration",
        "desulfonylation",
    ] {
        assert!(with.hyps.iter().all(|h| !h.label.contains(no)), "{no}");
    }
    // the morpholine opens, the thiadiazole sulfur is oxidised
    assert!(with.hyps.iter().any(|h| h.label.contains("ring_opening")));
    assert!(with.hyps.iter().any(|h| h.label.contains("s_oxidation")
        || h.alternatives.iter().any(|a| a.contains("s_oxidation"))));
}

#[test]
fn without_the_structure_only_the_fallback_and_plain_rules_work() {
    let rules = Rules::parse(&file()).unwrap();
    let without = run(&rules, None);
    let c4 = find(&without, "C9H16N4O3S");
    assert_eq!((c4.label.as_str(), c4.level), ("loss_of_c4h8", 4));
    assert!(without
        .hyps
        .iter()
        .all(|h| !h.label.contains("n_detertbutylation")
            && !h.label.contains("ring_opening")
            && !h.label.contains("photo_kolbe")));
    // hydroxylation is there either way
    assert_eq!(find(&without, "C13H24N4O4S").label, "hydroxylation");
    assert_eq!(
        find(&run(&rules, Some(&groups())), "C13H24N4O4S").label,
        "hydroxylation"
    );
    // a structure with no group at all leaves the plain rules
    let none: Vec<String> = Vec::new();
    let bare = run(&rules, Some(&none));
    assert!(
        bare.hyps.iter().any(|h| h.label == "hydroxylation")
            && bare.hyps.iter().all(|h| !h.label.contains("loss_of_c4h8"))
    );
}

#[test]
fn the_structure_json_of_the_page() {
    let s = Structure::parse(&mzlab_feat::chem::grafo::analyse_json(PARENT).unwrap()).unwrap();
    assert_eq!(s.formula.as_deref(), Some(FORMULA));
    assert!(s.groups.as_ref().unwrap().iter().any(|g| g == "n_tbutyl") && !s.fragments.is_empty());
    s.check_parent("C13H24N4S1O3").unwrap();
    assert_eq!(
        s.check_parent("C12H17ClN2O3S").err().unwrap().key,
        "error.hypo.structureMismatch"
    );
    assert_eq!(Structure::parse("").unwrap(), Structure::default());
    assert_eq!(
        Structure::parse("{").err().unwrap().key,
        "error.hypo.structure"
    );
    let p = Structure::parse(r#"{"profile":"metabolismo_I_II"}"#).unwrap();
    assert_eq!(
        (p.profile.as_deref(), p.groups),
        (Some("metabolismo_I_II"), None)
    );
    // the cleavage formulas go to the hypotheses as they are
    let frags = s.fragments.clone();
    let rules = Rules::parse(&file()).unwrap();
    let g = s.groups.clone();
    let reg = generate(
        &rules,
        &Inputs {
            parent: FORMULA,
            suspects: None,
            families: &[],
            imported: &[],
            fragments: &frags,
            groups: g.as_deref(),
        },
        &HypoParams::default(),
    )
    .unwrap();
    assert!(reg.hyps.iter().any(|h| h.sources & SRC_CLEAVAGE != 0));
}
