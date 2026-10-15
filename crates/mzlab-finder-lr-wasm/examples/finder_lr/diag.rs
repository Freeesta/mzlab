//! `finder_lr --diag`: why the Dig does not list the ion of a peak of an MS2 trace. For every peak that no row gets (before the MS2 targets), the
//! features of the aligned table within 0.5 Da and 0.5 min of its apex, with the cells, the score and the step of the Dig that dropped each one.

use mzlab_core::idxread::Index;
use mzlab_feat::legacy::lr::labels::{score, GROWS, PERSISTS};
use mzlab_feat::legacy::lr::{ms2link, rows, LrDig};

type Open<'a> = &'a mut dyn FnMut(usize) -> Result<Index<Vec<u8>>, String>;

/// The step of the Dig that dropped the feature `r` (the tolerance, the criteria and the neighbours are those of `rows::unexpected`).
fn step(d: &LrDig, r: usize) -> String {
    let t = &*d.table;
    let tol = d.settings.tol_da;
    if let Some(e) = d.entries.iter().find(|e| e.row == Some(r)) {
        return format!("is the row {} ({})", e.id, e.name);
    }
    if let Some(h) = d
        .entries
        .iter()
        .find(|e| !e.unexpected && (t.mz[r] - e.mz_x).abs() <= tol)
    {
        let own = d
            .entries
            .iter()
            .filter(|e| (e.mz_x - h.mz_x).abs() < 1e-9 && e.row.is_some())
            .count();
        return format!(
            "hypothesis '{}' (m/z {:.2}, {} peak(s) listed, its row at {:?} min) is within {tol} Da: not an unexpected ion; as an isomer it fails the 10 % / 4 peaks rule",
            h.name, h.mz_x, own, h.ref_rt
        );
    }
    if t.rt[r] < d.settings.rt_min {
        return format!("before rt_min ({})", d.settings.rt_min);
    }
    let sc = score(&d.lc, &rows::cells(t, r), d.settings.persist());
    let bad: Vec<&str> = [
        "absent in the blank",
        "absent at time zero",
        GROWS,
        PERSISTS,
    ]
    .into_iter()
    .filter(|n| {
        sc.criteria
            .iter()
            .any(|c| c.name == *n && c.status == "fail")
    })
    .collect();
    if !bad.is_empty() || !matches!(sc.label, "forte" | "possibile") {
        return format!("criteria: label {}, failed: {:?}", sc.label, bad);
    }
    "passes the criteria: dropped as the weaker of two neighbours or by max_unexpected".into()
}

pub fn print(d: &LrDig, open: Open) {
    let l = ms2link::compute(d, &[], open).expect("matches");
    let t = &*d.table;
    for u in l
        .unmatched
        .iter()
        .filter(|u| matches!(u.reason, "no_row" | "no_row_at_rt"))
    {
        let (Some(x), Some(k)) = (u.exp, u.peak) else {
            continue;
        };
        let (e, p) = (&l.exps[x], &l.exps[x].peaks[k]);
        println!(
            "== {} P {:.2} CE {:?} peak {:.2} min (S/N {:.1}, drift {:.2}): {}",
            e.file, e.precursor, e.ce, p.apex, p.sn, e.drift, u.reason
        );
        let mut any = false;
        for r in (0..t.len()).filter(|&r| {
            (t.mz[r] - e.precursor).abs() <= 0.5 + 0.4
                && (t.rt[r] - (p.apex - e.drift)).abs() <= 0.5
        }) {
            any = true;
            let c = rows::cells(t, r);
            let sc = score(&d.lc, &c, d.settings.persist());
            let cells: Vec<String> = c
                .iter()
                .map(|c| {
                    if c.detected {
                        format!("{:.0}", c.area)
                    } else {
                        "-".into()
                    }
                })
                .collect();
            println!(
                "   m/z {:.2} RT {:.2} score {:?} {} | cells {} | {}",
                t.mz[r],
                t.rt[r],
                sc.score,
                sc.label,
                cells.join(" "),
                step(d, r)
            );
            for c in &sc.criteria {
                println!("      {:<24} {:<5} {}", c.name, c.status, c.text);
            }
        }
        if !any {
            println!("   no feature in the table (detection)");
        }
    }
}
