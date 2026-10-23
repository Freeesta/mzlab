use super::tests::{arco, grafo_con, grafo_spostato, ion, trova, Strumento};
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

fn f(g: &Grafo, i: usize) -> &FormulaNodo {
    g.nodi[i].formula.as_ref().expect("a formula")
}

/// The node whose candidates include `formula` (the m/z may be shifted).
fn nodo(g: &Grafo, formula: &str) -> Option<usize> {
    g.nodi.iter().position(|n| {
        n.formula
            .as_ref()
            .is_some_and(|x| x.candidati.iter().any(|c| c.formula == formula))
    })
}

fn chiavi(g: &Grafo) -> Vec<(&str, Option<usize>)> {
    g.avvisi.iter().map(|a| (a.chiave, a.nodo)).collect()
}

fn radice(g: &Grafo) -> usize {
    g.nodi
        .iter()
        .position(|n| {
            n.formula
                .as_ref()
                .is_some_and(|x| x.fonte == Some("radice"))
        })
        .unwrap()
}

#[test]
fn every_fragment_gets_its_sub_formula() {
    let o = Opzioni {
        smiles: Some("CN1C=NC2=C1C(=O)N(C(=O)N2C)C".into()),
        ..con("C8H10N4O2+H")
    };
    let (g, j) = grafo_spostato(&o, &[0, 1, 2, 3], false, 0.0);
    let ione = g.ione.as_ref().unwrap();
    assert_eq!(ione.formula, "C8H11N4O2");
    assert!(ione.ppm.abs() < 0.01);
    let r = f(&g, trova(&g, "C8H11N4O2"));
    assert_eq!(
        (r.scelta.as_deref(), r.fonte),
        (Some("C8H11N4O2"), Some("radice"))
    );
    assert_eq!(r.candidati[0].dbe, 5.5);
    for x in FRAMMENTI {
        let n = f(&g, trova(&g, x));
        assert_eq!(
            (n.scelta.as_deref(), n.fonte),
            (Some(x), Some("unica")),
            "{n:?}"
        );
        let c = &n.candidati[0];
        assert!(c.pari && c.ppm.abs() < 0.01 && c.finestra == 10.0 && c.prove.is_empty());
    }
    assert_eq!(g.nodi.len(), 8);
    assert_eq!(g.scartati.formula, 0);
    assert!(chiavi(&g).is_empty(), "{:?}", g.avvisi);
    // the M+1 of the 138 is in its spectrum
    assert!((g.nodi[trova(&g, "C6H8N3O")].m1.unwrap() - 7.0).abs() < 1e-9);
    // the losses: a known one under its usual name, the others in Hill order
    let perdita = |a: &str, b: &str| arco(&g, a, b).unwrap().formula.clone().unwrap().0;
    assert_eq!(perdita("C6H8N3O", "C5H8N3"), "CO");
    assert_eq!(perdita("C5H8N3", "C4H7N2"), "HCN");
    assert_eq!(perdita("C8H11N4O2", "C6H8N3O"), "C2H3NO");
    let v = crate::json::parse(j.as_bytes()).unwrap();
    assert_eq!(
        v.get("ione").unwrap().get("formula").unwrap().str(),
        Some("C8H11N4O2")
    );
    assert_eq!(
        v.get("opzioni").unwrap().get("smiles").unwrap().str(),
        o.smiles.as_deref()
    );
}

#[test]
fn the_ion_can_be_typed_in_several_ways() {
    let base = grafo_spostato(&con("C8H11N4O2+"), &[0, 1, 2, 3], false, 0.0).0;
    for t in [
        "C8H11N4O2",
        "C8H11N4O2 [M]+",
        "C8H10N4O2 [M+H]+",
        "C8H10N4O2+H",
    ] {
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
        ("C9H13N4O2", "warn.msngrafo.rootMissing"),
    ] {
        let g = grafo_spostato(&con(t), &[0, 1, 2, 3], false, 0.0).0;
        assert_eq!(chiavi(&g), vec![(k, None)], "{t}");
        assert!(g.nodi.iter().all(|n| n.formula.is_none()) && g.nodi.len() == 8);
    }
}

