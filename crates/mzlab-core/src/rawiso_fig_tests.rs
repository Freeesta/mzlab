//! Tests of the figure of `raw isotopes`: a synthetic peak with known ratios gives a figure that holds the words, the numbers and the verdicts of its
//! `result`, the same bytes every time, and every text key it uses has a text in `lang/en/figure-raw.js`.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::*;
use crate::idxread::Index;
use crate::rawiso::tests::mzidx;
use crate::routes::{Query, Routes};

const Q: &str = "mz=181.07&rt=2.0,2.4";

fn figure(idx: Vec<u8>, query: &str) -> String {
    let n = idx.len() as u64;
    let mut r = Routes::new(Index::open(idx, n).unwrap());
    crate::rawinfo::handle(
        &mut r,
        "/api/rawiso",
        &Query::parse(&format!("{query}&figure=1")),
    )
    .unwrap()
    .unwrap()
}

#[test]
fn a_known_pattern_is_drawn_with_its_ratios_sigma_and_verdicts() {
    // 100 / 7 / 1 ions per scan over 21 scans: R1 = 0.07, R2 = 0.01; glucose [M+H]+ expects about 0.069 and 0.013
    let svg = figure(mzidx(100, 7, 1, false), &format!("{Q}&formula=C6H12O6"));
    assert!(svg.starts_with("<svg ") && svg.ends_with("</svg>\n"));
    assert!(svg.contains("Isotope pattern at m/z 181.07, 2 to 2.4 min"));
    assert!(svg.contains("C6H12O6 [M+H]+, expected ion m/z"));
    assert!(
        svg.contains("M (m/z 181)")
            && svg.contains("M+1 (m/z 182)")
            && svg.contains("M+2 (m/z 183)")
    );
    assert_eq!(svg.matches("<polyline").count(), 3);
    assert!(svg.contains("Peak region "), "{svg}");
    assert!(svg.contains("R1 = N(M+1) / N(M)") && svg.contains("R2 = N(M+2) / N(M)"));
    assert!(svg.contains("observed 0.07 ± "), "{svg}");
    assert!(svg.contains("expected 0.0"));
    assert_eq!(svg.matches(">consistent<").count(), 2, "{svg}");
}

#[test]
fn a_ratio_far_from_the_formula_is_inconsistent_and_without_a_formula_nothing_is_expected() {
    let bad = figure(mzidx(100, 30, 1, false), &format!("{Q}&formula=C6H12O6"));
    assert!(bad.contains(">inconsistent<"), "{bad}");
    let none = figure(mzidx(100, 7, 1, false), Q);
    assert!(none.contains("No formula given: observed ratios only"));
    assert!(!none.contains("expected 0") && !none.contains("consistent"));
    assert!(none.contains("observed 0.07 ± "));
}

#[test]
fn the_figure_is_one_well_formed_deterministic_document() {
    let a = figure(mzidx(100, 7, 1, false), &format!("{Q}&formula=C6H12O6"));
    let b = figure(mzidx(100, 7, 1, false), &format!("{Q}&formula=C6H12O6"));
    assert_eq!(a, b);
    let mut rd = quick_xml::Reader::from_str(&a);
    loop {
        match rd.read_event().unwrap() {
            quick_xml::events::Event::Eof => break,
            _ => continue,
        }
    }
}

#[test]
fn a_window_without_scans_still_answers_a_figure() {
    let svg = figure(mzidx(100, 7, 1, false), "mz=181.07&rt=50,60");
    assert!(svg.contains("No scans in the window"), "{svg}");
    assert_eq!(svg.matches("<polyline").count(), 0);
}

#[test]
fn figure_takes_only_one_and_every_text_key_has_a_text() {
    let n = mzidx(100, 7, 1, false);
    let len = n.len() as u64;
    let mut r = Routes::new(Index::open(n, len).unwrap());
    let e = crate::rawinfo::handle(
        &mut r,
        "/api/rawiso",
        &Query::parse(&format!("{Q}&figure=yes")),
    );
    assert_eq!(
        e.err().map(|e| e.key.to_string()).as_deref(),
        Some("err.raw.isotopes.param")
    );
    let src = include_str!("rawiso_fig.rs");
    let keys: Vec<&str> = src
        .split('"')
        .filter(|s| s.starts_with("note.rawiso.fig.") && !s.contains(' ') && !s.ends_with('.'))
        .collect();
    assert!(keys.len() >= 20);
    for k in keys {
        assert_ne!(tx(k, &[]), k, "no text for {k}");
    }
}
