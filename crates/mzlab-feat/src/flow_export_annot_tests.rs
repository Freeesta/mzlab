//! Tests of `export.annotations`: invented fixtures (4 features, a library, an in silico file, a screen, formulas and an `annot/1`), the exact CSV and its variants.

use super::*;
use crate::aligned::{Aligned, CELL_DETECTED};
use crate::recipe::Recipe;
use crate::store::{write_table, Origin};

fn table() -> Vec<u8> {
    let mut t = Aligned::with_capacity(1, 4);
    for (mz, rt) in [(200.1, 1.5), (250.2, 2.5), (300.3, 3.5), (350.4, 4.5)] {
        t.mz.push(mz);
        t.rt.push(rt);
        t.area.push(1e6);
        t.height.push(1e5);
        t.apex_rt.push(rt as f32);
        t.width.push(0.1);
        t.state.push(CELL_DETECTED);
    }
    write_table(&t, &Recipe::default(), &Origin::default())
}

fn hit(q: &str, name: &str, score: f64, silico: bool) -> String {
    let (source, alg) = if silico {
        ("insilico", "fragmenter")
    } else {
        ("library", "spectral_entropy")
    };
    format!("{{\"query\":\"{q}\",\"id\":\"L\",\"name\":\"{name}\",\"inchikey\":\"AAAAAAAAAAAAAA-BBBBBBBBBB-N\",\"smiles\":\"C\",\"formula\":\"C2H6\",\"score\":{score},\"shared\":5,\"mass_ppm\":1.25,\"precursor_delta\":0,\"source\":\"{source}\",\"algorithm\":\"{alg}\"}}")
}

fn library() -> String {
    let mut hits: Vec<String> = (0..3)
        .map(|k| hit("1", &format!("lib1-{k}"), 0.9 - 0.1 * k as f64, false))
        .collect();
    hits.push(hit("4", "lib4", 0.65, false));
    hits.extend((0..12).map(|k| hit("2", &format!("lib2-{k:02}"), 0.99 - 0.01 * k as f64, false)));
    format!("{{\"format\":\"candidates/1\",\"step\":\"spectra.match@1\",\"algorithm\":\"spectral_entropy\",\"hits\":[{}]}}", hits.join(","))
}

fn silico() -> String {
    let hits = [hit("1", "sil-a", 0.4, true), hit("1", "sil-b", 0.5, true)];
    format!("{{\"format\":\"candidates/1\",\"step\":\"spectra.insilico@1\",\"algorithm\":\"fragmenter\",\"hits\":[{}]}}", hits.join(","))
}

fn screen() -> String {
    let cand = |name: &str, ik: &str, m: u32, lists: &str| {
        format!("{{\"skeleton\":\"\",\"inchikey\":\"{ik}\",\"name\":\"{name}\",\"smiles\":\"C\",\"neutral_mass\":100,\"rt_diff_min\":0.1,\"fragments_listed\":3,\"fragments_matched\":{m},\"fragment_mz\":[],\"files\":[0],\"memberships\":[{lists}]}}")
    };
    let member =
        "{\"list\":\"eu-pfas\",\"entry_ref\":\"1\",\"form\":\"x\",\"group\":false,\"attr\":\"\"}";
    format!("{{\"format\":\"screen/1\",\"hits\":[{{\"feature\":3,\"mz\":300.3,\"adduct\":\"[M+H]+\",\"delta_ppm\":-0.5,\"formula\":\"C3H8O\",\"level\":5,\"candidates\":[{},{}]}}]}}", cand("iso-a", "CCCCCCCCCCCCCC-DDDDDDDDDD-N", 1, ""), cand("iso-b", "EEEEEEEEEEEEEE-FFFFFFFFFF-N", 2, member))
}

fn formulas() -> String {
    let f = |id: u32, neutral: &str| {
        format!("{{\"id\":\"{id}\",\"mz\":1,\"rt\":1,\"file\":\"f\",\"ion\":\"[M+H]+\",\"scan\":1,\"result\":{{\"candidates\":[{{\"formula\":\"C2H7+\",\"neutral\":\"{neutral}\",\"mz_calc\":1,\"error_ppm\":0.75,\"rdb\":0,\"score\":0.8,\"confidence\":0.5}}]}}}}")
    };
    format!(
        "{{\"format\":\"formulas/1\",\"step\":\"formula.ms1@1\",\"features\":[{},{},{}]}}",
        f(1, "C2H6"),
        f(2, "C3H8"),
        f(3, "C3H8O")
    )
}

