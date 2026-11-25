//! Tests of `rawxic`: the verdict on hand-made ion counts (zero and small backgrounds, the edges of the study), the thresholds and their refusals,
//! the quantile, and the route on a synthetic index (rows, members, the same numbers as the integrator, a missing level).
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::*;
use crate::idxread::Index;
use crate::json::parse;
use crate::mzml::Spectrum;
use crate::mzmlwrite::{Header, MzmlWriter};

const Q: f64 = 200.0;
const WIN: (f64, f64) = (2.5, 3.0);

/// A trace of 200 scans every 0.02 min from 1.0 min: `ions(i)` ions of `Q` cps in scan `i`.
fn trace(ions: impl Fn(usize) -> f64) -> (Vec<f64>, Vec<f64>) {
    let rt: Vec<f64> = (0..200).map(|i| round(1.0 + i as f64 * 0.02, 4)).collect();
    let y = (0..200).map(|i| ions(i) * Q).collect();
    (rt, y)
}

/// The scan index of a time of `trace`.
fn at(t: f64) -> usize {
    ((t - 1.0) / 0.02).round() as usize
}

fn verdict(ions: impl Fn(usize) -> f64) -> Judged {
    let (rt, y) = trace(ions);
    judge(&rt, &y, WIN, Some(Q), &Thresholds::default())
}

#[test]
fn zero_background_edges_are_nothing_trace_trace_trace_peak() {
    let k = at(2.7);
    let one = |n: f64| verdict(move |i| if i == k { n } else { 0.0 });
    for (n, want) in [
        (0.0, "nothing"),
        (1.0, "nothing"),
        (2.0, "trace"),
        (3.0, "trace"),
        (5.0, "trace"),
    ] {
        let j = one(n);
        assert_eq!(j.verdict, want, "{n} ions: {}", j.rule);
    }
    // three ions per scan over five scans: a peak, with its numbers
    let j = verdict(|i| if (k..k + 5).contains(&i) { 3.0 } else { 0.0 });
    assert_eq!(j.verdict, "peak", "{}", j.rule);
    let c = j.counts.unwrap();
    assert_eq!((c.gross, c.net, c.run), (15.0, 15.0, 5));
    assert_eq!(c.background, 0.0);
    assert!((c.l_d - 3.0).abs() < 0.01, "{}", c.l_d);
    assert!(c.sn_floor.unwrap() >= 3.0);
}

#[test]
fn a_background_lifts_the_floor_and_the_same_ions_are_only_a_trace() {
    // one ion every 20 scans (0.05 per scan) everywhere but under the peak: two ions per scan over five scans
    let k = at(2.7);
    let j = verdict(|i| {
        if (k..k + 5).contains(&i) {
            2.0
        } else if i % 20 == 3 {
            1.0
        } else {
            0.0
        }
    });
    assert_eq!(j.verdict, "trace", "{}", j.rule);
    let c = j.counts.unwrap();
    assert!(c.background > 0.0 && c.background < 0.2, "{}", c.background);
    // the S/N on the floor is the height over one ion: 2, under min_sn 3
    assert!(c.sn_floor.unwrap() < 3.0, "{:?}", c.sn_floor);
    assert!(c.net >= c.l_d && c.run >= 3, "{c:?}");
    assert!(
        j.rule.contains("sn_floor") && j.rule.contains("< min_sn 3"),
        "{}",
        j.rule
    );
}

#[test]
fn thresholds_are_parameters_and_the_rule_names_them() {
    let k = at(2.7);
    let (rt, y) = trace(|i| if (k..k + 5).contains(&i) { 2.0 } else { 0.0 });
    let t = |min_sn, min_points| Thresholds {
        min_sn,
        min_points,
        ..Default::default()
    };
    assert_eq!(judge(&rt, &y, WIN, Some(Q), &t(3.0, 3)).verdict, "trace");
    let j = judge(&rt, &y, WIN, Some(Q), &t(2.0, 3));
    assert_eq!(j.verdict, "peak", "{}", j.rule);
    let j = judge(&rt, &y, WIN, Some(Q), &t(2.0, 6));
    assert_eq!(j.verdict, "trace", "{}", j.rule);
    assert!(j.rule.contains("run 5 < min_points 6"), "{}", j.rule);
}

