//! The lists of the user (`liste/1`): CSV to JSON and back to the same bytes, an error for every wrong row, the same digest for any order of the rows, and the
//! transformation list of mzFinder read through the same code with the same rows.

use mzlab_core::json::parse;
use mzlab_feat::legacy::transf;
use mzlab_feat::liste::{errors_json, from_csv, from_json, from_legacy_text, to_json, Kind, Liste};

const TRANSF: &str = "name;delta;max_repeat;comment\nhydroxylation;+O;3;lit: ref one\ndemethylation;-CH2;;\n\"loss, water\";-H2O;2;\"a \"\"quoted\"\" note\"\n";
const ADDUCTS: &str =
    "name,delta,charge,comment\n[M+Na]+,+Na,1,\n[M+NH4]+,+NH4,1,\n[M-H]-,-H,-1,negative\n";
const SUSPECTS: &str = "# invented\nname;formula;mass;comment\nsuspect A;C10H12N2O;;from a list\nsuspect B;;234.1012;mass only\nsuspect C;C8H10O2;138.0681;both\n";

fn round(kind: Kind, csv: &str) -> (Liste, String) {
    let list = from_csv(kind, csv).unwrap_or_else(|e| panic!("{e:?}"));
    let json = to_json(&list);
    let back = from_json(json.as_bytes()).unwrap();
    assert_eq!(to_json(&back), json, "{} round trip", kind.name());
    assert_eq!(back.digest(), list.digest());
    (list, json)
}

#[test]
fn every_kind_goes_from_csv_to_json_and_back_to_the_same_bytes() {
    for (kind, csv, n) in [
        (Kind::Transformations, TRANSF, 3),
        (Kind::Adducts, ADDUCTS, 3),
        (Kind::Suspects, SUSPECTS, 3),
    ] {
        let (list, json) = round(kind, csv);
        assert_eq!(list.len(), n);
        let v = parse(json.as_bytes()).unwrap();
        assert_eq!(v.get("schema").unwrap().str(), Some("liste/1"));
        assert_eq!(v.get("tipo").unwrap().str(), Some(kind.name()));
        assert_eq!(list.digest().len(), 64);
    }
    let (list, _) = round(Kind::Transformations, TRANSF);
    let Liste::Transformations(t) = &list else {
        panic!()
    };
    assert_eq!(t[0].max_repeat, Some(3));
    assert_eq!(t[2].name, "loss, water");
    assert_eq!(t[2].comment, "a \"quoted\" note");
}

#[test]
fn the_order_of_the_rows_does_not_change_the_bytes() {
    let a = from_csv(Kind::Adducts, ADDUCTS).unwrap();
    let shuffled =
        "name;delta;charge;comment\n[M-H]-;-H;-1;negative\n[M+NH4]+;+NH4;1;\n[M+Na]+;+Na;1;\n";
    let b = from_csv(Kind::Adducts, shuffled).unwrap();
    assert_eq!(to_json(&a), to_json(&b));
    assert_eq!(a.digest(), b.digest());
    // another comment is another list
    let c = from_csv(Kind::Adducts, &shuffled.replace("negative", "other")).unwrap();
    assert_ne!(a.digest(), c.digest());
}

#[test]
fn every_wrong_row_has_its_own_error() {
    let csv = "name;delta;max_repeat;comment\nok;+O;2;\n;+O;;\nbad delta;+;;\nno change;;;\nrepeat;+O;11;\ntext;+O;x;\nfine;-H2;;\n";
    let e = from_csv(Kind::Transformations, csv).unwrap_err();
    let got: Vec<(usize, &str)> = e.iter().map(|r| (r.line, r.key)).collect();
    assert_eq!(
        got,
        [
            (3, "error.liste.noName"),
            (4, "error.liste.noDelta"),
            (5, "error.liste.noDelta"),
            (6, "error.liste.badRepeat"),
            (7, "error.liste.badRepeat"),
        ]
    );
    let json = errors_json(&e);
    assert_eq!(
        parse(json.as_bytes())
            .unwrap()
            .get("errors")
            .unwrap()
            .arr()
            .len(),
        5
    );
    let a = from_csv(
        Kind::Adducts,
        "name;delta;charge\nx;+Na;0\ny;+K;4\nz;+Q;1\n",
    )
    .unwrap_err();
    assert_eq!(
        a.iter().map(|r| r.key).collect::<Vec<_>>(),
        [
            "error.liste.badCharge",
            "error.liste.badCharge",
            "error.liste.badDelta"
        ]
    );
    let s = from_csv(Kind::Suspects, "name;formula;mass\na;;\nb;Xx2;\nc;;-5\n").unwrap_err();
    assert_eq!(
        s.iter().map(|r| (r.line, r.key)).collect::<Vec<_>>(),
        [
            (2, "error.liste.noMass"),
            (3, "error.liste.badFormula"),
            (4, "error.liste.badMass")
        ]
    );
}

#[test]
fn a_header_is_required_except_for_the_text_of_mzfinder() {
    assert_eq!(
        from_csv(Kind::Adducts, "[M+Na]+;+Na;1\n").unwrap_err()[0].key,
        "error.liste.noHeader"
    );
    assert_eq!(
        from_csv(Kind::Suspects, "name;colour\na;b\n").unwrap_err()[0].key,
        "error.liste.badColumn"
    );
    assert!(from_csv(Kind::Suspects, "").unwrap().is_empty());
    // the header-less form is `name;change[;comment]`
    let l = from_csv(Kind::Transformations, "# list\nhydroxylation;+O;note\n").unwrap();
    assert_eq!(l.len(), 1);
}

#[test]
fn the_text_of_mzfinder_gives_the_same_rows_through_the_list() {
    let text = "# comment\nhydroxylation;+O\n\nloss;-H2O;lit: a reference\nsame again;+O\n";
    let direct = transf::parse(text).unwrap();
    let via = from_legacy_text(text).unwrap().transformation_rows();
    assert_eq!(direct.len(), via.len());
    for (a, b) in direct.iter().zip(&via) {
        assert_eq!(
            (&a.name, &a.delta, &a.comment, a.max_repeat),
            (&b.name, &b.delta, &b.comment, b.max_repeat)
        );
    }
    assert!(
        from_legacy_text("one;").is_err(),
        "the errors are those of the text"
    );
}

/// Today's `liste/1` reading of the synthetic suspects file; M6 turns the same file into a molecules collection named like the file.
#[test]
fn synthetic_suspects_fixture_reads_today() {
    let csv = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/dati/sospetti_sintetici.csv"
    ))
    .unwrap();
    let (list, json) = round(Kind::Suspects, &csv);
    assert_eq!(list.len(), 3);
    assert!(json.contains("\"nome\":\"Esempina\"") && json.contains("\"massa\":234.1012"));
}
