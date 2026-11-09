use super::candidati::elementi;
use super::formule::addotto;
use super::regole::{read_table, table, Table};
use super::regole_valuta::{evaluate_with, loss_vectors, radicale};
use super::tests::{find, shifted_graph};
use super::*;
use crate::elements::parse_formula;

/// One loss rule of a test table (formula only, no references).
fn rule(id: &str, formula: &str, ions: &str, activation: &str) -> String {
    format!(
        "{{\"id\":\"{id}\",\"kind\":\"loss\",\"formula\":\"{formula}\",\"radical\":false,\
         \"requires\":{{\"all_groups\":[],\"any_groups\":[],\"site\":null,\"smarts\":null}},\
         \"excludes\":[],\"ions\":[{ions}],\"polarity\":\"+\",\"activation\":\"{activation}\",\
         \"energy\":\"low\",\"charge\":\"any\",\"priority\":1,\"after\":[],\"refs\":[],\"why\":\"test\"}}"
    )
}

fn tavola(rules: &[String]) -> &'static Table {
    let t = format!(
        "{{\"format\":\"fragrules/1\",\"rules\":[{}],\"globals\":[]}}",
        rules.join(",")
    );
    Box::leak(Box::new(read_table(&t).unwrap()))
}

fn con(att: Option<&str>) -> Options {
    Options {
        formula: Some("C8H10N4O2+H".into()),
        activation: att.map(String::from),
        regole: Some(RuleParams::default()),
        ..Options::default()
    }
}

#[test]
fn the_embedded_table_is_read_with_recomputed_masses() {
    let t = table();
    assert_eq!(t.rules.len(), 43);
    assert_eq!(t.rules.iter().filter(|r| r.solo_formula()).count(), 5);
    let nh3 = t.rules.iter().find(|r| r.id == "loss_nh3_ammonium_adduct");
    assert!((nh3.unwrap().massa - 17.02655).abs() < 1e-5);
    assert_eq!(
        (t.globali.trap_cutoff_q, t.globali.loss_tolerance_mda),
        (0.25, 3.0)
    );
    assert!(t.globali.even_electron && t.globali.beam_composite && t.globali.stepped_sum);
}

#[test]
fn a_key_outside_the_schema_or_a_bad_formula_is_an_error() {
    let ok = rule("a", "H2O", "", "both");
    let t = |r: &str| format!("{{\"format\":\"fragrules/1\",\"rules\":[{r}],\"globals\":[]}}");
    assert!(read_table(&t(&ok)).is_ok());
    let extra = ok.replace("\"why\"", "\"mass\":18,\"why\"");
    assert_eq!(
        read_table(&t(&extra)).err().unwrap().key,
        "err.fragrules.badKey"
    );
    let bad = ok.replace("H2O", "Zz9");
    assert_eq!(
        read_table(&t(&bad)).err().unwrap().key,
        "err.fragrules.badFormat"
    );
    let doppia = format!("{ok},{ok}");
    assert!(read_table(&t(&doppia)).is_err());
    assert!(read_table("{\"format\":\"x/1\",\"rules\":[],\"globals\":[]}").is_err());
}

#[test]
fn the_adduct_comes_from_the_typed_ion() {
    assert_eq!(addotto("C8H11N4O2+", 1), Some("[M+H]+"));
    assert_eq!(addotto("C8H10N4O2+H", 1), Some("[M+H]+"));
    assert_eq!(addotto("C8H10N4O2 [M+Na]+", 1), Some("[M+Na]+"));
    assert_eq!(addotto("C8H11N4O2 [M]+", 1), None);
    assert_eq!(addotto("C8H9N4O2-", -1), Some("[M-H]-"));
}

#[test]
fn a_radical_loss_has_an_odd_number_of_electrons() {
    for (f, r) in [("CH3", true), ("OH", true), ("CH4", false), ("H2O", false)] {
        assert_eq!(radicale(f), r, "{f}");
    }
}

