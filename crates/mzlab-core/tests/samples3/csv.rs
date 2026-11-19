//! CSV of `samples/1`: round trip both ways, the three separators, quotes and newlines, synonyms of the headers, typed free columns, errors.

use super::{many, rich};
use mzlab_core::samples::{self, csv_read, csv_write, Cell, ColumnType, Table};
use mzlab_core::samples::{table_from_json, table_to_json};
use serde_json::{json, Value};

fn written(t: &Table, sep: &str) -> String {
    let doc = table_to_json(t);
    let r: Value = serde_json::from_str(
        &csv_write(
            &json!({"samples": serde_json::from_str::<Value>(&doc).unwrap(), "separator": sep})
                .to_string(),
        )
        .unwrap(),
    )
    .unwrap();
    r["text"].as_str().unwrap().to_string()
}

fn read(text: &str, sep: &str, cols: &Table) -> Table {
    let doc: Value = serde_json::from_str(&table_to_json(cols)).unwrap();
    let input = json!({"text": text, "separator": sep, "columns": doc["columns"]});
    table_from_json(csv_read(&input.to_string()).unwrap().as_bytes()).unwrap()
}

#[test]
fn what_is_written_is_read_back_unchanged() {
    let mut t = rich();
    t.samples[0].note = "has, comma; semi\ttab \"quote\"\nnewline".into();
    t.samples[1].note = "  padded ".into();
    t.samples[2].set_extra("alfa", Cell::Text(String::new()));
    t.samples[3].extra.retain(|(k, _)| k != "zeta");
    t.samples[0].conc = Some(0.1 + 0.2);
    t.samples[0].unit = Some("mg/L".into());
    for sep in [",", ";", "\t"] {
        let text = written(&t, sep);
        let back = read(&text, sep, &t);
        assert_eq!(table_to_json(&back), table_to_json(&t), "sep {sep:?}");
        assert_eq!(written(&back, sep), text);
        // the separator is found by itself
        assert_eq!(table_to_json(&read(&text, "", &t)), table_to_json(&t));
    }
}

#[test]
fn free_columns_are_typed_by_the_values_when_not_declared() {
    let text = "file;note;temp;code;empty\nA_t5;x;21,5;007;\nB_t5;y;22;12;\n";
    let t = read(text, "", &Table::default());
    let kind = |n: &str| t.columns.iter().find(|c| c.name == n).unwrap().kind;
    // "21,5" is not a number written the way numbers are written here, so the column stays text and nothing is lost
    assert_eq!(kind("temp"), ColumnType::Text);
    assert_eq!(kind("code"), ColumnType::Text);
    assert_eq!(kind("empty"), ColumnType::Text);
    let t = read("file,temp\nA_t5,21.5\nB_t5,22\n", "", &Table::default());
    assert_eq!(t.columns[0].kind, ColumnType::Number);
    assert_eq!(t.samples[0].extra_of("temp"), Some(&Cell::Number(21.5)));
    assert_eq!(
        written(&t, ","),
        written(&read(&written(&t, ","), "", &Table::default()), ",")
    );
}

#[test]
fn synonyms_and_the_units_of_time() {
    let text = "name,role,condition,replicate,injection_order,time,dose,polarity\n\
                f1,standard,,2,5,12,25 ppm,+\nf2,control,B,,6,,,neg\nf3,sample,B,,7,30,,\n";
    let t = read(text, "", &Table::default());
    let c = &t.samples;
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
    assert!(t.columns.is_empty());
    let t = read(
        "file,time,time_unit\nx,2,h\ny,90,s\n",
        "",
        &Table::default(),
    );
    assert_eq!(
        (t.samples[0].time_min, t.samples[1].time_min),
        (Some(120.0), Some(1.5))
    );
    let t = read(
        "\u{feff}file,kind\r\nt0,t0\r\n\r\nbl,blank\r\n",
        "",
        &Table::default(),
    );
    assert_eq!((t.samples.len(), t.samples[0].time_min), (2, Some(0.0)));
    // written replicates win, the others are numbered
    let t = read(
        "file,series,time,replicate\na,S,5,\nb,S,5,\nc,S,5,9\n",
        "",
        &Table::default(),
    );
    assert_eq!(
        t.samples.iter().map(|c| c.replicate).collect::<Vec<_>>(),
        [1, 2, 9]
    );
}

#[test]
fn bad_tables_are_refused_with_the_place() {
    for (text, col) in [
        ("file,kind\nx,nonsense\n", "kind"),
        ("file,time\nx,abc\n", "time"),
        ("file,time_unit\nx,fortnight\n", "time_unit"),
        ("file,replicate\nx,0\n", "replicate"),
        ("file,polarity\nx,2\n", "polarity"),
        ("file,file\nx,y\n", "file"),
    ] {
        let e = samples::route_all("csv_read", &json!({"text": text}).to_string()).unwrap_err();
        assert_eq!(e.key, "error.badCsv", "{text}");
        assert!(
            e.params.iter().any(|(k, v)| *k == "column" && v == col),
            "{text}"
        );
    }
    assert!(samples::route_all("csv_read", &json!({"text": "file\nx,y\n"}).to_string()).is_err());
    let none = samples::route_all("csv_read", &json!({"text": ""}).to_string()).unwrap();
    assert!(none.contains("\"samples\":[]"));
    let _ = many;
}
