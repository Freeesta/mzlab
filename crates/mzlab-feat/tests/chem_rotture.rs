//! `chem::rotture`: the bond breaks of a public molecule (ethyl indole-3-carboxylate) on a graph built by hand: M+H 190.08626, the acid 162.05495
//! (ester bond), the acylium 144.04439 (the acid piece without its OH, two hydrogens moved) and a node no piece gives.

use mzlab_core::msngrafo::{Edge, GNode, Graph, SpettroG};
use mzlab_feat::chem::rotture::{rotture, Parametri};

const SMILES: &str = "CCOC(=O)c1c[nH]c2ccccc12";

fn nodo(mz: f64, livello: u8) -> GNode {
    GNode {
        mz,
        dev_ppm: 0.0,
        livello,
        albero: vec![],
        observations: 1,
        rel_max: 100.0,
        isotope_of: None,
        m1: None,
        width: None,
        upstream: None,
        unexplained: None,
        pos: (0, 0),
        formula: None,
    }
}

fn arco(da: usize, a: usize) -> Edge {
    Edge {
        da,
        a,
        perdita: 0.0,
        compatibili: vec![],
        formula: None,
        isolamento: false,
        misure: vec![],
    }
}

fn grafo() -> Graph {
    Graph {
        nodi: vec![
            nodo(190.08626, 1),
            nodo(162.05495, 2),
            nodo(144.04439, 3),
            nodo(120.5, 2),
        ],
        archi: vec![arco(0, 1), arco(0, 3), arco(1, 2)],
        radici: vec![0],
        polarita: 1,
        spettri: vec![SpettroG {
            albero: "190@cid35".into(),
            level: 2,
            att: "cid".into(),
            ce: 35.0,
            precursore: Some(0),
            width: None,
            picchi: vec![
                (162.055, 60.0, Some(1)),
                (120.5, 20.0, Some(3)),
                (99.0, 20.0, None),
            ],
        }],
        ..Default::default()
    }
}

#[test]
fn the_pieces_of_the_bond_breaks_explain_the_arcs_they_cover() {
    let r = rotture(&grafo(), SMILES, Parametri::default()).unwrap();
    let nodi: Vec<usize> = r.nodes.iter().map(|n| n.node).collect();
    assert_eq!(nodi, [0, 1, 2]);
    assert_eq!(r.nodes[0].ipotesi[0].legami, Vec::<usize>::new());
    assert_eq!(r.nodes[1].ipotesi[0].formula, "C9H7NO2");
    assert_eq!(r.nodes[1].ipotesi[0].h, 0);
    assert_eq!(r.nodes[2].ipotesi[0].h, -2);
    assert!(r.nodes[2].ipotesi[0].ppm.abs() < 1.0);
    let spiegati: Vec<bool> = r.edges.iter().map(|a| a.spiegato).collect();
    assert_eq!(spiegati, [true, false, true]);
    // 60 of the 100 % of the spectrum sits on an explained arc
    assert_eq!(r.spettri[0].explained, 60.0);
    assert_eq!(r.spettri[0].total, 100.0);
    let (sp, tot, int) = r.counts();
    assert_eq!((sp, tot), (2, 3));
    assert!((int - 60.0).abs() < 1e-9);
}

#[test]
fn no_hydrogen_transfer_loses_the_acylium_and_one_bond_is_enough_for_the_acid() {
    let p = Parametri {
        h_max: 0,
        max_legami: 1,
        ..Parametri::default()
    };
    let r = rotture(&grafo(), SMILES, p).unwrap();
    assert!(r.nodes.iter().all(|n| n.node != 2));
    assert!(r.nodes.iter().any(|n| n.node == 1));
    assert_eq!(r, rotture(&grafo(), SMILES, p).unwrap());
}

#[test]
fn a_mixture_gives_no_pieces_and_a_bad_smiles_is_an_error() {
    let r = rotture(
        &grafo(),
        "CCOC(=O)c1c[nH]c2ccccc12.Cl",
        Parametri::default(),
    )
    .unwrap();
    assert!(r.nodes.is_empty());
    assert_eq!(r.warnings, ["warn.msngrafo.structMixture"]);
    assert!(rotture(&grafo(), "C(C", Parametri::default()).is_err());
}