#[test]
fn a_rule_explains_the_loss_of_an_arc_by_formula_and_adduct() {
    let t = tavola(&[
        rule("co", "CO", "\"[M+H]+\"", "both"),
        rule("hcn", "CHN", "", "both"),
        rule("na", "CO2", "\"[M+Na]+\"", "both"),
    ]);
    let o = con(None);
    let (g, _) = shifted_graph(&o, &[0, 1, 2, 3], false, 0.0);
    let v = evaluate_with(t, &g, &o, &RuleParams::default());
    assert_eq!(v.addotto, Some("[M+H]+"));
    let leads = |da: &str, a: &str| {
        let (p, f) = (find(&g, da), find(&g, a));
        let i = g.archi.iter().position(|x| x.da == p && x.a == f).unwrap();
        v.edges[i].rules.iter().map(|r| r.id).collect::<Vec<_>>()
    };
    // CO leaves a fragment of the m/z 138, so the adduct of the root plays no part: the rule asks for it only on the root
    assert!(leads("C6H8N3O", "C5H8N3").is_empty());
    assert_eq!(leads("C5H8N3", "C4H7N2"), ["hcn"]);
    assert_eq!(v.explained + v.senza_regola, g.archi.len());
    assert!(v.intensita_spiegata > 0.0 && v.intensita_spiegata < 1.0);
    let e = v.edges.iter().flat_map(|a| &a.rules).next().unwrap();
    assert!(e.mda_error.abs() < 1.0 && !e.radical && e.perche == "test");
}

#[test]
fn a_loss_that_is_the_sum_of_two_rules_is_composite_only_in_a_beam() {
    let t = tavola(&[rule("co", "CO", "", "both"), rule("hcn", "CHN", "", "both")]);
    // C6H8N3O -> C4H7N2 is C2HNO = CO + HCN
    let fra = |att: Option<&str>| {
        let o = con(att);
        let (g, _) = shifted_graph(&o, &[0, 1, 2, 3], false, 0.0);
        let v = evaluate_with(t, &g, &o, &RuleParams::default());
        let (p, f) = (find(&g, "C6H8N3O"), find(&g, "C4H7N2"));
        let i = g.archi.iter().position(|x| x.da == p && x.a == f).unwrap();
        v.edges[i].composta.clone()
    };
    assert_eq!(fra(None), [("co", "hcn"), ("hcn", "co")]);
    assert!(fra(Some("cid")).is_empty());
}

#[test]
fn the_trap_cutoff_hides_what_is_below_it_and_follows_q() {
    // a loss of 138 leaves 57: above 0.25 / 0.908 of 195.09 (53.7), below 0.5 / 0.908 (107.4)
    let t = tavola(&[rule("big", "C6H6N2O2", "", "both")]);
    let o = con(Some("cid"));
    let (g, _) = shifted_graph(&o, &[0, 1, 2, 3], false, 0.0);
    let con_q = |q: f64| {
        let p = RuleParams {
            cutoff_q: q,
            ..RuleParams::default()
        };
        evaluate_with(t, &g, &o, &p).non_osservabili
    };
    let root = g.radici[0];
    let della_radice = |q: f64| {
        let v = con_q(q);
        v.into_iter().filter(|x| x.node == root).collect::<Vec<_>>()
    };
    assert!(della_radice(0.25).is_empty());
    let n = della_radice(0.5);
    assert_eq!(n.len(), 1);
    assert_eq!(n[0].rule, "big");
    assert!((n[0].threshold - 0.5 / 0.908 * g.nodi[root].mz).abs() < 1e-6);
    // the cut-off belongs to the trap: a beam spectrum has none
    let ob = con(Some("hcd"));
    let (gb, _) = shifted_graph(&ob, &[0, 1, 2, 3], false, 0.0);
    let p = RuleParams {
        cutoff_q: 0.5,
        ..RuleParams::default()
    };
    assert!(evaluate_with(t, &gb, &ob, &p).non_osservabili.is_empty());
}

