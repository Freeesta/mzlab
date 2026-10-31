//! `qc.run_order@1`: the facts and warnings of the injection sequence on synthetic sample tables.

use super::runorder_text;
use mzlab_core::campioni::{self, Campione, Tabella};
use mzlab_core::json::{parse, Value};

/// A file of the table: name, role, injection order, batch and concentration.
fn file(name: &str, kind: &str, order: u32, batch: &str, conc: Option<f64>) -> Campione {
    let mut c = Campione::new(name, "", order);
    c.kind = kind.into();
    c.batch = batch.into();
    c.conc = conc;
    c
}

/// A batch of `n` injections starting at `from`: a QC every `qc_every`, a blank first (when `blank`) and the standards at `levels`.
fn batch(
    name: &str,
    from: u32,
    n: u32,
    qc_every: u32,
    blank: bool,
    levels: &[f64],
) -> Vec<Campione> {
    (0..n)
        .map(|k| {
            let id = format!("{name}{k:02}");
            let order = from + k;
            match k {
                0 if blank => file(&id, "blank", order, name, None),
                _ if k > 0 && k <= levels.len() as u32 => {
                    file(&id, "std", order, name, Some(levels[k as usize - 1]))
                }
                _ if k % qc_every == qc_every - 1 => file(&id, "qc", order, name, None),
                _ => file(&id, "sample", order, name, None),
            }
        })
        .collect()
}

fn v2(cs: &[Campione]) -> Vec<u8> {
    campioni::to_json(cs).into_bytes()
}

fn v3(cs: &[Campione]) -> Vec<u8> {
    campioni::to_json3(&Tabella {
        campioni: cs.to_vec(),
        colonne: Vec::new(),
    })
    .into_bytes()
}

fn keys(text: &str) -> Vec<String> {
    let doc = parse(text.as_bytes()).unwrap();
    doc.get("warnings")
        .unwrap()
        .arr()
        .iter()
        .map(|w| {
            format!(
                "{}:{}",
                w.get("key").unwrap().str().unwrap(),
                w.get("batch").unwrap().str().unwrap()
            )
        })
        .collect()
}

fn warning<'a>(doc: &'a Value, key: &str) -> &'a Value {
    doc.get("warnings")
        .unwrap()
        .arr()
        .iter()
        .find(|w| w.get("key").unwrap().str() == Some(key))
        .unwrap()
}

#[test]
fn a_clean_table_has_no_warning() {
    let mut cs = batch("A", 1, 20, 5, true, &[1.0, 10.0]);
    cs.extend(batch("B", 21, 20, 5, true, &[1.0, 10.0]));
    let text = runorder_text(&[], &v3(&cs), None).unwrap();
    assert!(keys(&text).is_empty(), "{text}");
    let doc = parse(text.as_bytes()).unwrap();
    assert_eq!(doc.get("schema").unwrap().str(), Some("runorder/1"));
    assert_eq!(doc.get("batches").unwrap().arr().len(), 2);
}

#[test]
fn a_wide_qc_gap_a_missing_blank_and_one_standard_level_are_reported() {
    // batch A: QC at 5 and 17 (12 injections apart), no blank, standards at one level; batch B is clean
    let mut a = batch("A", 1, 24, 100, false, &[5.0]);
    for (k, c) in a.iter_mut().enumerate() {
        c.kind = if k == 4 || k == 16 {
            "qc".into()
        } else if k == 0 {
            "std".into()
        } else {
            "sample".into()
        };
        c.conc = (k == 0).then_some(5.0);
    }
    let mut cs = a;
    cs.extend(batch("B", 25, 20, 5, true, &[1.0, 10.0]));
    let text = runorder_text(&[], &v3(&cs), None).unwrap();
    assert_eq!(
        keys(&text),
        [
            "warn.qc.runorder.qc_gap:A",
            "warn.qc.runorder.no_blank_batch:A",
            "warn.qc.runorder.std_levels:A"
        ],
        "{text}"
    );
    let doc = parse(text.as_bytes()).unwrap();
    let gap = warning(&doc, "warn.qc.runorder.qc_gap");
    assert_eq!(gap.get("value").unwrap().f64(), Some(12.0));
    assert_eq!(gap.get("files").unwrap().arr().len(), 2);
    assert_eq!(
        warning(&doc, "warn.qc.runorder.std_levels")
            .get("value")
            .unwrap()
            .f64(),
        Some(1.0)
    );
}

