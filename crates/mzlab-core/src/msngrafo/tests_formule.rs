use super::tests::{arco, grafo_spostato, trova};
use super::*;

/// The fragments of the synthetic infusion (`tests::sintetico`) and their formulas.
const FRAMMENTI: [&str; 7] = [
    "C6H8N3O", "C5H8N3", "C5H6N3O", "C5H5N2O", "C4H7N2", "C3H5N2", "C2H4N",
];

fn con(formula: &str) -> Opzioni {
    Opzioni {
        formula: Some(formula.into()),
        ..Opzioni::default()
    }
}

fn proposta(g: &Grafo, i: usize) -> &str {
    &g.nodi[i].formula.as_ref().expect("a formula").proposta
}

fn chiavi(g: &Grafo) -> Vec<(&str, Option<usize>)> {
    g.avvisi.iter().map(|a| (a.chiave, a.nodo)).collect()
}

#[test]
fn every_fragment_gets_its_formula() {
    let (g, j) = grafo_spostato(&con("C8H10N4O2+H"), &[0, 1, 2, 3], false, 0.0);
    let ione = g.ione.as_ref().unwrap();
    assert_eq!(ione.formula, "C8H11N4O2");
    assert!(ione.ppm.abs() < 0.01);
    let r = trova(&g, "C8H11N4O2");
    let fr = g.nodi[r].formula.as_ref().unwrap();
    assert_eq!((fr.fonte, fr.motivo.0), ("radice", "why.msngrafo.root"));
    for f in FRAMMENTI {
        let n = g.nodi[trova(&g, f)].formula.as_ref().unwrap();
        assert_eq!(n.proposta, f);
        assert_eq!(n.fonte, "auto");
        assert!(n.pari && n.ppm.abs() < 0.01, "{f}: {n:?}");
    }
    assert_eq!(fr.dbe, 5.5);
    assert!(chiavi(&g).is_empty(), "{:?}", g.avvisi);
    // the losses: a known one under its usual name, the others in Hill order
    let co = arco(&g, "C6H8N3O", "C5H8N3").unwrap();
    assert_eq!(co.formula.as_ref().unwrap().0, "CO");
    let hcn = arco(&g, "C5H8N3", "C4H7N2").unwrap();
    assert_eq!(hcn.formula.as_ref().unwrap().0, "HCN");
    let p = arco(&g, "C8H11N4O2", "C6H8N3O").unwrap();
    assert_eq!(p.formula.as_ref().unwrap().0, "C2H3NO");
    let v = crate::json::parse(j.as_bytes()).unwrap();
    assert_eq!(
        v.get("ione").unwrap().get("formula").unwrap().str(),
        Some("C8H11N4O2")
    );
}

#[test]
fn the_ion_can_be_typed_in_three_ways() {
    let base = grafo_spostato(&con("C8H11N4O2+"), &[0, 1, 2, 3], false, 0.0).0;
    for t in ["C8H11N4O2", "C8H10N4O2 [M+H]+", "C8H10N4O2+H"] {
        let g = grafo_spostato(&con(t), &[0, 1, 2, 3], false, 0.0).0;
        assert_eq!(g, base, "{t}");
    }
}

#[test]
fn an_ion_of_the_other_polarity_or_unreadable_is_noted() {
    for (t, k) in [
        ("C8H10N4O2 [M-H]-", "warn.msngrafo.polarity"),
        ("C8H11N4O2-", "warn.msngrafo.polarity"),
        ("C8Xx", "warn.msngrafo.badFormula"),
    ] {
        let g = grafo_spostato(&con(t), &[0, 1, 2, 3], false, 0.0).0;
        assert_eq!(chiavi(&g), vec![(k, None)], "{t}");
        assert!(g.ione.is_none() && g.nodi.iter().all(|n| n.formula.is_none()));
    }
    // a root that is not in the graph
    let g = grafo_spostato(&con("C9H13N4O2"), &[0, 1, 2, 3], false, 0.0).0;
    assert_eq!(chiavi(&g), vec![("warn.msngrafo.rootMissing", None)]);
}

