//! `molecules/1`: the CSV and canonical JSON round trips, one error per wrong row, the search (mass in ppm, formula, text), the levels, the catalogue (written
//! back to the same bytes) and the parity of the search with the name box of Tabui (`tests/data/molecole_tabui_oro.json`, made by `tools/molecole_oro_tabui.mjs`).
//! The molecules of the tests are invented.

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

use mzlab_core::json::{parse, Value};
use mzlab_core::molecules::{
    cerca, digest, from_csv, from_json, route, to_csv, to_json, unisci, Molecola,
};

const LOOSE: &str = "\
name;molecular_formula;inchikey;synonyms;charge;prop:rt[min];prop:rt:source;prop:logp[1];prop:logp:source
Esempina;c10h15no;ABCDEFGHIJKLMN-OPQRSTUVWX-Y;Esempina HCl|esempina|Ex-1;;4.25;method A;1.8;calculated
\"Provolone B\";C12H20N2O3;;\"B, Provolone\";;;;;
Cationina;C8H12N;AAAAAAAAAAAAAA-BBBBBBBBBB-C;;1;;;;
";

fn ms(csv: &str) -> Vec<Molecola> {
    from_csv(csv, "utente", None).unwrap_or_else(|e| panic!("{e:?}"))
}

fn sorted_by_id(mut v: Vec<Molecola>) -> Vec<Molecola> {
    v.sort_by(|a, b| a.id.cmp(&b.id));
    v
}

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn catalogue() -> (String, Vec<Molecola>) {
    let text = std::fs::read_to_string(root().join("mzlab/web/molecole/catalogo.csv")).unwrap();
    let all =
        from_csv(&text, "catalogo", None).unwrap_or_else(|e| panic!("{:?}", &e[..e.len().min(3)]));
    (text, all)
}

#[test]
fn csv_json_csv_is_the_same_bytes() {
    let canonical = to_csv(&sorted_by_id(ms(LOOSE)));
    let first = ms(&canonical);
    assert_eq!(to_csv(&first), canonical);
    let json = to_json(&first);
    let back = from_json(json.as_bytes()).unwrap();
    assert_eq!(to_json(&back), json);
    assert_eq!(to_csv(&back), canonical);
    assert_eq!(digest(&back), digest(&first));
    // the ids: the InChIKey when there is one, else 12 hex of the SHA-256 of the record at birth, and they never change
    let ids: Vec<&str> = back.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids[0], "ik:AAAAAAAAAAAAAA-BBBBBBBBBB-C");
    assert!(ids[2].starts_with("u:") && ids[2].len() == 14, "{ids:?}");
    let mut renamed = back.clone();
    renamed[2].nome = "Provolone C".into();
    assert_eq!(
        from_json(to_json(&renamed).as_bytes()).unwrap()[2].id,
        ids[2]
    );
}

#[test]
fn mixed_columns_give_the_same_json() {
    let a = to_json(&ms(LOOSE));
    let shuffled = "\
inchikey;prop:logp:source;name;prop:rt:source;charge;synonyms;prop:logp[1];molecular_formula;prop:rt[min]
ABCDEFGHIJKLMN-OPQRSTUVWX-Y;calculated;Esempina;method A;;esempina|Ex-1|Esempina HCl;1.8;C10H15NO;4.25
;;Provolone B;;;B, Provolone;;C12H20N2O3;
AAAAAAAAAAAAAA-BBBBBBBBBB-C;;Cationina;;1;;;C8H12N;
";
    assert_eq!(to_json(&ms(shuffled)), a);
    // the other delimiters, comments and empty lines
    let tabs = "# list\n\nname\tmolecular_formula\nEsempina\tC10H15NO\n";
    assert_eq!(ms(tabs)[0].formula, "C10H15NO");
}

#[test]
fn mass_is_computed_with_the_electron() {
    let v = ms(LOOSE);
    let (neutral, cation) = (&v[0], &v[2]);
    assert!(
        (neutral.massa - 165.115364).abs() < 1e-5,
        "{}",
        neutral.massa
    );
    // C8H12N: 12 C, 12 H, 1 N, less one electron
    let m = 8.0 * 12.0 + 12.0 * 1.00782503223 + 14.00307400443 - 0.000548579909;
    assert!((cation.massa - m).abs() < 1e-6);
}

