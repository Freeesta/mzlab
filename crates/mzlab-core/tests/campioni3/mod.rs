//! `campioni/3`: migration of the `/2`, canonical bytes under permutations, `valida` (columns, keys, pairs), CSV both ways, the name model
//! (regex and template, spans, the generated names) and `dal_file`; schema check.

mod csv;
mod modello;

use mzlab_core::campioni::{
    self, from_json, per_tp, tabella_da_json, to_json, to_json3, valida, Cell, Colonna, Tabella,
    TipoColonna,
};
use mzlab_core::campioni::{proponi, Campione};
use serde_json::Value;

pub fn many(names: &[&str]) -> Vec<Campione> {
    let v: Vec<(String, String, String)> = names
        .iter()
        .map(|n| (n.to_string(), String::new(), String::new()))
        .collect();
    proponi(&v)
}

fn codes(t: &Tabella) -> Vec<String> {
    let j = campioni::route("valida", &to_json3(t)).unwrap();
    let v: Value = serde_json::from_str(&j).unwrap();
    v["avvisi"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| match a["colonna"].as_str() {
            Some(c) => format!("{}:{c}", a["codice"].as_str().unwrap()),
            None => a["codice"].as_str().unwrap().to_string(),
        })
        .collect()
}

fn table(names: &[&str]) -> Tabella {
    Tabella {
        campioni: many(names),
        colonne: Vec::new(),
    }
}

#[test]
fn a_campioni_2_migrates_with_the_same_per_tp() {
    let cs = many(&[
        "X_t000min",
        "X_t005min",
        "X_t010min",
        "bianco_solv",
        "X_dark",
        "std_5ppm",
    ]);
    let v2 = to_json(&cs);
    let t = tabella_da_json(v2.as_bytes()).unwrap();
    assert_eq!(t.campioni, cs);
    assert_eq!(
        per_tp(&t.campioni),
        per_tp(&from_json(v2.as_bytes()).unwrap())
    );
    let v3 = campioni::route("migra", &v2).unwrap();
    assert!(v3.contains("\"schema\":\"campioni/3\""));
    let back = tabella_da_json(v3.as_bytes()).unwrap();
    assert_eq!(per_tp(&back.campioni), per_tp(&cs));
    // a /2 asked for its canonical form stays a /2, a /3 stays a /3
    assert_eq!(campioni::route("canonico", &v2).unwrap(), v2);
    assert_eq!(campioni::route("canonico", &v3).unwrap(), v3);
    assert_eq!(campioni::route("digest", &v3).unwrap().len(), 64);
}

fn rich() -> Tabella {
    let mut t = table(&["A1_pos_t005", "A1_neg_t005", "B2_pos_t005", "B2_neg_t005"]);
    for (i, c) in t.campioni.iter_mut().enumerate() {
        c.sample_id = ["A1", "A1", "B2", "B2"][i].into();
        c.polarity = Some(if i % 2 == 0 { 1 } else { -1 });
        c.matrix = "water".into();
        c.batch = "2".into();
        c.group = "g".into();
        c.series = "S".into();
        c.acquired = Some(format!("2025-03-0{}T10:00:00", i + 1));
        c.set_extra("zeta", Cell::Numero(i as f64 + 0.5));
        c.set_extra("alfa", Cell::Testo(format!("v{i}")));
    }
    t.colonne = vec![
        Colonna::new("zeta", TipoColonna::Numero),
        Colonna {
            name: "alfa".into(),
            kind: TipoColonna::Categoria,
            values: Some(vec!["v3".into(), "v0".into(), "v1".into(), "v2".into()]),
            required: true,
        },
    ];
    t
}

fn permutations(n: usize) -> Vec<Vec<usize>> {
    if n == 1 {
        return vec![vec![0]];
    }
    let mut out = Vec::new();
    for p in permutations(n - 1) {
        for k in 0..=p.len() {
            let mut q = p.clone();
            q.insert(k, n - 1);
            out.push(q);
        }
    }
    out
}

#[test]
fn canonical_bytes_do_not_depend_on_the_order_written() {
    let t = rich();
    let want = to_json3(&t);
    let schema: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/schemi/campioni-3.json"
        ))
        .unwrap(),
    )
    .unwrap();
    assert!(super::fits(&serde_json::from_str(&want).unwrap(), &schema));
    assert_eq!(tabella_da_json(want.as_bytes()).unwrap().campioni.len(), 4);
    for p in permutations(4) {
        for q in [vec![0, 1], vec![1, 0]] {
            let mut u = t.clone();
            u.campioni = p.iter().map(|&i| t.campioni[i].clone()).collect();
            u.colonne = q.iter().map(|&i| t.colonne[i].clone()).collect();
            u.campioni.iter_mut().for_each(|c| c.extra.reverse());
            assert_eq!(to_json3(&u), want);
        }
    }
    assert_eq!(
        tabella_da_json(want.as_bytes()).unwrap().campioni[3].extra[0].0,
        "alfa"
    );
    assert_eq!(
        campioni::digest3(&t),
        campioni::digest3(&tabella_da_json(want.as_bytes()).unwrap())
    );
}

