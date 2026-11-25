//! The internal RT reference of `quant.targets` (`rt_ref`): the row whose apex moves the windows of each file, the evidence of what it found, and the
//! hints that point at `--offset auto` and `--rt-ref auto` when the reference row shows they are needed. Nothing here runs with `rt_ref` off.

use mzlab_core::error::Result;
use mzlab_core::idxread::ReadAt;
use mzlab_core::json::round;
use mzlab_core::rawxic::tallest;
use mzlab_core::routes::Routes;

#[cfg(not(target_arch = "wasm32"))]
use super::params::OffsetMode;
use super::params::{Params, RtRef};
#[cfg(not(target_arch = "wasm32"))]
use super::run::Row;
use super::run::{warn, Choice, GroupOffset};
use super::targets::{Role, Target};

/// What the internal reference said in one file: where its apex was looked for, where it was found and the shift applied to every window.
pub(crate) struct RtShift {
    pub reference: String,
    pub expected: f64,
    pub apex: Option<f64>,
    pub shift: f64,
    /// `ok`, `edge` (apex on the border of the search window, applied), `weak` (a peak under the S/N or shape limits, not applied) or `not_found`.
    pub status: &'static str,
    pub sn: Option<f64>,
    pub search: (f64, f64),
}

/// A pointer to a switch the data suggests (`note.quant.targets.*` of the language files, `{name}` filled from `params`): said on stderr by the
/// CLI, never written in the table or the log, so no byte of an existing output depends on it.
#[derive(Debug, Clone, PartialEq)]
pub struct Hint {
    pub key: &'static str,
    pub params: Vec<(&'static str, String)>,
}

/// The switches the evidence of the reference row points to: the offset at 0 while the reference is missed (`--offset auto`), and a reference
/// whose apex moves 0.1 min or more between files while the windows stay (`--rt-ref auto`). Blank files do not count.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn hints(p: &Params, targets: &[Target], rows: &[Row], blank: &[bool]) -> Vec<Hint> {
    let mut out = Vec::new();
    let Some(r) = targets.iter().position(|t| t.role == Role::Reference) else {
        return out;
    };
    let own: Vec<&Row> = rows
        .iter()
        .filter(|x| x.target == r && !blank[x.file])
        .collect();
    if p.offset == OffsetMode::Fixed(0.0) {
        let missed = own.iter().filter(|x| !x.detected()).count();
        if missed > 0 {
            out.push(Hint {
                key: "note.quant.targets.offsetAuto",
                params: vec![
                    ("id", targets[r].id.clone()),
                    ("count", missed.to_string()),
                    ("files", own.len().to_string()),
                ],
            });
        }
    }
    if p.rt_ref == RtRef::Off {
        let apex: Vec<f64> = own
            .iter()
            .filter(|x| x.detected())
            .filter_map(|x| x.hit.as_ref().map(|h| h.rt))
            .collect();
        let (lo, hi) = apex
            .iter()
            .fold((f64::MAX, f64::MIN), |a, &v| (a.0.min(v), a.1.max(v)));
        if apex.len() >= 2 && hi - lo >= 0.1 {
            out.push(Hint {
                key: "note.quant.targets.rtRef",
                params: vec![
                    ("id", targets[r].id.clone()),
                    ("spread", format!("{}", round(hi - lo, 2))),
                    ("min", format!("{}", round(lo, 2))),
                    ("max", format!("{}", round(hi, 2))),
                ],
            });
        }
    }
    out
}

