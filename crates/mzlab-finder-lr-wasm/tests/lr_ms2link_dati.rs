//! The MS2 matches on the real 3200 series of the private data repository (skipped without it): the experiments, their traces and the rows they
//! do and do not get. Every number comes from its `scratchpad/tabui_ms2_lr_truth.json`; none is written here.

use mzlab_feat::legacy::lr::ms2::pool;
use mzlab_feat::legacy::lr::ms2link::{Links, State};
use mzlab_feat::legacy::lr::{Entry, LrDig};
use serde_json::Value;

mod comune;
use comune::*;

#[test]
fn the_experiments_of_the_real_series_are_found_and_matched_as_the_truth_says() {
    let Some(dati) = root() else {
        eprintln!("SKIP lr_ms2link_dati: no data repository (MZLAB_DATI)");
        return;
    };
    let truth: Value = serde_json::from_str(
        &std::fs::read_to_string(dati.join("scratchpad/tabui_ms2_lr_truth.json")).unwrap(),
    )
    .unwrap();
    let full: Vec<String> = truth["full_files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap().into())
        .collect();
    let ms2: Vec<(String, f64)> = truth["ms2_files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["mzml"].as_str().unwrap().to_string(),
                f["time"].as_f64().unwrap(),
            )
        })
        .collect();
    let time = |name: &str| {
        name.rsplit("-t")
            .next()
            .and_then(|t| {
                t.split(|c: char| !c.is_ascii_digit())
                    .next()?
                    .parse::<f64>()
                    .ok()
            })
            .unwrap()
    };
    let mut files = Vec::new();
    let mut idxs = Vec::new();
    for (name, t, kind) in full
        .iter()
        .map(|n| (n, time(n), "full"))
        .chain(ms2.iter().map(|(n, t)| (n, *t, "ms2")))
    {
        files.push(format!(
            r#"{{"name": "{name}", "time": {t}, "type": "sample", "kind": "{kind}"}}"#
        ));
        idxs.push(index_of_file(
            &dati_percorsi::percorso(&format!("dati/3200 qtrap/mzML/{name}")).unwrap(),
        ));
    }
    let files = format!("[{}]", files.join(","));
    let parent = format!(
        r#"{{"neutral": "{}"}}"#,
        truth["parent"]["formula"].as_str().unwrap()
    );
    let n = full.len();
    let rest: Vec<&Vec<u8>> = idxs[n..].iter().collect();
    let d = linked(dig_for(&files, &parent, &idxs[..n]), &idxs[..n], &rest);
    let l = d.links.as_ref().unwrap();
    let exps = truth["experiments"].as_array().unwrap();
    assert_eq!(l.exps.len(), exps.len());
    for t in exps {
        check(&d, t);
    }
    assert!(!l.parent_has_ms2);
    assert_eq!(l.links.iter().filter(|k| k.row == 0).count(), 0);
    no_signal_pool(&truth, &d, &idxs);
}

/// The row made from the MS2 (`ms2_target`) that the peak `near` of experiment `i` got.
fn target_row<'a>(d: &'a LrDig, i: usize, near: &dyn Fn(usize) -> bool) -> Option<&'a Entry> {
    let l = d.links.as_ref()?;
    l.links
        .iter()
        .filter(|k| k.exp == i && near(k.peak))
        .find_map(|k| d.entries.iter().find(|e| e.id == k.row && e.ms2_target))
}

/// One experiment of the truth against what the engine found for it. `target` and `isotope` are the ions that the Dig does not list as a product of
/// its own: a `target` peak is matched (`auto`, `ambiguous` or `user`) to a row of any kind (Dig, hypothesis or MS2 target) whose reference RT is
/// within 0.1 min of the apex in the full scan; an `isotope` peak keeps the link of the isotope rule.
fn check(d: &LrDig, t: &Value) {
    let l = d.links.as_ref().unwrap();
    let (file, p) = (
        t["mzml"].as_str().unwrap(),
        t["precursor"].as_f64().unwrap(),
    );
    let (i, e) = l
        .exps
        .iter()
        .enumerate()
        .find(|(_, e)| e.file == file && (e.precursor - p).abs() < 0.06)
        .expect("the experiment");
    assert_eq!(
        (e.ce, e.polarity),
        (t["ce"].as_f64(), if t["polarity"] == "+" { 1 } else { -1 }),
        "{file} {p}"
    );
    let apex = t["trace_apex"].as_f64().unwrap();
    assert!(
        e.peaks.iter().any(|k| (k.apex - apex).abs() <= 0.1),
        "{file} {p}: {:?}",
        e.peaks
    );
    if t["expect"] == "no_signal" {
        // the trace is flat at the time of the row of the Dig: no link to it (the early maximum may make rows of its own)
        let rt = t["no_signal_row_rt"].as_f64().unwrap();
        let at_rt = |id: i64| {
            d.entries
                .iter()
                .find(|e| e.id == id)
                .and_then(|e| e.ref_rt)
                .is_some_and(|r| (r - rt).abs() < 0.05)
        };
        assert!(
            !l.links.iter().any(|k| k.exp == i && at_rt(k.row)),
            "{file} {p}: {:?}",
            l.links
        );
        return;
    }
    let near = |k: usize| (e.peaks[k].apex - apex).abs() <= 0.1;
    let isotope = l
        .links
        .iter()
        .any(|k| k.exp == i && near(k.peak) && k.state == State::Isotope);
    if t["expect"] == "isotope" {
        assert!(isotope, "{file} {p}: {:?} {:?}", l.links, l.unmatched);
        return;
    }
    let at = t["full_apex"].as_f64().unwrap_or(apex);
    let row_at = |id: i64| {
        d.entries
            .iter()
            .find(|e| e.id == id)
            .and_then(|e| e.ref_rt)
            .is_some_and(|r| (r - at).abs() <= 0.1)
    };
    let matched = l.links.iter().any(|k| {
        k.exp == i
            && near(k.peak)
            && matches!(k.state, State::Auto | State::Ambiguous | State::User)
            && row_at(k.row)
    });
    assert!(matched, "{file} {p}: {:?} {:?}", l.links, l.unmatched);
    let made = target_row(d, i, &near);
    if let Some(e) = made {
        // the cell of the full-scan file with the same sampling time is not empty
        let k =
            d.lc.iter()
                .position(|f| f.time == l.exps[i].time && f.time.is_some());
        assert!(
            k.is_some_and(|k| e.cells[k].detected || e.cells[k].area > 0.0),
            "{file} {p}: {:?}",
            e.cells
        );
    }
}

/// A row at the time of an experiment without signal gets no scan in its pool.
fn no_signal_pool(truth: &Value, d: &LrDig, idxs: &[Vec<u8>]) {
    let none = truth["experiments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["expect"] == "no_signal")
        .unwrap();
    let rt = none["no_signal_row_rt"].as_f64().unwrap();
    let row = d
        .entries
        .iter()
        .find(|e| e.unexpected && e.ref_rt.is_some_and(|r| (r - rt).abs() < 0.05))
        .expect("the row of the truth");
    let mut op = |k: usize| Ok(open(&idxs[k]));
    assert_eq!(pool(d, row, 6, &mut op).unwrap().scans, 0);
}