#[test]
fn the_key_is_added_at_the_end_and_a_graph_without_it_keeps_its_bytes() {
    let o = con(None);
    let (g, j) = shifted_graph(&o, &[0, 1, 2, 3], false, 0.0);
    let k = con_regole(&j, &g, &o, &RuleParams::default());
    assert!(k.starts_with(j.split(",\"vista\":").next().unwrap()));
    assert!(k.ends_with(&j[j.find(",\"vista\":").unwrap()..]));
    let v = crate::json::parse(k.as_bytes()).unwrap();
    let r = v.get("regole").unwrap();
    assert_eq!(r.get("tabella").and_then(|x| x.str()), Some("fragrules/1"));
    assert_eq!(r.get("archi").unwrap().arr().len(), g.archi.len());
    // the thresholds say where their value comes from and that it is still to be calibrated
    let thresholds = r.get("soglie").unwrap().arr();
    assert!(thresholds
        .iter()
        .any(|s| s.get("id").and_then(|x| x.str()) == Some("trap_cutoff")));
    assert!(thresholds.iter().all(
        |s| s.get("da_calibrare").and_then(|x| x.bool()) == Some(true)
            && s.get("origine").and_then(|x| x.str()).is_some()
    ));
    // every reference cited by a rule of an arc has its source
    let fonti: Vec<&str> = r
        .get("fonti")
        .unwrap()
        .arr()
        .iter()
        .filter_map(|f| f.get("key").and_then(|x| x.str()))
        .collect();
    for a in r.get("archi").unwrap().arr() {
        for x in a.get("regole").unwrap().arr() {
            assert!(x.get("perche").and_then(|p| p.str()).is_some());
            for k in x.get("refs").unwrap().arr() {
                assert!(fonti.contains(&k.str().unwrap()));
            }
        }
    }
    let without = Options {
        regole: None,
        ..o.clone()
    };
    let (g2, j2) = shifted_graph(&without, &[0, 1, 2, 3], false, 0.0);
    assert_eq!((g2, j2), (g, j));
}

#[test]
fn the_loss_vectors_of_the_rules_hold_only_from_the_adduct_of_the_root() {
    let f = parse_formula("C8H15N5O2").unwrap();
    let e = elementi(&f, 1);
    let nh3 = e.conta(&parse_formula("NH3").unwrap()).unwrap();
    let da_radice = loss_vectors(&e, 1, Some("[M+NH4]+"), true);
    assert!(da_radice.contains(&nh3));
    assert!(loss_vectors(&e, 1, Some("[M+NH4]+"), false).is_empty());
    assert!(loss_vectors(&e, 1, Some("[M+H]+"), true).is_empty());
}

#[test]
fn the_request_reads_the_rules_and_their_numbers() {
    let q = |s: &'static str| {
        let q = crate::routes::Query::parse(s);
        da_richiesta(&|k| q.get(k).map(String::from))
    };
    assert_eq!(q("").unwrap().0.regole, None);
    let p = q("regole=1&cutoff_q=0.4&toll_perdita_mda=5")
        .unwrap()
        .0
        .regole;
    assert_eq!(
        p,
        Some(RuleParams {
            cutoff_q: 0.4,
            toll_mda: 5.0
        })
    );
    assert_eq!(q("regole=1").unwrap().0.regole, Some(RuleParams::default()));
    assert!(q("regole=1&cutoff_q=x").is_err());
}

#[test]
fn a_single_spectrum_is_a_graph_of_one_level_with_the_precursor_as_root() {
    let prec = super::tests::ion("C8H11N4O2");
    let fragment = super::tests::ion("C6H8N3O");
    let o = Options {
        formula: Some("C8H10N4O2+H".into()),
        ..Options::default()
    };
    let g = graph_from_peaks(
        (prec, 1),
        ("HCD", 30.0),
        &[(fragment, 800.0), (prec, 200.0)],
        &o,
    );
    assert_eq!(g.radici.len(), 1);
    assert_eq!(g.archi.len(), 1);
    assert_eq!(g.spettri.len(), 1);
    let s = &g.spettri[0];
    assert_eq!(s.precursore, Some(g.radici[0]));
    // the base peak is 100 %, the peak of the precursor is a node of its own (the root)
    assert!(s
        .picchi
        .iter()
        .any(|p| (p.1 - 100.0).abs() < 1e-9 && p.2.is_some()));
    assert!(g.nodi[g.archi[0].a].formula.is_some());
}