#[test]
fn one_error_for_every_wrong_row() {
    let bad = "\
name;molecular_formula;monoisotopic_mass;inchikey;id;charge;prop:rt[min];prop:rt:source
;C2H6O;;;;;;
Ok one;C2H6O;;;;;;
Formula;Xx9;;;;;;
Mass;C2H6O;47.5;;;;;
Key;C2H6O;;not-a-key;;;;
Same id;C2H6O;;;u:0123456789ab;;;
Same id too;C3H8O;;;u:0123456789ab;;;
Charge;C2H6O;;;;1.5;;
Source;C2H6O;;;;;3.5;
";
    let errs = from_csv(bad, "utente", None).unwrap_err();
    let got: Vec<(usize, &str)> = errs.iter().map(|e| (e.line, e.key)).collect();
    assert_eq!(
        got,
        vec![
            (2, "error.molecules.noName"),
            (4, "error.molecules.badFormula"),
            (5, "error.molecules.massMismatch"),
            (6, "error.molecules.badInchikey"),
            (8, "error.molecules.dupId"),
            (9, "error.molecules.badCharge"),
            (10, "error.molecules.badProperty"),
        ]
    );
    let json = route("valida", &format!("{{\"csv\":{}}}", quoted(bad))).unwrap();
    assert!(json.contains("error.molecules.noName"));
    assert_eq!(
        from_csv("name;nope\nx;y\n", "utente", None).unwrap_err()[0].key,
        "error.molecules.badColumn"
    );
    assert_eq!(
        from_csv("name\nx\n", "utente", None).unwrap_err()[0].key,
        "error.molecules.noColumn"
    );
}

fn quoted(s: &str) -> String {
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    )
}

const SEARCH: &str = "\
name;synonyms;molecular_formula;inchikey
Esempina;Ex-1|Provola;C10H15NO;AAAAAAAAAAAAAA-AAAAAAAAAA-A
Esempinolo;;C10H15NO2;AAAAAAAAAAAAAA-AAAAAAAAAA-B
Alfa Esempina;;C11H17NO;AAAAAAAAAAAAAA-AAAAAAAAAA-C
Anti-esempina;;C12H19NO;AAAAAAAAAAAAAA-AAAAAAAAAA-D
Paresempina;;C13H21NO;AAAAAAAAAAAAAA-AAAAAAAAAA-E
Lattosio;;C12H22O11;AAAAAAAAAAAAAA-AAAAAAAAAA-F
";

fn names(ms: &[Molecola], hits: &[(usize, f64)]) -> Vec<String> {
    hits.iter().map(|h| ms[h.0].nome.clone()).collect()
}

#[test]
fn search_by_text_formula_and_mass() {
    let v = ms(SEARCH);
    // equal, starts with, starts a word (after a space or a hyphen), contains
    let t = cerca(&v, "esempina", 8, 10.0);
    assert_eq!(
        names(&v, &t),
        ["Esempina", "Alfa Esempina", "Anti-esempina", "Paresempina"]
    );
    assert_eq!(
        t.iter().map(|h| h.1).collect::<Vec<_>>(),
        [0.0, 2.0, 2.0, 3.0]
    );
    // a synonym counts, a prefix of "Esempina" also matches "Esempinolo" (same score, the shorter name first)
    assert_eq!(names(&v, &cerca(&v, "ex-1", 8, 10.0)), ["Esempina"]);
    assert_eq!(
        names(&v, &cerca(&v, "esempin", 2, 10.0)),
        ["Esempina", "Esempinolo"]
    );
    assert!(cerca(&v, "e", 8, 10.0).is_empty());
    // a formula (any order of writing) comes first
    assert_eq!(names(&v, &cerca(&v, "c12h22o11", 8, 10.0)), ["Lattosio"]);
    assert_eq!(
        names(&v, &cerca(&v, "H19NOC12", 8, 10.0)),
        ["Anti-esempina"]
    );
    // the mass: 9.99 ppm in, 10.01 ppm out, nearest first
    let m = v[0].massa;
    assert_eq!(
        cerca(&v, &format!("{}", m * (1.0 + 9.99e-6)), 8, 10.0).len(),
        1
    );
    assert!(cerca(&v, &format!("{}", m * (1.0 + 10.01e-6)), 8, 10.0).is_empty());
    assert_eq!(
        cerca(&v, &format!("{}", m * (1.0 + 10.01e-6)), 8, 20.0)[0].0,
        0
    );
}