#[test]
fn below_mz_150_a_wider_window_needs_an_evidence() {
    // every m/z off by 15 ppm: out of the 10 ppm window, inside the 20 ppm one below m/z 150
    let (g, _) = grafo_spostato(&con("C8H11N4O2"), &[0, 1, 2, 3], false, 15.0);
    assert_eq!(
        chiavi(&g),
        vec![("warn.msngrafo.rootMissing", Some(radice(&g)))]
    );
    let prove = |x: &str| {
        let n = f(&g, nodo(&g, x).unwrap_or_else(|| panic!("{x}")));
        let c = &n.candidati[0];
        assert!((c.ppm - 15.0).abs() < 0.1 && c.finestra == 20.0, "{c:?}");
        c.prove.clone()
    };
    // the 138 by its M+1, the 110 and the 83 by a known loss (CO, HCN)
    assert_eq!(prove("C6H8N3O"), vec!["isotopi"]);
    assert_eq!(prove("C5H8N3"), vec!["perdita"]);
    assert_eq!(prove("C4H7N2"), vec!["perdita"]);
    // no evidence for the others: they leave the graph...
    for x in ["C5H6N3O", "C5H5N2O", "C3H5N2", "C2H4N"] {
        assert!(nodo(&g, x).is_none(), "{x}");
    }
    // ...but the shifted 124 is within 10 ppm of another sub-formula of the root: a candidate, not an identification
    let n124 = f(&g, nodo(&g, "C7H8O2").unwrap());
    assert!(n124.candidati[0].ppm.abs() < 10.0 && n124.candidati[0].prove.is_empty());
    assert_eq!((g.nodi.len(), g.scartati.formula), (5, 3));
    // two evidences asked: a candidate of the wide window needs both
    let o = Opzioni {
        prove_min: 2,
        ..con("C8H11N4O2")
    };
    let (g, _) = grafo_spostato(&o, &[0, 1, 2, 3], false, 15.0);
    assert!(nodo(&g, "C6H8N3O").is_none());
    for n in g
        .nodi
        .iter()
        .filter_map(|n| n.formula.as_ref())
        .filter(|n| n.fonte != Some("radice"))
    {
        assert!(
            n.candidati
                .iter()
                .all(|c| c.ppm.abs() <= 10.0 || c.prove.len() >= 2),
            "{n:?}"
        );
    }
}

#[test]
fn an_isolated_precursor_left_unexplained_is_noted() {
    // the 138 is isolated: out of every window, it leaves the graph with a note on its nodes of the tree
    let o = Opzioni {
        ppm_formula: 3.0,
        ppm_basso: 3.0,
        ..con("C8H11N4O2")
    };
    let (g, _) = grafo_spostato(&o, &[0, 1, 2, 3], false, 15.0);
    let tolti: Vec<(&str, &str)> = g
        .avvisi
        .iter()
        .map(|a| (a.chiave, a.albero.as_str()))
        .collect();
    let attesi = [
        ("warn.msngrafo.isolatedUnexplained", "195@cid30>138@cid30"),
        ("warn.msngrafo.isolatedUnexplained", "195@cid30>138@hcd45"),
    ];
    assert!(attesi.iter().all(|x| tolti.contains(x)), "{tolti:?}");
    assert!(nodo(&g, "C6H8N3O").is_none());
}

#[test]
fn most_intensity_unexplained_is_noted_on_the_graph_only_then() {
    let nota = |g: &Grafo| g.avvisi.iter().any(|a| a.chiave == "warn.msngrafo.mostUnexplained");
    let (bene, _) = grafo_spostato(&con("C8H11N4O2"), &[0, 1, 2, 3], false, 0.0);
    assert!(!nota(&bene), "{:?}", bene.avvisi);
    // fragments 15 ppm off with a 3 ppm window: none has a candidate
    let stretta = Opzioni {
        ppm_formula: 3.0,
        ppm_basso: 3.0,
        ..con("C8H11N4O2")
    };
    let (male, _) = grafo_spostato(&stretta, &[0, 1, 2, 3], false, 15.0);
    assert!(nota(&male), "{:?} {:?}", male.avvisi, male.scartati);
}