#[test]
fn nothing_carries_an_upper_limit_and_no_scan_in_the_window_is_no_data() {
    let (rt, y) = trace(|_| 0.0);
    let j = judge(&rt, &y, WIN, Some(Q), &Thresholds::default());
    let c = j.counts.unwrap();
    assert!((c.ul - 3.0).abs() < 0.01, "{}", c.ul);
    let j = judge(&rt, &y, (9.0, 9.5), Some(Q), &Thresholds::default());
    assert_eq!((j.verdict, j.counts), ("no_data", None));
    assert!(j.rule.contains("no scan"), "{}", j.rule);
}

#[test]
fn without_a_quantum_the_pipeline_decides_and_the_flag_says_so() {
    let k = at(2.7);
    let (rt, y) = trace(|i| {
        if (k..k + 9).contains(&i) {
            5.0 - (i as f64 - (k + 4) as f64).abs()
        } else {
            0.0
        }
    });
    let j = judge(&rt, &y, WIN, None, &Thresholds::default());
    assert!(j.flags.contains(&"no_quantum"), "{:?}", j.flags);
    assert!(j.counts.is_none() && j.hit.is_some());
    assert!(
        j.rule.starts_with("no ion quantum: pipeline S/N"),
        "{}",
        j.rule
    );
    let j = judge(&rt, &vec![0.0; rt.len()], WIN, None, &Thresholds::default());
    assert_eq!(j.verdict, "nothing");
}

#[test]
fn the_window_without_sides_is_flagged_and_still_judged() {
    let (rt, y) = trace(|i| if i % 2 == 0 { 2.0 } else { 0.0 });
    let j = judge(&rt, &y, (1.0, 4.98), Some(Q), &Thresholds::default());
    assert!(j.flags.contains(&"no_background"), "{:?}", j.flags);
    assert_eq!(j.counts.unwrap().background, 0.0);
}

#[test]
fn the_normal_quantile_matches_the_tables() {
    for (alpha, z) in [
        (0.05, 1.644_854),
        (0.01, 2.326_348),
        (0.1, 1.281_552),
        (0.001, 3.090_232),
        (0.5, 0.0),
    ] {
        assert!((z_of(alpha) - z).abs() < 1e-5, "{alpha}: {}", z_of(alpha));
    }
}

/// An index of 120 MS1 scans every 0.02 min whose ions of 200 cps at m/z 250.3 form a peak of five scans at about 2.7 min.
fn mzidx() -> Vec<u8> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "r".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(200).unwrap();
    for i in 0..200 {
        let on = (at(2.7)..at(2.7) + 5).contains(&i);
        let s = Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt: 1.0 + i as f64 * 0.02,
            polarity: 1,
            mz: vec![150.3, 250.3],
            intensity: vec![200.0, if on { 600.0 } else { 0.0 }],
            ..Default::default()
        };
        w.spectrum(&s).unwrap();
    }
    w.close_spectra().unwrap();
    let b = w.finish().unwrap();
    let mut src = b.clone();
    let p = crate::upload::prepare(&mut src, b.len() as u64, "r.mzML").unwrap();
    crate::mzidx::build_mzml_to_vec(&p.table, &b, 64 << 20)
        .unwrap()
        .0
}

fn route(query: &str) -> Result<crate::json::Value> {
    let idx = mzidx();
    let n = idx.len() as u64;
    let mut r = Routes::new(Index::open(idx, n).unwrap());
    let t = crate::rawinfo::handle(&mut r, "/api/rawxic", &Query::parse(query))?.unwrap();
    Ok(parse(t.as_bytes()).unwrap())
}