#[test]
fn valida_checks_columns_keys_and_pairs() {
    let mut t = rich();
    assert!(codes(&t)
        .iter()
        .all(|c| !c.contains(':') && c != "duplicateKey" && !c.starts_with("pair")));
    // a required column without a value, a number in a text column, a value outside the category, a text in a number column
    t.campioni[0].extra.retain(|(k, _)| k != "alfa");
    t.campioni[1].set_extra("alfa", Cell::Numero(1.0));
    t.campioni[2].set_extra("alfa", Cell::Testo("nope".into()));
    t.campioni[3].set_extra("zeta", Cell::Testo("x".into()));
    let c = codes(&t);
    assert_eq!(c.iter().filter(|x| *x == "missingValue:alfa").count(), 1);
    assert_eq!(c.iter().filter(|x| *x == "typeMismatch:alfa").count(), 2);
    assert_eq!(c.iter().filter(|x| *x == "typeMismatch:zeta").count(), 1);
    // keys
    let mut d = rich();
    d.campioni[1].file = d.campioni[0].file.clone();
    assert!(codes(&d).contains(&"duplicateKey".into()));
    let mut s = rich();
    s.campioni[2].sha256 = "ab".into();
    s.campioni[3].sha256 = "ab".into();
    assert!(codes(&s).contains(&"duplicateKey".into()));
    // pairs
    let mut m = rich();
    m.campioni[3].polarity = Some(1);
    assert!(codes(&m).contains(&"pairMissing".into()));
    let mut x = rich();
    x.campioni[1].series = "other".into();
    assert!(codes(&x).contains(&"pairMismatch".into()));
    let mut y = rich();
    y.campioni[3].set_time(Some(30.0), "min");
    assert!(codes(&y).contains(&"pairMismatch".into()));
    let mut z = rich();
    z.campioni[3].kind = "qc".into();
    assert!(codes(&z).contains(&"pairMismatch".into()));
    // a campioni/2 document is checked as before
    let v2 = to_json(&many(&["X_t000min", "X_t005min"]));
    assert_eq!(
        campioni::route("valida", &v2).unwrap(),
        campioni::route("valida", &campioni::route("migra", &v2).unwrap()).unwrap()
    );
    assert!(!valida(&many(&["X_t000min", "X_t005min"])).is_empty());
}

#[test]
fn dal_file_fills_what_the_file_says() {
    let mut t = rich();
    t.campioni[0].sha256 = "aa".into();
    t.campioni[0].polarity = Some(-1); // the name was wrong
    t.campioni[1].polarity = None;
    let input = format!(
        r#"{{"campioni":{},"file":[
        {{"sha256":"aa","polarita":[1,1],"rt_min":0.1,"rt_max":9.9,"mz_min":50,"mz_max":600,"scansioni":420,"acquisito":"2025-04-01 08:30:15.123","modello":"QTRAP 3200","metodo":"m1.dam"}},
        {{"nome":"{}","polarita":[-1],"metodo_json":{{"origine":{{"file_creato":"2025-04-01T09:00:00"}},"strumento":{{"modello":"3200 QTRAP"}},"campione":{{"metodo_file":"neg.dam"}},"ms":{{"periodi":[{{"esperimenti":[{{"mz":[100,900]}},{{"mz":[80,400]}}]}}]}}}}}},
        {{"nome":"unknown.mzML","polarita":[1]}}]}}"#,
        to_json3(&t),
        t.campioni[1].file
    );
    let out: Value =
        serde_json::from_str(&campioni::route_tutto("dal_file", &input).unwrap()).unwrap();
    let c = &out["campioni"]["campioni"];
    let by = |f: &str| {
        c.as_array()
            .unwrap()
            .iter()
            .find(|x| x["file"] == f)
            .unwrap()
    };
    let a = by(&t.campioni[0].file);
    assert_eq!(
        (a["polarita"].clone(), a["acquisito"].as_str()),
        (1.into(), Some("2025-04-01T08:30:15"))
    );
    assert_eq!(a["dati_file"]["scansioni"], 420);
    assert_eq!(a["dati_file"]["mz_max"], 600);
    assert_eq!(a["dati_file"]["modello"], "QTRAP 3200");
    let b = by(&t.campioni[1].file);
    assert_eq!(
        (b["polarita"].clone(), b["acquisito"].as_str()),
        ((-1).into(), Some("2025-04-01T09:00:00"))
    );
    assert_eq!(
        (
            b["dati_file"]["mz_min"].clone(),
            b["dati_file"]["mz_max"].clone()
        ),
        (80.into(), 900.into())
    );
    assert_eq!(b["dati_file"]["metodo"], "neg.dam");
    assert_eq!(out["senza_campione"], serde_json::json!([2]));
    // only the replaced values are reported
    let d = out["discordanze"].as_array().unwrap();
    assert!(d
        .iter()
        .any(|x| x["campo"] == "polarita" && x["prima"] == "-1" && x["dopo"] == "1"));
    assert!(d
        .iter()
        .all(|x| x["indice"] != 1 || x["campo"] == "acquisito"));
    // the dati_file survive the canonical form
    let again = campioni::route(
        "canonico",
        &serde_json::to_string(&out["campioni"]).unwrap(),
    )
    .unwrap();
    assert!(again.contains("\"scansioni\":420"));
}

#[test]
fn errors_are_json_keys_not_panics() {
    for (op, input) in [
        ("csv_leggi", "{}"),
        ("csv_leggi", r#"{"testo":"a,b\n\"x"}"#),
        ("csv_scrivi", "{}"),
        ("modello", "{}"),
        ("modello", r#"{"modello":"(?P<a>","files":[]}"#),
        ("dal_file", "{}"),
        ("canonico", r#"{"schema":"campioni/9"}"#),
    ] {
        assert!(campioni::route_tutto(op, input).is_err(), "{op} {input}");
    }
}