#[test]
fn several_candidates_are_listed_by_ppm_and_none_is_chosen() {
    let o = Opzioni {
        ppm_formula: 300.0,
        ppm_basso: 300.0,
        ..con("C8H11N4O2")
    };
    let (g, _) = grafo_spostato(&o, &[0, 1, 2, 3], false, 0.0);
    let n = f(&g, trova(&g, "C6H8N3O"));
    assert!(
        n.candidati.len() >= 2 && n.scelta.is_none() && n.fonte.is_none(),
        "{n:?}"
    );
    assert_eq!(n.candidati[0].formula, "C6H8N3O");
    assert!(n.candidati.iter().any(|c| c.formula == "C5H6N4O"));
    assert!(n
        .candidati
        .windows(2)
        .all(|w| w[0].ppm.abs() <= w[1].ppm.abs()));
    // every candidate is under a candidate of each precursor
    let e = super::candidati::elementi(&crate::elements::parse_formula("C8H11N4O2").unwrap(), 1);
    let conta = |s: &str| {
        e.conta(&crate::elements::parse_formula(s).unwrap())
            .unwrap()
    };
    for a in &g.archi {
        for c in &f(&g, a.a).candidati {
            let sopra = &f(&g, a.da).candidati;
            let ok = sopra
                .iter()
                .any(|q| super::candidati::sotto(&conta(&c.formula), &conta(&q.formula)));
            assert!(ok, "{} under {:?}", c.formula, sopra);
        }
    }
    // with no choice, no formula of the losses
    assert!(arco(&g, "C8H11N4O2", "C6H8N3O").unwrap().formula.is_none());
}

#[test]
fn the_choice_of_the_user_wins_and_conflicts_are_noted() {
    let o = Opzioni {
        scelte: vec![
            (ion("C5H8N3"), "C5H8N3".into()),
            (ion("C4H7N2"), "C4H5NO".into()),
            (ion("C2H4N"), "C2H4Cl".into()),
        ],
        ..con("C8H11N4O2")
    };
    let (g, j) = grafo_spostato(&o, &[0, 1, 2, 3], false, 0.0);
    assert_eq!(f(&g, trova(&g, "C5H8N3")).fonte, Some("utente"));
    let k = trova(&g, "C4H7N2");
    assert_eq!(f(&g, k).scelta.as_deref(), Some("C4H5NO"));
    assert!(arco(&g, "C5H8N3", "C4H7N2").unwrap().formula.is_none());
    assert_eq!(
        chiavi(&g),
        vec![
            ("warn.msngrafo.userConflict", Some(k)),
            ("warn.msngrafo.userConflict", Some(trova(&g, "C2H4N"))),
        ]
    );
    // the choices are in the JSON, so the graph can be rebuilt
    let v = crate::json::parse(j.as_bytes()).unwrap();
    assert_eq!(
        v.get("opzioni").unwrap().get("scelte").unwrap().arr().len(),
        3
    );
}

