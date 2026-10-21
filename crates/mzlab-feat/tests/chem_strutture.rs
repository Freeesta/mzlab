//! `chem::strutture`: the cleavages of a public molecule (ethyl indole-3-carboxylate) that give the m/z of a node, with the peaks of its spectrum
//! that the same piece explains by a neutral loss.

use mzlab_core::msngrafo::IngressoStrutture;
use mzlab_feat::chem::strutture::strutture;

const SMILES: &str = "CCOC(=O)c1c[nH]c2ccccc12";

fn nodo(mz: f64, picchi: Vec<(f64, f64)>) -> IngressoStrutture {
    IngressoStrutture {
        nodo: 3,
        mz,
        polarita: 1,
        picchi,
        formule: vec![("C9H8NO2".into(), 162.05495)],
        radice: false,
    }
}

#[test]
fn the_piece_of_the_cleavage_explains_the_node_and_its_loss_peaks() {
    // 144.04439 = the acid piece without H2O; 190.05 is not a piece of this structure
    let ing = nodo(162.055, vec![(144.0444, 40.0), (190.05, 100.0)]);
    let s = strutture(SMILES, &ing, 10.0).unwrap();
    assert_eq!((s.atomi, s.legami, s.candidati.len()), (14, 4, 1));
    let c = &s.candidati[0];
    assert_eq!(c.formula.as_deref(), Some("C9H8NO2"));
    let t = &c.tagli[0];
    assert_eq!(
        (t.etichetta.as_str(), t.variante.as_str()),
        ("C2-O3", "ion")
    );
    assert_eq!(t.picchi.len(), 1);
    let p = &t.picchi[0];
    assert_eq!((p.mz, p.rel, p.perdita.as_str()), (144.0444, 40.0, "-H2O"));
    assert!(p.ppm.abs() < 1.0);
}

#[test]
fn the_same_input_gives_the_same_answer() {
    let ing = nodo(162.055, vec![(144.0444, 40.0)]);
    assert_eq!(
        strutture(SMILES, &ing, 10.0).unwrap(),
        strutture(SMILES, &ing, 10.0).unwrap()
    );
}

#[test]
fn a_mixture_is_not_cleaved_and_says_so_and_a_bad_smiles_is_an_error() {
    let ing = nodo(162.055, vec![]);
    let s = strutture("CCOC(=O)c1c[nH]c2ccccc12.Cl", &ing, 10.0).unwrap();
    assert!(s.candidati.is_empty());
    assert_eq!(s.avvisi, ["warn.msngrafo.structMixture"]);
    assert!(strutture("C(C", &ing, 10.0).is_err());
}

#[test]
fn a_negative_node_uses_the_negative_ion_form() {
    // [M-H]- of the acid piece: C9H7NO2 - H+ = 160.0404
    let mut ing = nodo(160.04043, vec![]);
    ing.polarita = -1;
    let s = strutture(SMILES, &ing, 10.0).unwrap();
    assert_eq!(s.candidati.len(), 1);
    assert!(s.candidati[0].ppm.abs() < 1.0);
}
