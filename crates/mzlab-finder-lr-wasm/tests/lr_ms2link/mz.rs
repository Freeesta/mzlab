//! The m/z of the match and the isotope rule (`docs/studi/TABUI_MS2_LR.md` §4 rules 2-3): the precursor is compared with the m/z read in the table
//! too, and an isotope peak takes only its own peak of the trace.

use super::*;

const READ_RT: f64 = 5.8;
const OTHER_RT: f64 = 11.5;

/// Two invented ions present in every file with a sampling time: one read 0.33 Da above the extraction m/z of the hypothesis at 273.054 (a
/// dihydroxylation), one at 258.06 (an unexpected ion 1.003 Da above the hydroxylations, 4 min later than them).
fn extra(t: Option<f64>) -> Vec<(f64, f64, f64, f64)> {
    t.filter(|k| *k > 0.0).map_or(vec![], |_| {
        vec![
            (273.054 + 0.33, READ_RT, 0.07, 6.0e5),
            (258.06, OTHER_RT, 0.07, 6.0e5),
        ]
    })
}

fn with_ions() -> &'static (Vec<Vec<u8>>, lr::LrDig) {
    static BASE: OnceLock<(Vec<Vec<u8>>, lr::LrDig)> = OnceLock::new();
    BASE.get_or_init(|| {
        let (files, idxs) = series_with(&extra);
        let d = dig(&files, &idxs);
        (idxs, d)
    })
}

fn link(name: &str, x: X) -> lr::LrDig {
    let mut d = with_ions().1.clone();
    let f = ms2_file(name, &[x]);
    d.files.push(FileLr {
        name: name.into(),
        label: name.into(),
        time: None,
        kind_type: "sample".into(),
        kind: "ms2".into(),
    });
    let cols = &with_ions().0;
    let all: Vec<&[u8]> = cols
        .iter()
        .map(|c| c.as_slice())
        .chain([f.as_slice()])
        .collect();
    ms2link::ms2_link(&mut d, &[], &mut |k| Ok(open(all[k]))).unwrap();
    d
}

fn entry(d: &lr::LrDig, id: i64) -> &lr::Entry {
    d.entries.iter().find(|e| e.id == id).unwrap()
}

#[test]
fn a_precursor_set_on_the_m_z_read_in_the_table_matches_the_row() {
    let d = with_ions().1.clone();
    let r = entry(&d, 9);
    let read = r.observed.unwrap();
    assert!(
        read - r.mz_x > 0.3 && (r.ref_rt.unwrap() - READ_RT).abs() < 0.1,
        "the row is not read above the extraction m/z"
    );
    let p = read + 0.18;
    assert!((p - r.mz_x).abs() > 0.5 && (p - r.mz).abs() > 0.5);
    let d = link("read.mzML", x(p, &[(READ_RT, 0.07, 5.0e4)]));
    let l = d.links.as_ref().unwrap();
    assert_eq!(l.links.len(), 1, "{:?} {:?}", l.links, l.unmatched);
    assert_eq!((l.links[0].row, l.links[0].state), (9, State::Auto));
    assert!((l.links[0].dmz - 0.18).abs() < 0.02, "{}", l.links[0].dmz);
}

#[test]
fn an_isotope_peak_takes_only_its_own_peak_of_the_trace() {
    let d = with_ions().1.clone();
    let p = entry(&d, 7).observed.unwrap() + 1.00335;
    let comp = entry(&d, 67);
    assert!(
        comp.observed.is_some_and(|o| (p - o).abs() <= 0.5) && comp.ref_rt.unwrap() > 11.0,
        "no row compatible with the precursor 4 min after the isotope"
    );
    let d = link(
        "iso2.mzML",
        x(p, &[(7.4, 0.07, 5.0e4), (OTHER_RT, 0.07, 5.0e4)]),
    );
    let l = d.links.as_ref().unwrap();
    let state_of = |row: i64| l.links.iter().find(|k| k.row == row).map(|k| k.state);
    assert_eq!(
        (state_of(7), state_of(67)),
        (Some(State::Isotope), Some(State::Auto)),
        "{:?} {:?}",
        l.links,
        l.unmatched
    );
}
