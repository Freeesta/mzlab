//! The MS2 targets (`lr::ms2target`): an MS2 on an ion that the Dig does not list makes a row of its own. The series has a small isomer-like ion at
//! the m/z of a hypothesis at another time: the Dig drops it (isomer limit, and no unexpected ion within the tolerance of a hypothesis).

use super::*;

const TARGET_RT: f64 = 11.0;

/// 5 % of the main isomer of the hypothesis at m/z 257.059, 3.6 min after the second one: below the 10 % of the isomer rule.
fn extra(t: Option<f64>) -> Vec<(f64, f64, f64, f64)> {
    t.map_or(vec![], |k| {
        vec![(257.0590, TARGET_RT, 0.07, 6.0e4 * (1.0 - (-0.06 * k).exp()))]
    })
}

fn with_target() -> &'static (Vec<Vec<u8>>, lr::LrDig) {
    static BASE: OnceLock<(Vec<Vec<u8>>, lr::LrDig)> = OnceLock::new();
    BASE.get_or_init(|| {
        let (files, idxs) = series_with(&extra);
        let d = dig(&files, &idxs);
        (idxs, d)
    })
}

fn link(d: &mut lr::LrDig, name: &str, f: &[u8], ov: &[Override]) {
    d.files.push(FileLr {
        name: name.into(),
        label: name.into(),
        time: None,
        kind_type: "sample".into(),
        kind: "ms2".into(),
    });
    let cols = &with_target().0;
    let all: Vec<&[u8]> = cols
        .iter()
        .map(|c| c.as_slice())
        .chain([f].into_iter())
        .collect();
    ms2link::ms2_link(d, ov, &mut |k| Ok(open(all[k]))).unwrap();
}

fn targets(d: &lr::LrDig) -> Vec<&lr::Entry> {
    d.entries.iter().filter(|e| e.ms2_target).collect()
}

/// P that a student would set for the feature: its m/z as the instrument shows it, to 0.1.
fn p_feature() -> f64 {
    ((257.0590 + SHIFT) * 10.0).round() / 10.0
}

#[test]
fn the_dig_does_not_list_the_ion_and_an_ms2_on_it_makes_a_row_with_its_criteria() {
    let mut d = with_target().1.clone();
    assert!(
        d.entries
            .iter()
            .all(|e| e.ref_rt.is_none_or(|t| (t - TARGET_RT).abs() > 0.5)),
        "the Dig lists the ion"
    );
    let n = d.entries.len();
    let f = ms2_file("t.mzML", &[x(p_feature(), &[(TARGET_RT, 0.07, 5.0e4)])]);
    link(&mut d, "t.mzML", &f, &[]);
    let t = targets(&d);
    assert_eq!((d.entries.len(), t.len()), (n + 1, 1));
    let e = t[0];
    assert!(e.unexpected && e.id == 200_001 && e.row.is_some());
    assert_eq!(
        e.name,
        format!(
            "MS2 target m/z {:.1} @ {:.2} min",
            p_feature(),
            e.ref_rt.unwrap()
        )
    );
    assert!((e.ref_rt.unwrap() - TARGET_RT).abs() < 0.1);
    assert!(e.cells.iter().any(|c| c.detected), "no cell");
    assert!(e.score.is_some() && !e.criteria.is_empty());
    let l = links(&d);
    assert_eq!(
        (l.links.len(), l.links[0].row),
        (1, 200_001),
        "{:?}",
        l.links
    );
    assert_eq!(l.links[0].state, State::Auto);
    assert!(
        l.links[0].why.ends_with("row made from this MS2"),
        "{}",
        l.links[0].why
    );
    assert!(l.unmatched.is_empty(), "{:?}", l.unmatched);
    // the summary: kind unexpected, ms2_target only on this row, one match; the live feed is the candidates
    let s: Value = serde_json::from_str(&lr::summary::summary(&d, "{}")).unwrap();
    let rows = s["rows"].as_array().unwrap();
    let r = rows.iter().find(|r| r["id"] == 200_001).unwrap();
    assert_eq!(
        (r["kind"].as_str(), r["ms2"].as_i64()),
        (Some("unexpected"), Some(1))
    );
    assert_eq!(r["ms2_target"], true);
    assert_eq!(
        rows.iter()
            .filter(|r| r.get("ms2_target").is_some())
            .count(),
        1
    );
    assert!(!lr::summary::live(&d).contains("200001"));
    // the pool of the row: the scans of the peak of the trace
    let cols = &with_target().0;
    let mut op = |k: usize| {
        Ok(if k < cols.len() {
            open(&cols[k])
        } else {
            open(&f)
        })
    };
    let m = pool(&d, e, 6, &mut op).unwrap();
    assert!(
        m.scans >= 10 && m.frags.len() == 2,
        "{} {}",
        m.scans,
        m.frags.len()
    );
}