#[test]
fn one_shift_calibrates_every_mz() {
    let (g, _) = grafo_spostato(&con("C8H11N4O2"), &[0, 1, 2, 3], false, 3.0);
    let (ppm, n) = g.calibrazione.unwrap();
    assert!((ppm - 3.0).abs() < 0.05 && n >= 3, "{ppm} {n}");
    for f in FRAMMENTI {
        let i = g
            .nodi
            .iter()
            .position(|x| x.formula.as_ref().is_some_and(|y| y.proposta == f));
        let x = g.nodi[i.unwrap_or_else(|| panic!("{f}"))]
            .formula
            .as_ref()
            .unwrap();
        assert!(x.ppm.abs() < 0.1, "{f}: {}", x.ppm);
    }
    let senza = Opzioni {
        calibra: false,
        ..con("C8H11N4O2")
    };
    let (g, _) = grafo_spostato(&senza, &[0, 1, 2, 3], false, 3.0);
    assert!(g.calibrazione.is_none());
    let x = g
        .nodi
        .iter()
        .find_map(|n| n.formula.as_ref().filter(|f| f.proposta == "C5H8N3"));
    assert!((x.unwrap().ppm - 3.0).abs() < 0.05);
}

#[test]
fn a_wide_window_gives_alternatives_and_the_best_is_proposed() {
    let o = Opzioni {
        ppm: 300.0,
        ..con("C8H11N4O2")
    };
    let (g, _) = grafo_spostato(&o, &[0, 1, 2, 3], false, 0.0);
    let n = g.nodi[trova(&g, "C6H8N3O")].formula.as_ref().unwrap();
    assert_eq!(n.proposta, "C6H8N3O");
    assert!(
        n.n_candidati >= 2 && n.motivo.0 == "why.msngrafo.best",
        "{n:?}"
    );
    assert!(n.alternative.iter().any(|a| a.formula == "C5H6N4O"));
    assert!(n
        .alternative
        .windows(2)
        .all(|w| w[0].punteggio >= w[1].punteggio));
    // every proposal is under the proposals of its precursors
    let e = super::candidati::elementi(&crate::elements::parse_formula("C8H11N4O2").unwrap(), 1);
    let conta = |i: usize| {
        e.conta(&crate::elements::parse_formula(proposta(&g, i)).unwrap())
            .unwrap()
    };
    for a in g.archi.iter().filter(|a| g.nodi[a.a].formula.is_some()) {
        assert!(super::candidati::sotto(&conta(a.a), &conta(a.da)));
    }
}

#[test]
fn a_formula_fixed_by_the_user_wins_and_conflicts_are_noted() {
    let o = Opzioni {
        fissate: vec![
            (super::tests::ion("C5H8N3"), "C5H8N3".into()),
            (super::tests::ion("C4H7N2"), "C4H5NO".into()),
            (super::tests::ion("C2H4N"), "C2H4Cl".into()),
        ],
        ..con("C8H11N4O2")
    };
    let (g, j) = grafo_spostato(&o, &[0, 1, 2, 3], false, 0.0);
    let n110 = g.nodi[trova(&g, "C5H8N3")].formula.as_ref().unwrap();
    assert_eq!((n110.fonte, n110.motivo.0), ("utente", "why.msngrafo.user"));
    let k = trova(&g, "C4H7N2");
    assert_eq!(proposta(&g, k), "C4H5NO");
    assert!(arco(&g, "C5H8N3", "C4H7N2").unwrap().formula.is_none());
    assert_eq!(
        chiavi(&g),
        vec![
            ("warn.msngrafo.userConflict", Some(trova(&g, "C2H4N"))),
            ("warn.msngrafo.userConflict", Some(k)),
        ]
    );
    // the fixed formulas are in the JSON, so the graph can be rebuilt
    let v = crate::json::parse(j.as_bytes()).unwrap();
    assert_eq!(
        v.get("opzioni")
            .unwrap()
            .get("fissate")
            .unwrap()
            .arr()
            .len(),
        3
    );
}