#[test]
fn a_batch_without_qc_is_reported_and_the_limits_are_parameters() {
    let cs = batch("A", 1, 12, 100, true, &[1.0, 2.0]);
    assert_eq!(
        keys(&runorder_text(&[], &v2(&cs), None).unwrap()),
        ["warn.qc.runorder.no_qc:1"]
    );
    let off = vec![
        ("blank_per_batch".to_string(), Value::Bool(false)),
        ("std_levels_min".to_string(), Value::Num(3.0)),
    ];
    assert_eq!(
        keys(&runorder_text(&off, &v2(&cs), None).unwrap()),
        ["warn.qc.runorder.no_qc:1", "warn.qc.runorder.std_levels:1"]
    );
    let wide = batch("A", 1, 30, 12, true, &[1.0, 2.0]);
    assert_eq!(
        keys(&runorder_text(&[], &v2(&wide), None).unwrap()),
        ["warn.qc.runorder.qc_gap:1"]
    );
    let loose = vec![("qc_every_max".to_string(), Value::Num(12.0))];
    assert!(keys(&runorder_text(&loose, &v2(&wide), None).unwrap()).is_empty());
}

#[test]
fn a_table_without_order_and_one_with_repeated_orders_are_reported() {
    let cs = batch("A", 1, 10, 5, true, &[1.0, 2.0]);
    let mut doc = parse(&v2(&cs)).unwrap();
    if let Value::Obj(fields) = &mut doc
        && let Some((_, Value::Arr(items))) = fields.iter_mut().find(|(k, _)| k == "campioni")
    {
        for (k, item) in items.iter_mut().enumerate() {
            if let Value::Obj(f) = item {
                if k == 3 {
                    f.retain(|(key, _)| key != "ordine_iniezione");
                }
                if k == 5 || k == 6 {
                    f.iter_mut()
                        .find(|(key, _)| key == "ordine_iniezione")
                        .unwrap()
                        .1 = Value::Num(8.0);
                }
            }
        }
    }
    let mut out = mzlab_core::json::Out::default();
    mzlab_core::json::write_value(&mut out, &doc);
    let text = runorder_text(&[], out.0.as_bytes(), None).unwrap();
    let doc = parse(text.as_bytes()).unwrap();
    assert_eq!(
        warning(&doc, "warn.qc.runorder.order_missing")
            .get("files")
            .unwrap()
            .arr()[0]
            .str(),
        Some("A03")
    );
    assert_eq!(
        warning(&doc, "warn.qc.runorder.order_duplicate")
            .get("files")
            .unwrap()
            .arr()
            .len(),
        3
    );
}

#[test]
fn campioni_2_and_3_give_the_same_result_and_a_design_overrides_the_batch_field() {
    let mut cs = batch("A", 1, 20, 5, true, &[1.0, 10.0]);
    cs.extend(batch("B", 21, 20, 5, false, &[1.0, 10.0]));
    for c in &mut cs {
        c.batch = String::new();
    }
    let design = |split: u32| {
        let lots: Vec<String> = cs
            .iter()
            .map(|c| {
                format!(
                    "{{\"file\":\"{}\",\"lotto\":\"{}\",\"sha256\":\"\"}}",
                    c.file,
                    if c.order <= split { "X" } else { "Y" }
                )
            })
            .collect();
        format!("{{\"condizioni\":[{{\"livelli\":{{}},\"nome\":\"\",\"repliche\":1}}],\"confronti\":[],\"fattori\":[],\"lotti\":[{}],\"riferimento\":null,\"schema\":\"disegno/1\"}}", lots.join(",")).into_bytes()
    };
    let one = runorder_text(&[], &v2(&cs), None).unwrap();
    assert_eq!(one, runorder_text(&[], &v3(&cs), None).unwrap());
    assert_eq!(
        parse(one.as_bytes())
            .unwrap()
            .get("batches")
            .unwrap()
            .arr()
            .len(),
        1
    );
    let split = runorder_text(&[], &v2(&cs), Some(&design(20))).unwrap();
    assert_eq!(
        split,
        runorder_text(&[], &v3(&cs), Some(&design(20))).unwrap()
    );
    assert_eq!(keys(&split), ["warn.qc.runorder.no_blank_batch:Y"]);
}

#[test]
fn the_text_does_not_depend_on_the_order_of_the_files_in_the_table() {
    let mut cs = batch("A", 1, 20, 7, false, &[1.0]);
    let forward = runorder_text(&[], &v2(&cs), None).unwrap();
    cs.reverse();
    assert_eq!(forward, runorder_text(&[], &v2(&cs), None).unwrap());
}
