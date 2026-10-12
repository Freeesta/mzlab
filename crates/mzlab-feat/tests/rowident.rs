//! The identity columns (neutral mass, adduct, source) of the aligned table and the facts of the files in the meta: computed from the hypotheses,
//! written and read back identical; a table without them reads as before.

use mzlab_feat::facts::FileFacts;
use mzlab_feat::hypo::{Hypothesis, SRC_FAMILY, SRC_RULE};
use mzlab_feat::hypo::Dir;
use mzlab_feat::rowident::{adduct_names, RowIdent, SRC_DETECTED};
use mzlab_feat::store::{origin_from_meta, read_table, write_table, Origin};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED, CELL_TARGETED};

fn table() -> Aligned {
    let mut t = Aligned::with_capacity(2, 3);
    for (mz, rt, states) in [
        (150.0, 5.0, [CELL_DETECTED, CELL_DETECTED]),
        (200.1, 6.0, [CELL_ABSENT, CELL_TARGETED]),
        (300.2, 7.0, [CELL_DETECTED, CELL_ABSENT]),
    ] {
        t.mz.push(mz);
        t.rt.push(rt);
        for s in states {
            t.area.push(1e5);
            t.height.push(1e4);
            t.apex_rt.push(rt as f32);
            t.width.push(0.1);
            t.state.push(s);
        }
    }
    t
}

fn hyp(neutral: f64, adduct: &'static str, sources: u8) -> Hypothesis {
    let mz = mzlab_core::elements::ion_mz(neutral, adduct).unwrap();
    Hypothesis {
        id: 1,
        sources,
        label: "x".into(),
        formula: String::new(),
        neutral,
        mz,
        adduct,
        polarity: 1,
        steps: 1,
        level: 3,
        dir: Dir::Free,
        rt_ref: None,
        alternatives: Vec::new(),
    }
}

#[test]
fn identity_follows_the_matched_hypothesis() {
    let mut t = table();
    let na = hyp(177.0, "[M+Na]+", SRC_RULE | SRC_FAMILY);
    let hyps = [na.clone()];
    t.mz[0] = na.mz;
    let id = RowIdent::compute(&t, &hyps, 1);
    assert_eq!(id.len(), 3);
    let code = adduct_names().iter().position(|a| *a == "[M+Na]+").unwrap() as u8 + 1;
    assert_eq!((id.neutral[0], id.ion[0]), (177.0, code));
    assert_eq!(id.src[0], (SRC_RULE | SRC_FAMILY) << 1 | SRC_DETECTED);
    // no hypothesis: the protonated ion, and the source says only whether it was detected
    let h = adduct_names().iter().position(|a| *a == "[M+H]+").unwrap() as u8 + 1;
    assert_eq!(id.ion[2], h);
    assert!((id.neutral[2] - (300.2 - 1.00727646688)).abs() < 1e-9);
    assert_eq!((id.src[1], id.src[2]), (0, SRC_DETECTED));
}

#[test]
fn identity_and_facts_survive_the_file() {
    let mut t = table();
    t.ident = RowIdent::compute(&t, &[hyp(149.0, "[M+H]+", SRC_RULE)], 1);
    let facts = FileFacts {
        polarity: 1,
        resolution: 60000.0,
        acquisition: "dda".into(),
        instrument: "orbitrap".into(),
    };
    let o = Origin {
        inputs: vec![("a.mzML".into(), "ab".repeat(32)), ("b.mzML".into(), "cd".repeat(32))],
        facts: vec![facts.clone(), FileFacts::default()],
        samples: Some("ef".repeat(32)),
        ..Origin::default()
    };
    let bytes = write_table(&t, &Recipe::default(), &o);
    assert_eq!(bytes, write_table(&t, &Recipe::default(), &o));
    let (back, meta) = read_table(&bytes).unwrap();
    assert_eq!(back.ident, t.ident);
    assert_eq!(back.to_bytes(), t.to_bytes());
    let o2 = origin_from_meta(&meta);
    assert_eq!(o2.inputs, o.inputs);
    assert_eq!(o2.facts, o.facts);
    assert_eq!(o2.samples, o.samples);
    assert_eq!(meta.get("adducts").unwrap().arr().len(), adduct_names().len());
    assert!(read_table(&bytes[..bytes.len() - 100]).is_err());
}

#[test]
fn a_table_without_identity_reads_empty() {
    let t = table();
    let bytes = write_table(&t, &Recipe::default(), &Origin::default());
    let (back, meta) = read_table(&bytes).unwrap();
    assert!(back.ident.is_empty());
    assert!(meta.get("adducts").is_none() && meta.get("columns").is_none());
}