#[test]
fn the_route_gives_a_peak_a_background_ion_and_an_empty_mass() {
    let v = route("mz=250.3,150.3,400&rt=2.5,3.0").unwrap();
    let rows = v.get("rows").unwrap().arr();
    let verdicts: Vec<_> = rows
        .iter()
        .map(|r| r.get("verdict").unwrap().str().unwrap())
        .collect();
    assert_eq!(verdicts, ["peak", "nothing", "nothing"], "{rows:?}");
    // the integrator's numbers are those of the table
    let p = &rows[0];
    assert!(p.get("area").unwrap().f64().unwrap() > 0.0);
    for k in [
        "height",
        "sn",
        "apex_rt",
        "start_rt",
        "end_rt",
        "points",
        "shape",
        "sn_floor",
        "counts_net",
        "l_c",
        "l_d",
        "z_counts",
        "flags",
        "rule",
    ] {
        assert!(p.get(k).is_some(), "{k}");
    }
    assert_eq!(p.get("sn_estimator").unwrap().str(), Some(SN_ESTIMATOR));
    assert!(rows[1].get("ul_counts").unwrap().f64().unwrap() > 0.0);
    assert_eq!(p.get("ul_counts"), Some(&crate::json::Value::Null));
    // the upper limit names the level it was computed at: the default alpha, and the one asked for
    assert_eq!(rows[1].get("ul_alpha").unwrap().f64(), Some(0.05));
    assert_eq!(p.get("ul_alpha"), Some(&crate::json::Value::Null));
    let w = route("mz=250.3,260&rt=2.5,3.0&alpha=0.01").unwrap();
    assert_eq!(
        w.get("rows").unwrap().arr()[1]
            .get("ul_alpha")
            .unwrap()
            .f64(),
        Some(0.01)
    );
    assert_eq!(
        v.get("ion_quantum").unwrap().get("cps").unwrap().f64(),
        Some(200.0)
    );
    assert_eq!(
        v.get("thresholds")
            .unwrap()
            .get("min_points")
            .unwrap()
            .f64(),
        Some(3.0)
    );
    assert_eq!(v.get("mz_offset"), Some(&crate::json::Value::Null));
}

#[test]
fn a_level_without_scans_is_no_data_and_bad_asks_are_refused_with_their_key() {
    let v = route("mz=250.3&rt=2.5,3.0&ms_level=2").unwrap();
    assert_eq!(
        v.get("rows").unwrap().arr()[0]
            .get("verdict")
            .unwrap()
            .str(),
        Some("no_data")
    );
    for bad in [
        "rt=2,3",
        "mz=250",
        "mz=x&rt=2,3",
        "mz=250&rt=3,2",
        "mz=250&rt=2,3&da=0",
        "mz=250&rt=2,3&alpha=0.9",
        "mz=250&rt=2,3&min_points=0",
        "mz=250&rt=2,3&min_sn=-1",
    ] {
        let e = route(bad).unwrap_err();
        assert_eq!(e.key, "err.raw.xic.param", "{bad}");
    }
    assert_eq!(
        route("mz=250&rt=2,3&mz_offset=abc").unwrap_err().key,
        "err.mz.offset.spec"
    );
}

/// A list longer than `MAX_MASSES` is refused with its count and the limit, never cut; the list of exactly `MAX_MASSES` is answered whole.
#[test]
fn too_many_masses_are_refused_not_cut() {
    let list = |n: usize| {
        (0..n)
            .map(|i| format!("{}", 200 + i))
            .collect::<Vec<_>>()
            .join(",")
    };
    let e = route(&format!("mz={}&rt=2,3", list(super::MAX_MASSES + 1))).unwrap_err();
    assert_eq!(e.key, "err.raw.xic.many");
    let ok = route(&format!("mz={}&rt=2,3", list(super::MAX_MASSES))).unwrap();
    assert_eq!(ok.get("rows").unwrap().arr().len(), super::MAX_MASSES);
}

#[test]
fn an_offset_moves_the_window_and_is_flagged() {
    // the ion sits 0.3 Da above 250.0: a 0.1 Da window misses it until the offset is told
    let miss = route("mz=250.0&rt=2.5,3.0&da=0.1").unwrap();
    assert_eq!(
        miss.get("rows").unwrap().arr()[0]
            .get("verdict")
            .unwrap()
            .str(),
        Some("nothing")
    );
    let hit = route("mz=250.0&rt=2.5,3.0&da=0.1&mz_offset=0.3").unwrap();
    let r = &hit.get("rows").unwrap().arr()[0];
    assert_eq!(r.get("verdict").unwrap().str(), Some("peak"), "{r:?}");
    assert!(r
        .get("flags")
        .unwrap()
        .arr()
        .iter()
        .any(|f| f.str() == Some("offset_applied")));
    assert_eq!(
        hit.get("mz_offset").unwrap().get("da").unwrap().f64(),
        Some(0.3)
    );
}
