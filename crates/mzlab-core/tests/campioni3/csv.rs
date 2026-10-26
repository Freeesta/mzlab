//! CSV of `campioni/3`: round trip both ways, the three separators, quotes and newlines, synonyms of the headers, typed free columns, errors.

use super::{many, rich};
use mzlab_core::campioni::{self, csv_leggi, csv_scrivi, Cell, Tabella, TipoColonna};
use mzlab_core::campioni::{tabella_da_json, to_json3};
use serde_json::{json, Value};

fn written(t: &Tabella, sep: &str) -> String {
    let doc = to_json3(t);
    let r: Value = serde_json::from_str(
        &csv_scrivi(
            &json!({"campioni": serde_json::from_str::<Value>(&doc).unwrap(), "separatore": sep})
                .to_string(),
        )
        .unwrap(),
    )
    .unwrap();
    r["testo"].as_str().unwrap().to_string()
}

fn read(text: &str, sep: &str, cols: &Tabella) -> Tabella {
    let doc: Value = serde_json::from_str(&to_json3(cols)).unwrap();
    let input = json!({"testo": text, "separatore": sep, "colonne": doc["colonne"]});
    tabella_da_json(csv_leggi(&input.to_string()).unwrap().as_bytes()).unwrap()
}

#[test]
fn what_is_written_is_read_back_unchanged() {
    let mut t = rich();
    t.campioni[0].note = "has, comma; semi\ttab \"quote\"\nnewline".into();
    t.campioni[1].note = "  padded ".into();
    t.campioni[2].set_extra("alfa", Cell::Testo(String::new()));
    t.campioni[3].extra.retain(|(k, _)| k != "zeta");
    t.campioni[0].conc = Some(0.1 + 0.2);
    t.campioni[0].unit = Some("mg/L".into());
    for sep in [",", ";", "\t"] {
        let text = written(&t, sep);
        let back = read(&text, sep, &t);
        assert_eq!(to_json3(&back), to_json3(&t), "sep {sep:?}");
        assert_eq!(written(&back, sep), text);
        // the separator is found by itself
        assert_eq!(to_json3(&read(&text, "", &t)), to_json3(&t));
    }
}

#[test]
fn free_columns_are_typed_by_the_values_when_not_declared() {
    let text = "file;note;temp;code;empty\nA_t5;x;21,5;007;\nB_t5;y;22;12;\n";
    let t = read(text, "", &Tabella::default());
    let kind = |n: &str| t.colonne.iter().find(|c| c.name == n).unwrap().kind;
    // "21,5" is not a number written the way numbers are written here, so the column stays text and nothing is lost
    assert_eq!(kind("temp"), TipoColonna::Testo);
    assert_eq!(kind("code"), TipoColonna::Testo);
    assert_eq!(kind("empty"), TipoColonna::Testo);
    let t = read("file,temp\nA_t5,21.5\nB_t5,22\n", "", &Tabella::default());
    assert_eq!(t.colonne[0].kind, TipoColonna::Numero);
    assert_eq!(t.campioni[0].extra_of("temp"), Some(&Cell::Numero(21.5)));
    assert_eq!(
        written(&t, ","),
        written(&read(&written(&t, ","), "", &Tabella::default()), ",")
    );
}

#[test]
fn synonyms_and_the_units_of_time() {
    let text = "name,role,condition,replicate,injection_order,time,dose,polarity\n\
                f1,standard,,2,5,12,25 ppm,+\nf2,control,B,,6,,,neg\nf3,sample,B,,7,30,,\n";
    let t = read(text, "", &Tabella::default());
    let c = &t.campioni;
    assert_eq!(
        (c[0].kind.as_str(), c[0].replicate, c[0].order),
        ("std", 2, 5)
    );
    assert_eq!((c[0].conc, c[0].unit.as_deref()), (Some(25.0), Some("ppm")));
    assert_eq!((c[0].time_min, c[0].polarity), (Some(12.0), Some(1)));
    assert_eq!(
        (c[1].kind.as_str(), c[1].series.as_str(), c[1].polarity),
        ("dark", "B", Some(-1))
    );
    assert_eq!((c[2].time_min, c[2].time_confirmed), (Some(30.0), true));
    assert!(t.colonne.is_empty());
    let t = read(
        "file,tempo,unita_tempo\nx,2,h\ny,90,s\n",
        "",
        &Tabella::default(),
    );
    assert_eq!(
        (t.campioni[0].time_min, t.campioni[1].time_min),
        (Some(120.0), Some(1.5))
    );
    let t = read(
        "\u{feff}file,tipo\r\nt0,t0\r\n\r\nbl,blank\r\n",
        "",
        &Tabella::default(),
    );
    assert_eq!((t.campioni.len(), t.campioni[0].time_min), (2, Some(0.0)));
    // written replicates win, the others are numbered
    let t = read(
        "file,serie,tempo,replica\na,S,5,\nb,S,5,\nc,S,5,9\n",
        "",
        &Tabella::default(),
    );
    assert_eq!(
        t.campioni.iter().map(|c| c.replicate).collect::<Vec<_>>(),
        [1, 2, 9]
    );
}

#[test]
fn bad_tables_are_refused_with_the_place() {
    for (text, col) in [
        ("file,tipo\nx,nonsense\n", "tipo"),
        ("file,tempo\nx,abc\n", "tempo"),
        ("file,unita_tempo\nx,fortnight\n", "unita_tempo"),
        ("file,replica\nx,0\n", "replica"),
        ("file,polarita\nx,2\n", "polarita"),
        ("file,file\nx,y\n", "file"),
    ] {
        let e =
            campioni::route_tutto("csv_leggi", &json!({"testo": text}).to_string()).unwrap_err();
        assert_eq!(e.key, "error.badCsv", "{text}");
        assert!(
            e.params.iter().any(|(k, v)| *k == "colonna" && v == col),
            "{text}"
        );
    }
    assert!(
        campioni::route_tutto("csv_leggi", &json!({"testo": "file\nx,y\n"}).to_string()).is_err()
    );
    let none = campioni::route_tutto("csv_leggi", &json!({"testo": ""}).to_string()).unwrap();
    assert!(none.contains("\"campioni\":[]"));
    let _ = many;
}
