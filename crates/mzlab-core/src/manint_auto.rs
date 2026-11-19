//! «Integrate peaks in all selected files»: the engine finds every peak of every SIM/MRM channel of every file (`peakq_lrauto::lr_auto_integrate`), puts the
//! peaks of one channel that fall at the same time in different files into one group (apex within `rt_tol` of the group's mean) and returns them as the windows
//! `manint::table_text` reads, plus a log per file. A peak alone in its group, while another file holds the channel, is marked `unmatched`. The page's
//! `api/manauto`, the app op `auto` and `mzlab-cli peak table --set auto=true` (plus `--set params=PATH`) are these bytes.

use super::{key_of, parse, samples_and_keys, Key, Result, Value};
use crate::peakq_lrauto::{lr_auto_integrate, LrAutoParams};

/// Two peaks of one channel are the same peak when their apexes are this close (min).
pub const RT_TOL: f64 = 0.10;

/// One peak found in one file: where it is in the request and what the window needs.
struct Found {
    file: usize,
    tr: usize,
    key: Key,
    a: f64,
    b: f64,
    apex: f64,
    line: [f64; 4],
    sn: f64,
    group: usize,
}

/// The engine's numbers from the request `{"rt_tol", "half", "valley" (%), "sn", "min_pts", "despike", "split": "drop"|"valley"}`; a missing one keeps its default.
fn params_of(p: &Value) -> (LrAutoParams, f64) {
    let num = |k: &str| p.get(k).and_then(Value::f64).unwrap_or(-1.0);
    let split = match p.get("split").and_then(Value::str) {
        Some("valley") => 1.0,
        _ => 0.0,
    };
    let tol = num("rt_tol");
    let despike = p
        .get("despike")
        .and_then(Value::bool)
        .map_or(-1.0, f64::from);
    let lr = LrAutoParams::from_slice(&[
        num("half"),
        num("valley"),
        num("sn"),
        num("min_pts"),
        despike,
        split,
    ]);
    (lr, if tol > 0.0 { tol } else { RT_TOL })
}

/// The peaks of one channel, from every file that holds it, in groups of apexes within `tol` of the group's mean (a file gives a group one peak).
fn group(found: &mut [Found], tol: f64) -> Vec<f64> {
    let mut order: Vec<usize> = (0..found.len()).collect();
    crate::sortc::by(&mut order, |&i, &j| found[i].apex.total_cmp(&found[j].apex));
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for i in order {
        let fits = |g: &Vec<usize>| {
            let mean = g.iter().map(|&k| found[k].apex).sum::<f64>() / g.len() as f64;
            (found[i].apex - mean).abs() <= tol && g.iter().all(|&k| found[k].file != found[i].file)
        };
        match groups.iter().rposition(fits) {
            Some(g) => groups[g].push(i),
            None => groups.push(vec![i]),
        }
    }
    groups
        .iter()
        .enumerate()
        .map(|(n, g)| {
            for &i in g {
                found[i].group = n;
            }
            g.iter().map(|&k| found[k].apex).sum::<f64>() / g.len() as f64
        })
        .collect()
}

/// The windows of every peak of every channel of every file, grouped, and the log of each file, as JSON:
/// `{"tol", "windows": [{"file", "tr", "a", "b", "baseline": "line", "line": [t0, y0, t1, y1], "grp", "origin": "auto"|"unmatched", "sn"}], "log": [{"file", "tr", "peaks": [apex...]}], "groups", "unmatched"}`.
/// `grp` is the mean apex of the group (min): the table orders the rows of a channel by it.
pub fn auto_text(files: &str, params: &str) -> Result<String> {
    let (samples, keys) = samples_and_keys(files)?;
    let (lr, tol) = params_of(&parse(
        if params.trim().is_empty() {
            "{}"
        } else {
            params
        }
        .as_bytes(),
    )?);
    let mut found: Vec<Found> = Vec::new();
    let mut log: Vec<String> = Vec::new();
    for (file, (_, trs, _)) in samples.iter().enumerate() {
        for tr in trs.iter().filter(|t| t.t.len() >= 2) {
            let last = tr.t[tr.t.len() - 1];
            let r = lr_auto_integrate(&tr.t, &tr.y, tr.t[0], last, lr);
            let apexes: Vec<String> = r
                .peaks
                .iter()
                .map(|p| format!("{}", tr.t[p.apex]))
                .collect();
            log.push(format!(
                "{{\"file\":{file},\"tr\":{},\"peaks\":[{}]}}",
                tr.pos,
                apexes.join(",")
            ));
            for p in &r.peaks {
                found.push(Found {
                    file,
                    tr: tr.pos,
                    key: key_of(tr),
                    a: tr.t[p.lo],
                    b: tr.t[p.hi],
                    apex: tr.t[p.apex],
                    line: [tr.t[p.lo], p.base_lo, tr.t[p.hi], p.base_hi],
                    sn: p.sn,
                    group: 0,
                });
            }
        }
    }
    let mut out: Vec<String> = Vec::new();
    let (mut n_groups, mut n_alone) = (0, 0);
    for (k, _, _) in &keys {
        let mut mine: Vec<Found> = found.iter().filter(|f| f.key == *k).map(copy).collect();
        let held = samples
            .iter()
            .filter(|s| s.1.iter().any(|t| key_of(t) == *k))
            .count();
        let means = group(&mut mine, tol);
        n_groups += means.len();
        for f in &mine {
            let alone = held > 1 && mine.iter().filter(|g| g.group == f.group).count() == 1;
            n_alone += usize::from(alone);
            out.push(format!(
                "{{\"file\":{},\"tr\":{},\"a\":{},\"b\":{},\"baseline\":\"line\",\"line\":[{},{},{},{}],\"grp\":{},\"origin\":\"{}\",\"sn\":{}}}",
                f.file, f.tr, f.a, f.b, f.line[0], f.line[1], f.line[2], f.line[3], means[f.group],
                if alone { "unmatched" } else { "auto" }, f.sn
            ));
        }
    }
    Ok(format!(
        "{{\"tol\":{tol},\"windows\":[{}],\"log\":[{}],\"groups\":{n_groups},\"unmatched\":{n_alone}}}",
        out.join(","),
        log.join(",")
    ))
}

fn copy(f: &Found) -> Found {
    Found {
        key: f.key.clone(),
        ..*f
    }
}