#[test]
fn a_peak_with_no_feature_in_the_table_is_no_feature_and_makes_no_row() {
    let mut d = with_target().1.clone();
    let n = d.entries.len();
    let f = ms2_file("nf.mzML", &[x(380.1, &[(12.0, 0.07, 5.0e4)])]);
    link(&mut d, "nf.mzML", &f, &[]);
    assert_eq!(d.entries.len(), n);
    // the end of the trace is a peak of its own: every peak that the table does not explain is listed
    assert!(reasons(&d).iter().all(|r| *r == "no_feature") && !reasons(&d).is_empty());
    assert!(links(&d).links.is_empty());
    // a feature at the m/z but at another time: no_feature too
    let mut d = with_target().1.clone();
    let f = ms2_file(
        "nf2.mzML",
        &[x(p_feature(), &[(TARGET_RT + 1.5, 0.07, 5.0e4)])],
    );
    link(&mut d, "nf2.mzML", &f, &[]);
    assert!(
        reasons(&d).contains(&"no_feature"),
        "{:?}",
        links(&d).unmatched
    );
    assert!(targets(&d).is_empty());
}

#[test]
fn an_ion_that_is_already_a_row_makes_no_target() {
    let mut d = with_target().1.clone();
    let n = d.entries.len();
    let p = p_of(&d, 7);
    let f = ms2_file("row.mzML", &[x(p, &[(7.4, 0.07, 5.0e4)])]);
    link(&mut d, "row.mzML", &f, &[]);
    assert_eq!((d.entries.len(), targets(&d).len()), (n, 0));
    assert_eq!((links(&d).links.len(), links(&d).links[0].row), (1, 7));
}

#[test]
fn relinking_is_stable_and_the_user_can_say_not_this_ion() {
    let mut d = with_target().1.clone();
    let f = ms2_file("re.mzML", &[x(p_feature(), &[(TARGET_RT, 0.07, 5.0e4)])]);
    link(&mut d, "re.mzML", &f, &[]);
    let mut first = mzlab_core::json::Out::default();
    ms2link::json(&mut first, links(&d));
    // the same corrections again: the same entries and matches
    let (n, again) = (d.entries.len(), d.files.len());
    d.files.pop();
    link(&mut d, "re.mzML", &f, &[]);
    let mut second = mzlab_core::json::Out::default();
    ms2link::json(&mut second, links(&d));
    assert_eq!((d.entries.len(), d.files.len()), (n, again));
    assert_eq!(first.0, second.0);
    // not this ion: no row is made
    let no = Override {
        file: "re.mzML".into(),
        precursor: p_feature(),
        ce: Some(10.0),
        apex: TARGET_RT,
        row: None,
    };
    d.files.pop();
    link(&mut d, "re.mzML", &f, &[no]);
    assert!(targets(&d).is_empty());
    assert!(reasons(&d).contains(&"user"), "{:?}", links(&d).unmatched);
    // and back
    d.files.pop();
    link(&mut d, "re.mzML", &f, &[]);
    assert_eq!(targets(&d).len(), 1);
}
