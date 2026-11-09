//! The rules that read the structure and the rules that follow another, on the synthetic graph of `tests`: the structure side is replaced by the
//! `attese` it would compute, so these tests need no SMILES.

use super::rules::{read_table, Table};
use super::rules_evaluate::evaluate_with;
use super::tests::{find, shifted_graph};
use super::*;

/// A rule of a test table: `kind`, `formula`, the required groups (JSON list items) and the ids it follows.
fn regola(id: &str, kind: &str, formula: &str, groups: &str, after: &str) -> String {
    format!(
        "{{\"id\":\"{id}\",\"kind\":\"{kind}\",\"formula\":\"{formula}\",\"radical\":false,\
         \"requires\":{{\"all_groups\":[{groups}],\"any_groups\":[],\"site\":null,\"smarts\":null}},\
         \"excludes\":[],\"ions\":[],\"polarity\":\"+\",\"activation\":\"both\",\
         \"energy\":\"low\",\"charge\":\"any\",\"priority\":1,\"after\":[{after}],\"refs\":[],\"why\":\"test\"}}"
    )
}

fn tavola(rules: &[String]) -> &'static Table {
    let t = format!(
        "{{\"format\":\"fragrules/1\",\"rules\":[{}],\"globals\":[]}}",
        rules.join(",")
    );
    Box::leak(Box::new(read_table(&t).unwrap()))
}

fn con(attese: Vec<Expectation>) -> Options {
    Options {
        formula: Some("C8H10N4O2+H".into()),
        regole: Some(RuleParams::default()),
        attese,
        ..Options::default()
    }
}

fn expectation(regola: &str, formula: &str, ione: bool) -> Expectation {
    Expectation {
        rule: regola.into(),
        formula: formula.into(),
        ione,
    }
}

/// The ids of the rules that explain the arc `da` -> `a`.
fn leads(t: &'static Table, o: &Options, da: &str, a: &str) -> Vec<(String, String)> {
    let (g, _) = shifted_graph(o, &[0, 1, 2, 3], false, 0.0);
    let v = evaluate_with(t, &g, o, &RuleParams::default());
    let (p, f) = (find(&g, da), find(&g, a));
    let i = g.archi.iter().position(|x| x.da == p && x.a == f).unwrap();
    v.edges[i]
        .rules
        .iter()
        .map(|r| (r.id.to_string(), r.formula.clone()))
        .collect()
}

#[test]
fn a_rule_that_reads_the_structure_explains_an_arc_only_with_its_expected_product() {
    // the formula of the table is not the one that holds: the structure gave CHN
    let t = tavola(&[regola("s", "loss", "CH4", "\"amine\"", "")]);
    assert!(leads(t, &con(vec![]), "C5H8N3", "C4H7N2").is_empty());
    let o = con(vec![expectation("s", "CHN", false)]);
    assert_eq!(
        leads(t, &o, "C5H8N3", "C4H7N2"),
        [("s".to_string(), "CHN".to_string())]
    );
    // an expectation of another rule or of an unknown one explains nothing
    let altro = con(vec![expectation("x", "CHN", false)]);
    assert!(leads(t, &altro, "C5H8N3", "C4H7N2").is_empty());
}

#[test]
fn an_ion_is_not_a_loss() {
    let t = tavola(&[regola("s", "loss", "CH4", "\"amine\"", "")]);
    let o = con(vec![expectation("s", "CHN", true)]);
    assert!(leads(t, &o, "C5H8N3", "C4H7N2").is_empty());
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
        leads(t, &o, "C5H8N3", "C4H7N2")
            .into_iter()
            .map(|x| x.0)
            .collect::<Vec<_>>(),
        ["hcn_after_co"]
    );
    let only = tavola(&[regola("hcn_after_co", "loss", "CHN", "", "\"co\"")]);
    assert!(leads(only, &o, "C5H8N3", "C4H7N2").is_empty());
    let other = tavola(&[
        regola("nh3", "loss", "NH3", "", ""),
        regola("hcn_after_co", "loss", "CHN", "", "\"nh3\""),
    ]);
    assert!(leads(other, &o, "C5H8N3", "C4H7N2").is_empty());
}

#[test]
fn an_expected_ion_is_found_on_the_node_of_its_mass() {
    let t = tavola(&[regola("ion_s", "ion", "CH4N", "\"amine\"", "")]);
    let o = con(vec![expectation("ion_s", "C5H8N3", true)]);
    let (g, _) = shifted_graph(&o, &[0, 1, 2, 3], false, 0.0);
    let v = evaluate_with(t, &g, &o, &RuleParams::default());
    assert_eq!(v.ions.len(), 1);
    assert_eq!(v.ions[0].nodo, find(&g, "C5H8N3"));
    assert!(evaluate_with(t, &g, &con(vec![]), &RuleParams::default())
        .ions
        .is_empty());
}