/// The shift of one file from its internal reference: the tallest peak of the reference m/z within its window widened by `rt_span`, minus the RT
/// it is expected at. A weak or missing peak moves nothing (shift 0, the status says why): the windows are never moved on noise.
pub(super) fn measure<R: ReadAt>(
    routes: &mut Routes<R>,
    t: &Target,
    expected: f64,
    offs: &[GroupOffset],
    p: &Params,
) -> Result<RtShift> {
    let search = (
        round((t.rt_min.min(expected) - p.rt_span).max(0.0), 4),
        round(t.rt_max.max(expected) + p.rt_span, 4),
    );
    let mut out = RtShift {
        reference: t.id.clone(),
        expected,
        apex: None,
        shift: 0.0,
        status: "not_found",
        sn: None,
        search,
    };
    routes.pol = t.polarity;
    routes.mz_off = offs
        .iter()
        .find(|g| g.polarity == t.polarity)
        .map_or(0.0, |g| g.offset.da);
    routes.forget_sweep();
    let m = routes.sweep_matrix(&[t.mz], &[p.da], p.ms_level, None)?;
    if !m.rt.is_empty() {
        let rt: Vec<f64> = m.rt.iter().map(|&x| round(x, 4)).collect();
        let y: Vec<f64> = m.y[0].iter().map(|&v| round(v, 1)).collect();
        let (hit, _, _, edge) = tallest(&rt, &y, search.0, search.1, p.smooth_half, p.valley_pct);
        if let Some(h) = hit {
            out.apex = Some(h.rt);
            out.sn = Some(h.sn);
            if h.sn < p.min_sn || h.points < p.min_points || h.shape < p.min_shape {
                out.status = "weak";
            } else {
                out.status = if edge { "edge" } else { "ok" };
                out.shift = round(h.rt - expected, 4);
            }
        }
    }
    drop(m);
    routes.forget_sweep();
    Ok(out)
}

/// The row `rt_ref` follows and the RT it is expected at (the given one, else the centre of its window); an id that is not in the table is
/// `err.quant.targets.param`. `auto` with no standard/parent row gives `None`.
pub(super) fn rt_reference(targets: &[Target], p: &Params) -> Result<Option<(usize, f64)>> {
    let (i, rt) = match &p.rt_ref {
        RtRef::Off => return Ok(None),
        RtRef::Auto { rt } => match targets.iter().position(|t| t.role == Role::Reference) {
            Some(i) => (i, *rt),
            None => return Ok(None),
        },
        RtRef::Id { id, rt } => match targets.iter().position(|t| &t.id == id) {
            Some(i) => (i, *rt),
            None => return Err(super::params::bad("rt_ref", &p.rt_ref_text())),
        },
    };
    let t = &targets[i];
    Ok(Some((
        i,
        rt.unwrap_or(round((t.rt_min + t.rt_max) / 2.0, 4)),
    )))
}

/// The choices (and the warning) `rt_ref` adds to the log: which row, which frame.
pub(crate) fn rt_choices(
    targets: &[Target],
    p: &Params,
    rt_ref: Option<(usize, f64)>,
    choices: &mut Vec<Choice>,
    warnings: &mut Vec<String>,
) {
    if p.rt_ref != RtRef::Off {
        match rt_ref {
            Some((i, expected)) => {
                let named = matches!(p.rt_ref, RtRef::Id { .. });
                choices.push(Choice {
                    key: "rt_reference",
                    value: targets[i].id.clone(),
                    reason: if named {
                        "the row named by rt_ref".into()
                    } else {
                        "first standard/parent row".into()
                    },
                });
                let given = match &p.rt_ref {
                    RtRef::Auto { rt } | RtRef::Id { rt, .. } => rt.is_some(),
                    RtRef::Off => false,
                };
                choices.push(Choice {
                    key: "rt_expected",
                    value: format!("{expected}"),
                    reason: if given {
                        "given in rt_ref".into()
                    } else {
                        "centre of the reference window; every window moves by the apex found minus this".into()
                    },
                });
            }
            None => {
                warn("warn.quant.targets.noRtReference", warnings);
                choices.push(Choice {
                    key: "rt_reference",
                    value: "none".into(),
                    reason: "no standard/parent row: windows not moved".into(),
                });
            }
        }
    }
}