#[test]
fn levels_replace_by_id_and_come_back() {
    let cat = from_csv(SEARCH, "catalogo", None).unwrap();
    let mut mine = cat[1].clone();
    mine.nome = "Esempinolo mio".into();
    mine.origine.tipo = "utente".into();
    let all = unisci(&cat, &[mine.clone()]);
    assert_eq!(all.len(), cat.len());
    assert_eq!(all[1].nome, "Esempinolo mio");
    // an imported copy does not beat the user one, in any order
    let mut imported = mine.clone();
    imported.nome = "Esempinolo importato".into();
    imported.origine.tipo = "importata".into();
    assert_eq!(
        unisci(&cat, &[mine.clone(), imported.clone()])[1].nome,
        "Esempinolo mio"
    );
    assert_eq!(unisci(&cat, &[imported, mine])[1].nome, "Esempinolo mio");
    // taking the copy away brings the catalogue record back
    assert_eq!(unisci(&cat, &[])[1].nome, "Esempinolo");
    // the same through the bindings
    let doc = route(
        "a_json",
        "{\"molecole\":[{\"nome\":\"Nuova\",\"formula\":\"CH4\"}],\"schema\":\"molecules/1\"}",
    )
    .unwrap();
    let loaded = route(
        "carica",
        &format!(
            "{{\"catalogo\":{},\"utente\":{}}}",
            quoted(SEARCH),
            quoted(&doc)
        ),
    )
    .unwrap();
    assert!(
        loaded.contains("\"totale\":7") && loaded.contains("\"altri\":1"),
        "{loaded}"
    );
    assert!(route("cerca", "{\"q\":\"nuova\"}")
        .unwrap()
        .contains("\"nome\":\"Nuova\""));
    let id = route("cerca", "{\"q\":\"lattosio\"}").unwrap();
    let id = parse(id.as_bytes())
        .unwrap()
        .at(0)
        .and_then(|h| h.get("id"))
        .and_then(Value::str)
        .unwrap()
        .to_string();
    assert!(route("get", &format!("{{\"id\":\"{id}\"}}"))
        .unwrap()
        .contains("\"formula\":\"C12H22O11\""));
    assert_eq!(route("get", "{\"id\":\"ik:none\"}").unwrap(), "null");
    route("carica", "{\"utente\":null}").unwrap();
    assert!(route("cerca", "{\"q\":\"nuova\"}").unwrap() == "[]");
}

#[test]
fn the_catalogue_is_canonical_and_complete() {
    let (text, all) = catalogue();
    // the last record (timolol) was added after the two old lists
    assert_eq!(all.len(), 625 + 645 - 43 + 1);
    assert_eq!(to_csv(&all), text, "mzlab/web/molecole/catalogo.csv is not in the canonical form: run tools/genera_molecole.py");
    let both = all.iter().filter(|m| m.origine.catalogo.len() == 2).count();
    assert_eq!(both, 43 + 1);
    assert_eq!(
        all.iter()
            .filter(|m| m.origine.catalogo.contains(&"tabui".to_string()))
            .count(),
        625 + 1
    );
}

