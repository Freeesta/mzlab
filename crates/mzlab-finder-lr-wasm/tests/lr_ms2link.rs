//! The MS2 matches of the low-resolution Dig (`lr::ms2link`) on the synthetic series: one test per case of the table of `docs/studi/TABUI_MS2_LR.md`
//! §8. The MS2 files are written here: traces over the whole run (a constant level with noise plus Gaussian peaks at given times), as the 3200 writes
//! the experiments of a product-ion injection.

use std::sync::OnceLock;

use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::upload::prepare;
use mzlab_feat::legacy::lr::ms2link::{self, Links, Override, State};
use mzlab_feat::legacy::lr::plan::FileLr;
use mzlab_feat::legacy::lr::{self, detail::detail, ms2::pool};
use serde_json::Value;

mod comune;
use comune::*;
#[path = "lr_ms2link/sim.rs"]
mod sim;
#[path = "lr_ms2link/target.rs"]
mod target;

/// One experiment of an MS2 file: a scan every `step` min over `0.5..14`, the TIC `LEVEL` plus the Gaussian `peaks` (apex, sigma, height).
struct X {
    precursor: f64,
    ce: Option<f64>,
    polarity: i8,
    peaks: Vec<(f64, f64, f64)>,
    step: f64,
}

const LEVEL: f64 = 300.0;

fn x(precursor: f64, peaks: &[(f64, f64, f64)]) -> X {
    X {
        precursor,
        ce: Some(10.0),
        polarity: 1,
        peaks: peaks.to_vec(),
        step: 0.02,
    }
}

/// The fragments of every scan: 17 and 44 Da under the precursor, in the ratio 1 : 0.5.
const LOSSES: [(f64, f64); 2] = [(44.0, 0.5), (17.0, 1.0)];

