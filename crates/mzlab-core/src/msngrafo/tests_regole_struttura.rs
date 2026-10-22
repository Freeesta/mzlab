//! The rules that read the structure and the rules that follow another, on the synthetic graph of `tests`: the structure side is replaced by the
//! `attese` it would compute, so these tests need no SMILES.

use super::regole::{leggi_tabella, Tabella};
use super::regole_valuta::valuta_con;
use super::tests::{grafo_spostato, trova};
use super::*;

/// A rule of a test table: `kind`, `formula`, the required groups (JSON list items) and the ids it follows.
fn regola(id: &str, kind: &str, formula: &str, gruppi: &str, after: &str) -> String {
    format!(
        "{{\"id\":\"{id}\",\"kind\":\"{kind}\",\"formula\":\"{formula}\",\"radical\":false,\
         \"requires\":{{\"all_groups\":[{gruppi}],\"any_groups\":[],\"site\":null,\"smarts\":null}},\
         \"excludes\":[],\"ions\":[],\"polarity\":\"+\",\"activation\":\"both\",\
         \"energy\":\"low\",\"charge\":\"any\",\"priority\":1,\"after\":[{after}],\"refs\":[],\"why\":\"test\"}}"
    )
}

fn tavola(regole: &[String]) -> &'static Tabella {
    let t = format!(
        "{{\"format\":\"fragrules/1\",\"rules\":[{}],\"globals\":[]}}",
        regole.join(",")
    );
    Box::leak(Box::new(leggi_tabella(&t).unwrap()))
}

fn con(attese: Vec<Attesa>) -> Opzioni {
    Opzioni {
        formula: Some("C8H10N4O2+H".into()),
        regole: Some(ParamRegole::default()),
        attese,
        ..Opzioni::default()
    }
}

fn attesa(regola: &str, formula: &str, ione: bool) -> Attesa {
    Attesa {
        regola: regola.into(),
        formula: formula.into(),
        ione,
    }
}

/// The ids of the rules that explain the arc `da` -> `a`.
fn porta(t: &'static Tabella, o: &Opzioni, da: &str, a: &str) -> Vec<(String, String)> {
    let (g, _) = grafo_spostato(o, &[0, 1, 2, 3], false, 0.0);
    let v = valuta_con(t, &g, o, &ParamRegole::default());
    let (p, f) = (trova(&g, da), trova(&g, a));
    let i = g.archi.iter().position(|x| x.da == p && x.a == f).unwrap();
    v.archi[i]
        .regole
        .iter()
        .map(|r| (r.id.to_string(), r.formula.clone()))
        .collect()
}

#[test]
fn a_rule_that_reads_the_structure_explains_an_arc_only_with_its_expected_product() {
    // the formula of the table is not the one that holds: the structure gave CHN
    let t = tavola(&[regola("s", "loss", "CH4", "\"amine\"", "")]);
    assert!(porta(t, &con(vec![]), "C5H8N3", "C4H7N2").is_empty());
    let o = con(vec![attesa("s", "CHN", false)]);
    assert_eq!(
        porta(t, &o, "C5H8N3", "C4H7N2"),
        [("s".to_string(), "CHN".to_string())]
    );
    // an expectation of another rule or of an unknown one explains nothing
    let altro = con(vec![attesa("x", "CHN", false)]);
    assert!(porta(t, &altro, "C5H8N3", "C4H7N2").is_empty());
}

#[test]
fn an_ion_is_not_a_loss() {
    let t = tavola(&[regola("s", "loss", "CH4", "\"amine\"", "")]);
    let o = con(vec![attesa("s", "CHN", true)]);
    assert!(porta(t, &o, "C5H8N3", "C4H7N2").is_empty());
}

#[test]
fn a_rule_that_follows_another_holds_only_after_it() {
    let t = tavola(&[
        regola("co", "loss", "CO", "", ""),
        regola("hcn_after_co", "loss", "CHN", "", "\"co\""),
    ]);
    let o = con(vec![]);
    // C6H8N3O -> C5H8N3 is CO, then C5H8N3 -> C4H7N2 is HCN
    assert_eq!(
        porta(t, &o, "C5H8N3", "C4H7N2")
            .into_iter()
            .map(|x| x.0)
            .collect::<Vec<_>>(),
        ["hcn_after_co"]
    );
    let solo = tavola(&[regola("hcn_after_co", "loss", "CHN", "", "\"co\"")]);
    assert!(porta(solo, &o, "C5H8N3", "C4H7N2").is_empty());
    let altra = tavola(&[
        regola("nh3", "loss", "NH3", "", ""),
        regola("hcn_after_co", "loss", "CHN", "", "\"nh3\""),
    ]);
    assert!(porta(altra, &o, "C5H8N3", "C4H7N2").is_empty());
}

#[test]
fn an_expected_ion_is_found_on_the_node_of_its_mass() {
    let t = tavola(&[regola("ion_s", "ion", "CH4N", "\"amine\"", "")]);
    let o = con(vec![attesa("ion_s", "C5H8N3", true)]);
    let (g, _) = grafo_spostato(&o, &[0, 1, 2, 3], false, 0.0);
    let v = valuta_con(t, &g, &o, &ParamRegole::default());
    assert_eq!(v.ioni.len(), 1);
    assert_eq!(v.ioni[0].nodo, trova(&g, "C5H8N3"));
    assert!(valuta_con(t, &g, &con(vec![]), &ParamRegole::default())
        .ioni
        .is_empty());
}
