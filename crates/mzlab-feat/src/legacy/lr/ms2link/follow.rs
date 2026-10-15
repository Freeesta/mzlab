//! Rules 10 and 11 of the match: the corrections of the user that name an experiment, and the peaks and experiments that no row got.

use super::exp::Exp;
use super::rules::*;
use super::{State, Unmatched};
use crate::legacy::lr::Entry;

/// Rule 10: the corrections that name this experiment. Returns the peaks they took.
pub(super) fn corrections(c: &mut Ctx, e: &mut Exp, x: usize) -> Vec<usize> {
    let d = c.d;
    let tol = d.settings.rt_tol_min;
    let mut taken = Vec::new();
    for (i, o) in c.ov.iter().enumerate() {
        let same_ce = match (o.ce, e.ce) {
            (None, None) => true,
            (Some(a), Some(b)) => (a - b).abs() <= KEY_TOL,
            _ => false,
        };
        if o.file != e.file || (o.precursor - e.precursor).abs() > KEY_TOL || !same_ce {
            continue;
        }
        let near = (0..e.peaks.len())
            .filter(|&p| (e.peaks[p].apex - o.apex).abs() <= KEY_APEX * tol)
            .min_by(|a, b| {
                (e.peaks[*a].apex - o.apex)
                    .abs()
                    .total_cmp(&(e.peaks[*b].apex - o.apex).abs())
            });
        let peak = match near {
            Some(p) => p,
            None => match find(e, o.apex, KEY_APEX * tol) {
                Some(h) => register(e, &h),
                None => continue,
            },
        };
        let row = o.row.map(|id| d.entries.iter().find(|r| r.id == id));
        match row {
            Some(None) => continue,
            Some(Some(r)) => {
                let at = expected(d, r, e);
                let hit = Hit {
                    apex: e.peaks[peak].apex,
                    lo: e.peaks[peak].lo,
                    hi: e.peaks[peak].hi,
                    sn: e.peaks[peak].sn,
                    window: false,
                };
                let p = Pair {
                    row: r,
                    dmz: dmz(r, e.precursor),
                    drt: hit.apex - at,
                    hit,
                };
                let mut l = link_of(x, peak, &p, State::User, vec![], "");
                l.why = "set by the user".into();
                c.links.push(l);
            }
            None => c.un.push(Unmatched {
                exp: Some(x),
                peak: Some(peak),
                reason: "user",
                rows: vec![],
                why: "set by the user: not this ion".into(),
            }),
        }
        c.used[i] = true;
        taken.push(peak);
    }
    taken
}

/// Rule 11: the peaks (and, without any, the experiment) that no row got.
pub(super) fn leftovers(
    c: &mut Ctx,
    e: &Exp,
    x: usize,
    comp: &[&Entry],
    claimed: &[usize],
    any_pair: bool,
    any_link: bool,
) {
    let ids: Vec<i64> = comp.iter().map(|r| r.id).collect();
    for (i, p) in e
        .peaks
        .iter()
        .enumerate()
        .filter(|(i, _)| !claimed.contains(i))
    {
        let (reason, why) = if comp.is_empty() {
            ("no_row", format!("no row at m/z {:.2}", e.precursor))
        } else {
            (
                "no_row_at_rt",
                format!(
                    "rows at m/z {:.2}, but not at {:.2} min",
                    e.precursor, p.apex
                ),
            )
        };
        c.un.push(Unmatched {
            exp: Some(x),
            peak: Some(i),
            reason,
            rows: ids.clone(),
            why,
        });
    }
    if any_pair || any_link {
        return;
    }
    let (reason, why) = if e.peaks.is_empty() && e.front {
        (
            "front",
            format!("signal only before {:.2} min", c.d.settings.rt_min),
        )
    } else if !comp.is_empty() {
        (
            "no_signal",
            format!(
                "no MS2 signal at the time of the row(s) at m/z {:.2}",
                e.precursor
            ),
        )
    } else if e.peaks.is_empty() {
        (
            "no_row",
            format!("no row at m/z {:.2}, and no peak in the trace", e.precursor),
        )
    } else {
        return;
    };
    c.un.push(Unmatched {
        exp: Some(x),
        peak: None,
        reason,
        rows: ids,
        why,
    });
}