fn annot() -> String {
    let row = |id: u32, level: &str, why: &str| {
        format!("{{\"id\":\"{id}\",\"level\":\"{level}\",\"why\":\"{why}\",\"gates\":[]}}")
    };
    let std = "{\"id\":\"1\",\"level\":\"1\",\"why\":\"standard\",\"name\":\"Std\",\"entry\":\"ZZZZZZZZZZZZZZ-YYYYYYYYYY-N\",\"standard\":{\"name\":\"Std\",\"rt\":1.5,\"adduct\":\"[M+H]+\",\"ran\":true,\"gates\":[{\"id\":\"stdrt\",\"ok\":true,\"value\":0.05,\"threshold\":0.2},{\"id\":\"stdscore\",\"ok\":true,\"value\":0.95,\"threshold\":0.7},{\"id\":\"stdppm\",\"ok\":true,\"value\":1.1,\"threshold\":5}]},\"gates\":[]}";
    format!(
        "{{\"format\":\"annot/1\",\"rows\":[{},{},{},{}]}}",
        std,
        row(2, "2a", "2a"),
        row(3, "3a", "suspect_fragments"),
        row(4, "5", "none")
    )
}

fn run(p: &Params, with_annot: bool, extra: &str) -> Made {
    let (l, s, f, a, e) = (library(), silico(), screen(), formulas(), annot());
    let tab = table();
    let cands = [
        ("match.json", l.as_bytes()),
        ("insilico.json", s.as_bytes()),
        ("extra.json", extra.as_bytes()),
    ];
    let inputs = Inputs {
        table: &tab,
        candidates: if extra.is_empty() {
            &cands[..2]
        } else {
            &cands
        },
        screen: &[("screen.json", f.as_bytes())],
        formulas: &[("formulas.json", a.as_bytes())],
        annot: with_annot.then_some(("annot.json", e.as_bytes())),
    };
    export_made(p, &inputs).unwrap()
}

fn params(top: usize, min: &str, sep: char) -> Params {
    Params {
        top_per_method: top,
        min_level: min.into(),
        separator: sep,
    }
}

#[test]
fn the_csv_is_exact() {
    let made = run(&params(10, "", ','), true, "");
    assert_eq!(made.unjoined, 0);
    assert_eq!(
        made.csv,
        include_str!("../tests/golden/export_annotations.csv")
    );
}

#[test]
fn a_feature_keeps_at_most_top_per_method_rows_of_a_method() {
    let rows = |top| {
        run(&params(top, "", ','), true, "")
            .csv
            .lines()
            .filter(|l| l.starts_with("2,") && l.contains(",library,"))
            .count()
    };
    assert_eq!(rows(10), 10);
    assert_eq!(rows(3), 3);
}

#[test]
fn min_level_drops_the_features_below_it() {
    let csv = run(&params(10, "3a", ','), true, "").csv;
    assert!(csv.lines().skip(1).all(|l| !l.starts_with("4,")));
    assert!(csv.lines().any(|l| l.starts_with("3,")));
}

#[test]
fn a_bad_min_level_is_an_error() {
    let tab = table();
    let inputs = Inputs {
        table: &tab,
        ..Inputs::default()
    };
    assert_eq!(
        export_made(&params(10, "9z", ','), &inputs)
            .err()
            .unwrap()
            .key,
        "err.flow.annotations.level"
    );
}

#[test]
fn the_semicolon_only_changes_the_separator() {
    let comma = run(&params(10, "", ','), true, "").csv;
    let semi = run(&params(10, "", ';'), true, "").csv;
    assert_eq!(comma.replace(',', ";"), semi);
}

#[test]
fn without_annot_the_level_columns_are_empty() {
    let csv = run(&params(10, "", ','), false, "").csv;
    for l in csv.lines().skip(1) {
        let c: Vec<&str> = l.split(',').collect();
        assert_eq!((c[3], c[4]), ("", ""), "{l}");
    }
    assert!(csv.lines().count() > 1);
}

#[test]
fn a_query_that_cannot_be_joined_keeps_an_empty_feature_id_and_is_counted() {
    let extra = format!("{{\"format\":\"candidates/1\",\"step\":\"spectra.match@1\",\"algorithm\":\"cosine\",\"hits\":[{}]}}", hit("sample.12", "orphan", 0.7, false));
    let made = run(&params(10, "", ','), true, &extra);
    assert_eq!(made.unjoined, 1);
    let last = made.csv.lines().last().unwrap();
    assert!(last.starts_with(",,,,,library,1,orphan,"), "{last}");
    assert!(last.ends_with(",extra.json:sample.12"), "{last}");
}

#[test]
fn the_packed_text_is_the_same_text() {
    let (l, a) = (library(), annot());
    let manifest = format!("[{{\"port\":\"candidates\",\"name\":\"match.json\",\"len\":{}}},{{\"port\":\"annot\",\"name\":\"annot.json\",\"len\":{}}}]", l.len(), a.len());
    let blob = [l.as_bytes(), a.as_bytes()].concat();
    let packed = export_text_packed("{}", &table(), &manifest, &blob).unwrap();
    let inputs = Inputs {
        table: &table(),
        candidates: &[("match.json", l.as_bytes())],
        annot: Some(("annot.json", a.as_bytes())),
        ..Inputs::default()
    };
    assert_eq!(
        packed,
        export_made(&Params::from_params(&[]), &inputs).unwrap().csv
    );
}
