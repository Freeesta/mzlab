//! The neutral-mass hypotheses of a connected component of the identity graph (`hr/identity.py::_hypotheses`).

use super::*;

/// The role of a member in a hypothesis.
#[derive(Clone)]
pub(super) struct Role {
    pub(super) label: String,
    pub(super) err: f64,
    pub(super) kind: &'static str,
    pub(super) base: bool,
    pub(super) reference: Option<String>,
}

pub(super) struct Hyp {
    pub(super) mass: f64,
    pub(super) roles: Vec<(usize, Role)>,
    pub(super) n_base: usize,
    pub(super) rms: f64,
    pub(super) area: f64,
    pub(super) anchor: usize,
    pub(super) mzmine: bool,
}

pub(super) fn explain_by_adduct(c: &Ctx, ms: &[usize], fs: &[Feat], m: f64) -> Vec<(usize, Role)> {
    let mut role = Vec::new();
    for &j in ms {
        let mut best: Option<(&str, f64, bool)> = None;
        for &(name, shift, mult, b) in c.forms {
            let exp = mult * m + shift;
            let err = fs[j].mz - exp;
            if err.abs() <= (exp * c.o.ppm * 1e-6).max(c.o.abs_da)
                && best.is_none_or(|x| err.abs() < x.1.abs())
            {
                best = Some((name, err, b));
            }
        }
        if let Some((name, err, b)) = best {
            role.push((
                j,
                Role {
                    label: name.into(),
                    err,
                    kind: "adduct",
                    base: b,
                    reference: None,
                },
            ));
        }
    }
    role
}

/// Up to three rounds: a member unexplained is the isotopologue or the in-source fragment of an explained one.
pub(super) fn grow(c: &Ctx, ms: &[usize], fs: &[Feat], role: &mut Vec<(usize, Role)>) {
    for _ in 0..3 {
        let mut grew = false;
        for &j in ms {
            if role.iter().any(|x| x.0 == j) {
                continue;
            }
            let snapshot = role.clone();
            for (e, r) in &snapshot {
                let d = fs[j].mz - fs[*e].mz;
                let tol = (fs[*e].mz * c.o.ppm * 1e-6).max(c.o.abs_da);
                let rf = r.reference.clone().unwrap_or_else(|| r.label.clone());
                let iso = edge::ISOTOPES.iter().find(|x| (d - x.1).abs() <= tol);
                let hit = match iso {
                    Some((nm, v)) => Some((
                        "isotope",
                        format!("{nm} isotope of {rf}"),
                        d - v,
                        rf.clone(),
                    )),
                    None => {
                        c.t.losses
                            .iter()
                            .find(|x| (d + x.1).abs() <= tol)
                            .map(|(nm, v)| {
                                (
                                    "loss",
                                    format!("-{nm} in source from {rf}"),
                                    d + v,
                                    format!("the -{nm} fragment"),
                                )
                            })
                    }
                };
                if let Some((kind, label, err, rf)) = hit {
                    role.push((
                        j,
                        Role {
                            label,
                            err,
                            kind,
                            base: false,
                            reference: Some(rf),
                        },
                    ));
                    grew = true;
                    break;
                }
            }
        }
        if !grew {
            break;
        }
    }
}