#[test]
fn a_negative_infusion_is_read_the_same_way() {
    let neg = Strumento {
        negativo: true,
        ..Strumento::default()
    };
    for t in [
        "C8H11N4O2-",
        "C8H12N4O2 [M-H]-",
        "C8H12N4O2-H",
        "C6H8N4 [M+CH3COO]-",
        "C6H8N4+CH3COO",
    ] {
        let (g, _) = grafo_con(&con(t), &[0, 1, 2, 3], false, 0.0, &neg);
        let ione = g
            .ione
            .as_ref()
            .unwrap_or_else(|| panic!("{t}: {:?}", g.avvisi));
        assert_eq!((g.polarita, ione.formula.as_str()), (-1, "C8H11N4O2"));
        // the electron is added, not taken: the anions are 2 electrons above the cations
        assert!(ione.ppm.abs() < 0.01, "{t}: {}", ione.ppm);
        for x in FRAMMENTI {
            let n = f(&g, nodo(&g, x).unwrap_or_else(|| panic!("{t}: {x}")));
            assert!(n.candidati[0].ppm.abs() < 0.01 && n.scelta.as_deref() == Some(x));
        }
        assert!(chiavi(&g).is_empty(), "{t}: {:?}", g.avvisi);
    }
    // a positive adduct on a negative file
    let (g, _) = grafo_con(&con("C8H10N4O2 [M+H]+"), &[0, 1, 2, 3], false, 0.0, &neg);
    assert_eq!(chiavi(&g), vec![("warn.msngrafo.polarity", None)]);
}

#[test]
fn behind_a_narrow_window_the_isotopes_are_no_evidence() {
    // 15 ppm off (the wide window): the 138 has only its M+1 as evidence
    let stretta = Strumento {
        larghezza: Some(1.0),
        ..Strumento::default()
    };
    let (g, j) = grafo_con(&con("C8H11N4O2"), &[0, 1, 2, 3], false, 15.0, &stretta);
    assert!(nodo(&g, "C6H8N3O").is_none());
    let r = radice(&g);
    assert_eq!(g.nodi[r].larghezza, Some(1.0));
    let n124 = &g.nodi[nodo(&g, "C7H8O2").unwrap()];
    assert_eq!(n124.a_monte, Some(1.0));
    let v = crate::json::parse(j.as_bytes()).unwrap();
    let iso = |i: usize| {
        v.get("nodi").unwrap().arr()[i]
            .get("isolamento")
            .unwrap()
            .get("stretto")
            .unwrap()
            .clone()
    };
    assert_eq!(
        iso(nodo(&g, "C7H8O2").unwrap()),
        crate::json::Value::Bool(true)
    );
    // a window that takes the M+1: the evidence counts
    let larga = Strumento {
        larghezza: Some(3.0),
        ..Strumento::default()
    };
    let (g, _) = grafo_con(&con("C8H11N4O2"), &[0, 1, 2, 3], false, 15.0, &larga);
    let n = f(&g, nodo(&g, "C6H8N3O").unwrap());
    assert_eq!(n.candidati[0].prove, vec!["isotopi"]);
}

#[test]
fn the_unexplained_peaks_are_counted_on_their_precursor() {
    let (g, _) = grafo_spostato(&con("C8H11N4O2"), &[0, 1, 2, 3], false, 15.0);
    let ns = |x: &str| g.nodi[nodo(&g, x).unwrap()].non_spiegati.clone().unwrap();
    // the 138 lost the 109 of its MS3; the 110 lost the 69 and the 42 of its MS4, the stronger first
    assert_eq!(ns("C6H8N3O").n, 1);
    let n110 = ns("C5H8N3");
    assert_eq!((n110.n, n110.picchi.len()), (2, 2));
    assert!(n110.picchi[0].1 >= n110.picchi[1].1);
    assert!(g.nodi[radice(&g)].non_spiegati.is_none());
    // in the spectra a peak with no node is an unexplained one: the clean spectrum is the others
    let ms4 = g
        .spettri
        .iter()
        .find(|s| s.albero == "195@cid30>138@cid30>110@cid35")
        .unwrap();
    assert_eq!(ms4.picchi.iter().filter(|p| p.2.is_none()).count(), 2);
    assert_eq!(ms4.precursore, nodo(&g, "C5H8N3"));
    // without a formula every peak has its node
    let (g0, _) = grafo_spostato(&Opzioni::default(), &[0, 1, 2, 3], false, 15.0);
    assert_eq!(g0.spettri.len(), 4);
    assert!(g0
        .spettri
        .iter()
        .all(|s| s.picchi.iter().all(|p| p.2.is_some())));
}