/// The queries of `tools/molecole_oro_tabui.mjs`: prefixes of 2, 3 and 5 characters of every name of Tabui, then 100 inner pieces.
fn queries(tabui: &[&Molecola]) -> Vec<String> {
    let mut names: Vec<&str> = Vec::new();
    for m in tabui {
        names.push(&m.nome);
        if m.origine.catalogo.len() == 1 {
            names.extend(m.sinonimi.iter().map(String::as_str));
        }
    }
    let mut out: Vec<String> = Vec::new();
    for n in &names {
        for k in [2, 3, 5] {
            let c: Vec<char> = n.chars().collect();
            let p: String = c.iter().take(k).collect();
            if c.len() >= k && !out.contains(&p) {
                out.push(p);
            }
        }
    }
    for i in 0..100usize {
        let mut j = (i * 37 + 11) % names.len();
        while names[j].chars().count() < 8 {
            j = (j + 1) % names.len();
        }
        let c: Vec<char> = names[j].chars().collect();
        let (len, start) = (3 + i % 3, 1 + (i * 7) % (c.len() - (3 + i % 3) - 1));
        out.push(c[start..start + len].iter().collect());
    }
    out
}

#[test]
fn search_gives_what_the_name_box_of_tabui_gave() {
    let (_, all) = catalogue();
    let mut tabui: Vec<Molecola> = all
        .into_iter()
        .filter(|m| m.origine.catalogo.contains(&"tabui".to_string()))
        .collect();
    // the gold data come from the 625 compounds of the old list; timolol came after
    tabui.truncate(625);
    let common: Vec<usize> = (0..tabui.len())
        .filter(|&k| tabui[k].origine.catalogo.len() == 2)
        .collect();
    let oro = std::fs::read_to_string(root().join("tests/data/molecole_tabui_oro.json")).unwrap();
    let oro = parse(oro.as_bytes()).unwrap();
    let expected = oro.get("risultati").unwrap().arr();
    let qs = queries(&tabui.iter().collect::<Vec<_>>());
    assert_eq!(qs.len(), expected.len());
    assert_eq!(
        oro.get("righe").and_then(Value::f64),
        Some(tabui.len() as f64)
    );
    let (mut diff, mut numeric, mut extra) = (Vec::new(), 0, 0);
    for (q, e) in qs.iter().zip(expected) {
        // a number is a mass now: the old box searched it as text (a CAS number begun)
        if q.parse::<f64>().is_ok() {
            numeric += 1;
            continue;
        }
        let want: Vec<usize> = e
            .arr()
            .iter()
            .filter_map(|x| x.f64())
            .map(|x| x as usize)
            .collect();
        let whole: Vec<usize> = cerca(&tabui, q, usize::MAX, 10.0)
            .iter()
            .map(|h| h.0)
            .collect();
        // the 43 molecules of the two lists carry the names of the second one too: they may be found (or ranked) by those, but nothing else changes
        let (old_rest, new_rest): (Vec<usize>, Vec<usize>) = (
            want.iter()
                .copied()
                .filter(|k| !common.contains(k))
                .collect(),
            whole
                .iter()
                .copied()
                .filter(|k| !common.contains(k))
                .collect(),
        );
        let lost: Vec<&usize> = want
            .iter()
            .filter(|k| common.contains(k) && !whole.contains(k))
            .collect();
        extra += whole
            .iter()
            .filter(|k| common.contains(k) && !want.contains(k))
            .count();
        // without a molecule of the two lists in the way, the box gave these 8 and so does the search
        if !whole.iter().any(|k| common.contains(k)) {
            let top: Vec<usize> = cerca(&tabui, q, 8, 10.0).iter().map(|h| h.0).collect();
            assert_eq!(top, want[..want.len().min(8)], "{q:?}");
        }
        if old_rest != new_rest || !lost.is_empty() {
            diff.push(format!(
                "{q:?}: {} -> {} hits, lost {lost:?}",
                want.len(),
                whole.len()
            ));
        }
    }
    println!(
        "{} queries, {numeric} numeric, {extra} extra hits of the common molecules",
        qs.len()
    );
    assert!(
        diff.is_empty(),
        "{} of {} queries differ:\n{}",
        diff.len(),
        qs.len(),
        diff.iter().take(20).cloned().collect::<Vec<_>>().join("\n")
    );
}