/// `hypotheses`: neutral masses that explain the members, best first.
pub(super) fn hypotheses(c: &Ctx, ms: &[usize], fs: &[Feat], maxa: &[f64]) -> Vec<Hyp> {
    let mut cap: Vec<usize> = ms.to_vec();
    cap.sort_by(|&a, &b| maxa[b].total_cmp(&maxa[a]));
    cap.truncate(c.o.max_hyp);
    let mut cands: Vec<(f64, &str)> = Vec::new();
    for &i in &cap {
        for f in c.forms {
            cands.push(((fs[i].mz - f.1) / f.2, "t"));
        }
    }
    for &i in ms {
        if let Some(n) = fs[i].iin_any.and_then(|g| g.get("neutral")).and_then(|v| {
            v.f64()
                .or_else(|| v.str().and_then(|s| s.trim().parse().ok()))
        }) {
            cands.push((n, "mzmine"));
        }
    }
    cands.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(b.1)));
    let mut clusters: Vec<(f64, bool)> = Vec::new();
    for (m, src) in cands {
        match clusters.last_mut() {
            Some(cl) if (m - cl.0).abs() <= (m * c.o.ppm * 1e-6).max(c.o.abs_da) => {
                cl.1 |= src == "mzmine"
            }
            _ => clusters.push((m, src == "mzmine")),
        }
    }
    let mut out = Vec::new();
    for (m, mzmine) in clusters {
        let mut role = explain_by_adduct(c, ms, fs, m);
        if role.is_empty() {
            continue;
        }
        grow(c, ms, fs, &mut role);
        let errs: Vec<f64> = role.iter().map(|x| x.1.err * x.1.err).collect();
        let mut anchor = role[0].0;
        let key = |j: usize, r: &Role| (r.kind == "adduct", r.base, maxa[j]);
        for (j, r) in &role {
            let (a, b) = (
                key(*j, r),
                key(anchor, &role.iter().find(|x| x.0 == anchor).unwrap().1),
            );
            if (a.0, a.1) > (b.0, b.1) || ((a.0, a.1) == (b.0, b.1) && a.2 > b.2) {
                anchor = *j;
            }
        }
        out.push(Hyp {
            mass: m,
            n_base: role.iter().filter(|x| x.1.base).count(),
            rms: mean(&errs).sqrt(),
            area: role.iter().fold(0.0, |a, x| a + maxa[x.0]),
            anchor,
            mzmine,
            roles: role,
        });
    }
    out.sort_by(|a, b| {
        b.roles
            .len()
            .cmp(&a.roles.len())
            .then(b.n_base.cmp(&a.n_base))
            .then(a.rms.total_cmp(&b.rms))
            .then(b.area.total_cmp(&a.area))
    });
    let mut fin: Vec<Hyp> = Vec::new();
    for h in out {
        let covered = fin.iter().any(|g| {
            h.roles.iter().all(|x| g.roles.iter().any(|y| y.0 == x.0))
                && (h.mass - g.mass).abs() / g.mass < 1e-9
        });
        if !covered {
            fin.push(h);
        }
    }
    fin
}

pub(super) fn hyp_out(h: Option<&Hyp>, fs: &[Feat], o: &mut Out) {
    let Some(h) = h else {
        o.raw("null");
        return;
    };
    let first = &h.roles.iter().find(|x| x.0 == h.anchor).unwrap().1.label;
    o.raw("{\"mass\":")
        .num(json::round(h.mass, 5))
        .raw(",\"explained\":")
        .int(h.roles.len() as i64);
    o.raw(",\"anchor\":");
    write_value(fs[h.anchor].v.get("k").unwrap_or(&Value::Null), o);
    o.raw(",\"form\":")
        .str(first)
        .raw(",\"from_mzmine\":")
        .raw(if h.mzmine { "true" } else { "false" });
    o.raw(",\"roles\":{");
    let mut done: Vec<&str> = Vec::new();
    let mut first_key = true;
    for (j, r) in &h.roles {
        let k = fs[*j].key.as_str();
        if done.contains(&k) {
            continue;
        }
        done.push(k);
        o.raw(if first_key { "" } else { "," })
            .str(k)
            .raw(":{\"label\":")
            .str(&r.label);
        o.raw(",\"kind\":")
            .str(r.kind)
            .raw(",\"dmz\":")
            .num(json::round(r.err, 5));
        o.raw(",\"ppm\":")
            .num(json::round(r.err / fs[*j].mz * 1e6, 2))
            .raw("}");
        first_key = false;
    }
    o.raw("}}");
}

/// `numpy.median`.
pub(super) fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}