fn ms2_file(name: &str, xs: &[X]) -> Vec<u8> {
    let mut rng = Lcg(5);
    let mut cycles: Vec<(f64, usize)> = Vec::new();
    for (j, e) in xs.iter().enumerate() {
        let n = ((14.0 - 0.5) / e.step) as usize;
        cycles.extend((0..n).map(|i| (0.5 + e.step * i as f64, j)));
    }
    cycles.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: name.into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(cycles.len()).unwrap();
    for (i, (rt, j)) in cycles.iter().enumerate() {
        let e = &xs[*j];
        let top: f64 = e
            .peaks
            .iter()
            .map(|(c, s, a)| a * (-0.5 * ((rt - c) / s).powi(2)).exp())
            .sum();
        let tic = (LEVEL + top) * (1.0 + 0.03 * rng.normal());
        w.spectrum(&Spectrum {
            index: i,
            id: format!("sample=1 period=1 cycle={} experiment={}", i + 1, j + 2),
            level: 2,
            rt: *rt,
            polarity: e.polarity,
            profile: false,
            filter: "+ EPI".into(),
            precursor: Some(e.precursor),
            mz: LOSSES.iter().map(|l| e.precursor - l.0).collect(),
            intensity: LOSSES.iter().map(|l| (tic * l.1) as f32).collect(),
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    let mzml = String::from_utf8(w.finish().unwrap()).unwrap();
    // the writer has no collision energy: the nth precursor gets the energy of the nth scan
    let mut out = String::new();
    for (k, part) in mzml.split("</selectedIonList></precursor>").enumerate() {
        out += part;
        if let Some((_, j)) = cycles.get(k) {
            out += "</selectedIonList>";
            if let Some(ce) = xs[*j].ce {
                out += &format!("<activation><cvParam cvRef=\"MS\" accession=\"MS:1000045\" name=\"collision energy\" value=\"{ce}\"/></activation>");
            }
            out += "</precursor>";
        }
    }
    let bytes = out.into_bytes();
    let mut src = bytes.clone();
    let p = prepare(&mut src, bytes.len() as u64, name).unwrap();
    build_mzml_to_vec(&p.table, &bytes, 64 << 20).unwrap().0
}

/// The Dig of the series without MS2 files, computed once.
fn base() -> &'static (Vec<Vec<u8>>, lr::LrDig) {
    static BASE: OnceLock<(Vec<Vec<u8>>, lr::LrDig)> = OnceLock::new();
    BASE.get_or_init(|| {
        let (files, idxs) = series();
        let d = dig(&files, &idxs);
        (idxs, d)
    })
}

fn row(d: &lr::LrDig, id: i64) -> &lr::Entry {
    d.entries.iter().find(|e| e.id == id).unwrap()
}

/// `d` with MS2 files `(name, time, index)` and the matches (`ov` = corrections).
fn matched(mut d: lr::LrDig, ms2: &[(&str, Option<f64>, Vec<u8>)], ov: &[Override]) -> lr::LrDig {
    for (name, time, _) in ms2 {
        d.files.push(FileLr {
            name: name.to_string(),
            label: name.to_string(),
            time: *time,
            kind_type: "sample".into(),
            kind: "ms2".into(),
        });
    }
    let cols = &base().0;
    let all: Vec<&Vec<u8>> = cols.iter().chain(ms2.iter().map(|m| &m.2)).collect();
    ms2link::ms2_link(&mut d, ov, &mut |k| Ok(open(all[k]))).unwrap();
    d
}

fn one(d: lr::LrDig, name: &str, x: X) -> lr::LrDig {
    let f = ms2_file(name, &[x]);
    matched(d, &[(name, None, f)], &[])
}

fn scans_of(d: &lr::LrDig, id: i64, ms2: &[u8]) -> usize {
    let cols = &base().0;
    let mut op = |k: usize| {
        Ok(if k < cols.len() {
            open(&cols[k])
        } else {
            open(ms2)
        })
    };
    pool(d, row(d, id), 6, &mut op).unwrap().scans
}

fn links(d: &lr::LrDig) -> &Links {
    d.links.as_ref().unwrap()
}

fn reasons(d: &lr::LrDig) -> Vec<&'static str> {
    links(d).unmatched.iter().map(|u| u.reason).collect()
}

/// P that a student would set for the row `id`: its calculated m/z read as the instrument shows it (+0.3).
fn p_of(d: &lr::LrDig, id: i64) -> f64 {
    (row(d, id).mz_x * 10.0).round() / 10.0
}

#[test]
fn a_peak_at_the_time_of_the_row_is_its_match_and_gives_its_fragments() {
    let d = base().1.clone();
    let p = p_of(&d, 7);
    let f = ms2_file("a.mzML", &[x(p, &[(7.4, 0.07, 5.0e4)])]);
    let d = matched(d, &[("a.mzML", None, f.clone())], &[]);
    let l = links(&d);
    assert_eq!((l.links.len(), l.links[0].row), (1, 7), "{:?}", l.links);
    assert_eq!(l.links[0].state, State::Auto);
    assert!((l.exps[0].peaks[l.links[0].peak].apex - 7.4).abs() < 0.05);
    let cols = &base().0;
    let mut op = |k: usize| {
        Ok(if k < cols.len() {
            open(&cols[k])
        } else {
            open(&f)
        })
    };
    let m = pool(&d, row(&d, 7), 6, &mut op).unwrap();
    assert!((10..=40).contains(&m.scans), "{}", m.scans);
    assert_eq!(m.frags.len(), 2);
    assert!((m.frags[0].mz - (p - 17.0)).abs() < 0.01 && m.frags[0].rel == 100.0);
    assert!((m.frags[1].mz - (p - 44.0)).abs() < 0.01);
    // no other row got a scan
    assert_eq!(scans_of(&d, 10007, &f), 0);
    assert!(l.unmatched.is_empty(), "{:?}", l.unmatched);
}

#[test]
fn a_flat_trace_is_no_signal_and_gives_no_scans() {
    let d = base().1.clone();
    let p = p_of(&d, 100001);
    let f = ms2_file("flat.mzML", &[x(p, &[])]);
    let d = matched(d, &[("flat.mzML", None, f.clone())], &[]);
    assert!(links(&d).links.is_empty());
    assert_eq!(reasons(&d), ["no_signal"]);
    assert_eq!(links(&d).unmatched[0].rows, [100001]);
    assert_eq!(scans_of(&d, 100001, &f), 0);
}

#[test]
fn two_isomers_and_two_peaks_give_one_peak_to_each_row() {
    let d = base().1.clone();
    let p = p_of(&d, 7);
    assert_eq!(p, p_of(&d, 10007));
    let f = ms2_file(
        "iso.mzML",
        &[x(p, &[(7.4, 0.07, 5.0e4), (8.3, 0.07, 4.0e4)])],
    );
    let d = matched(d, &[("iso.mzML", None, f)], &[]);
    let l = links(&d);
    let mut got: Vec<(i64, f64)> = l
        .links
        .iter()
        .map(|k| (k.row, l.exps[0].peaks[k.peak].apex))
        .collect();
    got.sort_by_key(|a| a.0);
    assert_eq!(got.len(), 2, "{:?}", l.links);
    assert_eq!((got[0].0, got[1].0), (7, 10007));
    assert!((got[0].1 - 7.4).abs() < 0.05 && (got[1].1 - 8.3).abs() < 0.05);
    assert!(l.links.iter().all(|k| k.state == State::Auto));
}

#[test]
fn two_rows_and_one_peak_are_ambiguous_with_the_alternatives() {
    let mut d = base().1.clone();
    let mut near = row(&d, 7).clone();
    (near.id, near.ref_rt) = (999, Some(7.55));
    d.entries.push(near);
    let p = p_of(&d, 7);
    let d = one(d, "amb.mzML", x(p, &[(7.45, 0.07, 5.0e4)]));
    let l = links(&d);
    assert_eq!(l.links.len(), 1, "{:?}", l.links);
    assert_eq!((l.links[0].row, l.links[0].state), (7, State::Ambiguous));
    assert_eq!(l.links[0].alternatives, [999]);
}

#[test]
fn a_peak_at_the_time_of_the_parent_with_a_lighter_precursor_is_not_a_product() {
    let mut d = base().1.clone();
    let p = p_of(&d, 18);
    let rt = row(&d, 0).ref_rt.unwrap();
    // the product-like row 18 coelutes with the parent in this series: not in-source (a candidate, not an unexpected ion)
    d.entries.iter_mut().find(|e| e.id == 18).unwrap().ref_rt = Some(rt + 0.02);
    let d = one(d, "parent.mzML", x(p, &[(rt, 0.07, 5.0e4)]));
    assert!(links(&d).links.is_empty(), "{:?}", links(&d).links);
    assert!(
        reasons(&d).contains(&"parent_rt"),
        "{:?}",
        links(&d).unmatched
    );
}

#[test]
fn the_isotope_peak_of_a_row_is_shown_but_gives_no_scans() {
    let d = base().1.clone();
    let p = ((row(&d, 89).mz_x + 1.00335) * 10.0).round() / 10.0;
    let f = ms2_file("iso13.mzML", &[x(p, &[(6.8, 0.07, 5.0e4)])]);
    let d = matched(d, &[("iso13.mzML", None, f.clone())], &[]);
    let l = links(&d);
    assert_eq!(l.links.len(), 1, "{:?} {:?}", l.links, l.unmatched);
    assert_eq!((l.links[0].row, l.links[0].state), (89, State::Isotope));
    assert_eq!(scans_of(&d, 89, &f), 0);
}

#[test]
fn an_experiment_of_the_other_polarity_does_not_match() {
    let d = base().1.clone();
    let p = p_of(&d, 7);
    let mut e = x(p, &[(7.4, 0.07, 5.0e4)]);
    e.polarity = -1;
    let d = one(d, "neg.mzML", e);
    assert!(links(&d).links.is_empty());
    assert_eq!(reasons(&d), ["polarity"]);
}

#[test]
fn two_collision_energies_are_two_matches_and_the_biggest_gives_the_pool() {
    let d = base().1.clone();
    let p = p_of(&d, 7);
    let mut slow = x(p, &[(7.4, 0.07, 5.0e4)]);
    (slow.ce, slow.step) = (Some(30.0), 0.04);
    let f = ms2_file("ce.mzML", &[x(p, &[(7.4, 0.07, 5.0e4)]), slow]);
    let d = matched(d, &[("ce.mzML", None, f.clone())], &[]);
    let l = links(&d);
    assert_eq!(l.links.len(), 2, "{:?}", l.links);
    assert_eq!(l.of(7).count(), 2);
    let cols = &base().0;
    let mut op = |k: usize| {
        Ok(if k < cols.len() {
            open(&cols[k])
        } else {
            open(&f)
        })
    };
    let m = pool(&d, row(&d, 7), 6, &mut op).unwrap();
    assert_eq!(m.collision_energy, Some(10.0));
    let det: Value = serde_json::from_str(&detail(&d, 7, &mut op).unwrap()).unwrap();
    assert_eq!(det["ms2"]["collision_energy"], 10.0);
    let ls = det["ms2_links"].as_array().unwrap();
    assert_eq!(ls.len(), 2);
    assert!(ls
        .iter()
        .all(|k| k["state"] == "auto" && k["fragments"].as_array().unwrap().len() == 2));
}

#[test]
fn the_drift_of_a_file_is_measured_and_reported() {
    let d = base().1.clone();
    let xs: Vec<X> = [(89, 6.8), (100001, 5.2), (11, 4.4)]
        .iter()
        .map(|(id, rt)| x(p_of(&d, *id), &[(rt + 0.1, 0.07, 1.0e5)]))
        .collect();
    let f = ms2_file("drift.mzML", &xs);
    let d = matched(d, &[("drift.mzML", None, f)], &[]);
    let l = links(&d);
    assert_eq!(l.links.len(), 3, "{:?} {:?}", l.links, l.unmatched);
    for e in &l.exps {
        assert!((e.drift - 0.1).abs() < 0.04, "{}", e.drift);
    }
    for k in &l.links {
        assert!((k.drt - 0.1).abs() < 0.05, "{k:?}");
    }
}

#[test]
fn the_corrections_of_the_user_beat_the_rules() {
    let d = base().1.clone();
    let p = p_of(&d, 7);
    let f = ms2_file(
        "user.mzML",
        &[x(p, &[(7.4, 0.07, 5.0e4), (8.3, 0.07, 4.0e4)])],
    );
    let ms2 = [("user.mzML", None, f.clone())];
    let ov = |apex: f64, row: Option<i64>| Override {
        file: "user.mzML".into(),
        precursor: p,
        ce: Some(10.0),
        apex,
        row,
    };
    // the peak at 7.4 goes to the other isomer
    let a = matched(d.clone(), &ms2, &[ov(7.4, Some(10007))]);
    let l = links(&a);
    let to = |l: &Links, id: i64| l.links.iter().filter(|k| k.row == id).count();
    assert_eq!((to(l, 7), to(l, 10007)), (0, 2), "{:?}", l.links);
    assert!(l.links.iter().any(|k| k.state == State::User));
    // to no row: the row loses its scans
    let b = matched(d.clone(), &ms2, &[ov(7.4, None)]);
    assert_eq!(scans_of(&b, 7, &f), 0);
    assert!(reasons(&b).contains(&"user"));
    // a correction without its peak any more is listed as stale
    let c = matched(d, &ms2, &[ov(12.0, Some(7))]);
    assert!(reasons(&c).contains(&"stale"), "{:?}", links(&c).unmatched);
    assert_eq!(links(&c).links.len(), 2);
}

#[test]
fn the_json_has_the_contract_and_two_runs_give_the_same_one() {
    let d = base().1.clone();
    let p = p_of(&d, 7);
    let f = ms2_file("json.mzML", &[x(p, &[(7.4, 0.07, 5.0e4)])]);
    let ms2 = [("json.mzML", Some(15.0), f)];
    let (a, b) = (matched(d.clone(), &ms2, &[]), matched(d, &ms2, &[]));
    let (mut oa, mut ob) = (
        mzlab_core::json::Out::default(),
        mzlab_core::json::Out::default(),
    );
    ms2link::json(&mut oa, links(&a));
    ms2link::json(&mut ob, links(&b));
    assert_eq!(oa.0, ob.0);
    let v: Value = serde_json::from_str(&oa.0).unwrap();
    assert_eq!(v["schema"], "ms2_links/1");
    assert_eq!(
        (v["prec_tol"].as_f64(), v["rt_tol"].as_f64()),
        (Some(0.5), Some(0.25))
    );
    let e = &v["experiments"][0];
    for k in [
        "file",
        "time",
        "polarity",
        "precursor",
        "ce",
        "scans",
        "rt",
        "drift",
        "peaks",
    ] {
        assert!(e.get(k).is_some(), "{k}");
    }
    assert_eq!(
        (e["time"].as_f64(), e["polarity"].as_i64(), e["ce"].as_f64()),
        (Some(15.0), Some(1), Some(10.0))
    );
    for k in [
        "exp",
        "peak",
        "row",
        "state",
        "dmz",
        "drt",
        "alternatives",
        "why",
    ] {
        assert!(v["links"][0].get(k).is_some(), "{k}");
    }
    assert_eq!(v["parent"]["has_ms2"], false);
    let s: Value = serde_json::from_str(&lr::summary::summary(&a, "{}")).unwrap();
    assert_eq!(s["ms2_links"], v);
    let n = |id: i64| {
        s["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == id)
            .unwrap()["ms2"]
            .as_i64()
    };
    assert_eq!((n(7), n(10007), n(0)), (Some(1), Some(0), Some(0)));
}
