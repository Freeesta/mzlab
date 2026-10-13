//! The observed network (`tp::rete`): arcs of the differences of mass of the rules among co-varying rows, the rows they explain up to the third step, and the
//! hypotheses of the next round from the accepted products.

use mzlab_core::elements::{self, ion_mz};
use mzlab_feat::evid::rowproofs::{Signs, FOR, NONE};
use mzlab_feat::hypo::{
    Dir, HypoParams, Hypothesis, Outcome, Registry, Rules, SRC_NETWORK, SRC_RULE,
};
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};
use mzlab_feat::tp::*;

/// Files: blank, t0, then 5, 10, 20, 40 and 80 minutes of treatment.
const MINUTES: [f64; 7] = [f64::NAN, 0.0, 5.0, 10.0, 20.0, 40.0, 80.0];
const O: f64 = 15.994915;

fn rules() -> Rules {
    let j = r#"{"format":"regole_tp","version":1,"regole":[{"id":"oh","nome":"hydroxylation","delta_formula":"+O","passi_max":1,"livello_previsione":3,"fonte":"transformation_table","direzione_rt":"piu_polare"}]}"#;
    Rules::parse(j.as_bytes()).unwrap()
}

fn table(rows: &[(f64, f64, [f64; 7])]) -> Aligned {
    let mut t = Aligned::with_capacity(7, rows.len());
    for (mz, rt, a) in rows {
        t.mz.push(*mz);
        t.rt.push(*rt);
        for v in a {
            t.area.push(*v);
            t.height.push((v / 10.0) as f32);
            t.apex_rt.push(*rt as f32);
            t.width.push(0.1);
            t.state
                .push(if *v > 0.0 { CELL_DETECTED } else { CELL_ABSENT });
        }
    }
    t
}

/// The parent, a chain of four hydroxylations that rise one after the other, and a row of the right mass that does not move with its neighbour.
fn chain() -> Aligned {
    table(&[
        (300.0, 5.0, [0.0, 1e7, 9e6, 7e6, 5e6, 3e6, 1e6]),
        (300.0 + O, 4.5, [0.0, 0.0, 2e5, 5e5, 8e5, 9e5, 9e5]),
        (300.0 + 2.0 * O, 4.0, [0.0, 0.0, 1e5, 3e5, 6e5, 9e5, 1e6]),
        (300.0 + 3.0 * O, 3.5, [0.0, 0.0, 0.0, 1e5, 2e5, 5e5, 8e5]),
        (300.0 + 4.0 * O, 3.0, [0.0, 0.0, 0.0, 0.0, 1e5, 3e5, 6e5]),
        // 15.995 above the first product, but flat while it rises: no arc
        (
            300.0 + 2.0 * O + 0.0,
            6.0,
            [0.0, 0.0, 4e5, 4e5, 4e5, 4e5, 4e5],
        ),
    ])
}

fn build_with(t: &Aligned, hyps: &[Hypothesis], signs: &[Signs], rules: Option<&Rules>) -> TpBlock {
    let inp = TpInputs {
        aligned: t,
        minutes: &MINUTES,
        hyps,
        signs,
        parent: (300.0, 5.0),
        fragments: None,
        params: &TpParams::default(),
        rules,
    };
    build(&inp).unwrap().expect("the parent is in the table")
}

#[test]
fn the_network_explains_the_chain_up_to_the_third_step() {
    let (t, r) = (chain(), rules());
    let signs = vec![[NONE; 10]; t.len()];
    let without = build_with(&t, &[], &signs, None);
    assert_eq!(
        without.count(ST_UNEXPLAINED),
        5,
        "no network: nothing explains them"
    );
    let b = build_with(&t, &[], &signs, Some(&r));
    assert_eq!(
        &b.status[..5],
        &[ST_PARENT, ST_TP, ST_TP, ST_TP, ST_UNEXPLAINED],
        "the fourth step is out"
    );
    assert!(b.via[1..4].iter().all(|&v| v == VIA_NETWORK));
    assert_eq!(b.via[4], 0);
    assert_eq!(b.status[5], ST_UNEXPLAINED, "a flat row has no arc");
    // a network explanation, a kinetic proof for and nothing against make the row probable
    let mut s = signs.clone();
    s[2][5] = FOR;
    assert_eq!(build_with(&t, &[], &s, Some(&r)).kinship[2], KIN_PROBABLE);
}

#[test]
fn the_arcs_run_from_the_row_the_rule_starts_at_and_carry_the_generation() {
    let (t, r) = (chain(), rules());
    let signs = vec![[NONE; 10]; t.len()];
    let mut b = build_with(&t, &[], &signs, None);
    let inp = TpInputs {
        aligned: &t,
        minutes: &MINUTES,
        hyps: &[],
        signs: &signs,
        parent: (300.0, 5.0),
        fragments: None,
        params: &TpParams::default(),
        rules: Some(&r),
    };
    let used = explain_by_arcs(&inp, &r, &mut b, &NetParams::default());
    let mut got: Vec<(u32, u32, u8)> = used.iter().map(|a| (a.from, a.to, a.generation)).collect();
    got.sort();
    assert_eq!(got, vec![(0, 1, 1), (1, 2, 2), (2, 3, 3)]);
    let all = arcs(&inp, &r, &b, &NetParams::default());
    assert!(
        all.windows(2)
            .all(|w| (w[0].from, w[0].to) <= (w[1].from, w[1].to)),
        "sorted"
    );
    assert!(all.iter().all(|a| a.rule == 0));
}

fn hyp(formula: &str, steps: u8) -> Hypothesis {
    let neutral = elements::mass(&elements::parse_formula(formula).unwrap());
    Hypothesis {
        id: 0,
        sources: SRC_RULE,
        label: "oh".into(),
        formula: formula.into(),
        neutral,
        mz: ion_mz(neutral, "[M+H]+").unwrap(),
        adduct: "[M+H]+",
        polarity: 1,
        steps,
        level: 3,
        dir: Dir::Free,
        rt_ref: None,
        alternatives: Vec::new(),
    }
}

#[test]
fn the_next_round_starts_from_the_accepted_products_and_stops_at_the_third_step() {
    let h1 = hyp("C10H20N2O3", 1);
    let h3 = hyp("C10H20N2O5", 3);
    let parent_mz = 300.0;
    let t = table(&[
        (parent_mz, 5.0, [0.0, 1e7, 9e6, 7e6, 5e6, 3e6, 1e6]),
        (h1.mz, 4.0, [0.0, 0.0, 2e5, 5e5, 8e5, 9e5, 9e5]),
        (h3.mz, 3.0, [0.0, 0.0, 1e5, 3e5, 6e5, 9e5, 1e6]),
    ]);
    let reg = Registry {
        hyps: vec![h1.clone(), h3.clone()],
        outcomes: vec![Outcome::NotSearched; 2],
        rules_version: 1,
        polarity: 1,
        suspects_cut: 0,
    };
    let mut signs = vec![[NONE; 10]; 3];
    signs[1][5] = FOR;
    signs[2][5] = FOR;
    let b = build_with(&t, &reg.hyps, &signs, None);
    assert_eq!((b.kinship[1], b.kinship[2]), (KIN_PROBABLE, KIN_PROBABLE));
    let r = rules();
    let p = HypoParams::default();
    let next = next_generation(&r, &reg, &t, &b, &p, &NetParams::default());
    assert_eq!(next.len(), 1, "the third step is not taken further");
    let n = &next[0];
    assert_eq!(
        (n.formula.as_str(), n.sources, n.steps, n.label.as_str()),
        ("C10H20N2O4", SRC_NETWORK, 2, "oh>oh")
    );
    // a kinship below the threshold starts nothing
    let low = NetParams {
        min_kinship: KIN_CERTAIN,
        ..NetParams::default()
    };
    assert!(next_generation(&r, &reg, &t, &b, &p, &low).is_empty());
    // appended to the registry: the ids and outcomes of the first round stay, the new one gets the next id
    let mut reg2 = reg.clone();
    assert_eq!(reg2.append(next.clone()), 1);
    assert_eq!((reg2.hyps[2].id, reg2.outcomes.len()), (2, 3));
    assert_eq!(reg2.hyps[0], reg.hyps[0]);
    // the same list again adds nothing
    assert_eq!(reg2.append(next), 0);
    assert!(reg2.to_json().contains("\"rete\""));
}
