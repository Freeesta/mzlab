// Ported from Attributio gradient.py.
//! The choice of the feature that stands for an identity at the two sites: the largest feature, or the largest of the same sorbent, with its log2 fold change.

use super::gradient::AreasBySorbent;

/// One feature of an identity with its areas.
#[derive(Clone)]
pub(super) struct Cand {
    pub(super) us: f64,
    pub(super) ds: f64,
    pub(super) pol: String,
    pub(super) id: i64,
    pub(super) rt: f64,
}

/// First element with the largest key (Python's `max` keeps the first of equals).
pub(super) fn first_max<T>(v: &[T], key: impl Fn(&T) -> f64) -> &T {
    let mut best = &v[0];
    for x in &v[1..] {
        if key(x) > key(best) {
            best = x;
        }
    }
    best
}

pub(super) struct Pick {
    pub(super) us: f64,
    pub(super) ds: f64,
    pub(super) pol: String,
    pub(super) id: i64,
    pub(super) rt: f64,
    pub(super) l_ud: Option<f64>,
    pub(super) l_du: Option<f64>,
    pub(super) sorbent: Option<String>,
    pub(super) discordant: bool,
}

pub(super) fn logs(us: f64, ds: f64) -> (Option<f64>, Option<f64>) {
    if us > 0.0 && ds > 0.0 {
        (Some((us / ds).log2()), Some((ds / us).log2()))
    } else {
        (None, None)
    }
}

pub(super) fn pick_plain(a: &[Cand]) -> Pick {
    let common: Vec<Cand> = a
        .iter()
        .filter(|t| t.us > 0.0 && t.ds > 0.0)
        .cloned()
        .collect();
    let (us, ds, pol, id, rt) = if common.is_empty() {
        let bu = first_max(a, |t| t.us);
        let bd = first_max(a, |t| t.ds);
        (bu.us, bd.ds, bu.pol.clone(), bu.id, bu.rt)
    } else {
        let b = first_max(&common, |t| t.us.max(t.ds));
        (b.us, b.ds, b.pol.clone(), b.id, b.rt)
    };
    let (l_ud, l_du) = logs(us, ds);
    Pick {
        us,
        ds,
        pol,
        id,
        rt,
        l_ud,
        l_du,
        sorbent: None,
        discordant: false,
    }
}

/// Whether the other sorbents point the other way for the ion (`discordant`).
pub(super) fn other_sorbents_disagree(
    by_sb: &AreasBySorbent,
    sorbents: &[String],
    ion: (&str, i64),
    used: &str,
    sites: [&str; 2],
    l_ud: f64,
    fold: f64,
) -> bool {
    let get = |site: &str, sb: &str| {
        by_sb
            .get(&(ion.0.to_string(), site.to_string(), sb.to_string()))
            .and_then(|m| m.get(&ion.1))
            .copied()
            .unwrap_or(0.0)
    };
    let thr = fold.log2();
    let mut discordant = false;
    for sb in sorbents.iter().filter(|s| *s != used) {
        let (o_us, o_ds) = (get(sites[0], sb), get(sites[1], sb));
        if l_ud.abs() >= thr {
            let up = l_ud > 0.0;
            discordant = if o_us > 0.0 && o_ds > 0.0 {
                let o = (o_us / o_ds).log2();
                (o > 0.0) != up && o.abs() >= thr
            } else if up {
                o_ds > 0.0 && o_us == 0.0
            } else {
                o_us > 0.0 && o_ds == 0.0
            };
        }
    }
    discordant
}

pub(super) fn pick_same_sorbent(
    a: &[Cand],
    by_sb: &AreasBySorbent,
    sorbents: &[String],
    sites: [&str; 2],
    fold: f64,
) -> Pick {
    let get = |pol: &str, site: &str, sb: &str, id: i64| {
        by_sb
            .get(&(pol.to_string(), site.to_string(), sb.to_string()))
            .and_then(|m| m.get(&id))
            .copied()
            .unwrap_or(0.0)
    };
    let mut pairs = Vec::new();
    for t in a {
        for sb in sorbents {
            let (u, d) = (
                get(&t.pol, sites[0], sb, t.id),
                get(&t.pol, sites[1], sb, t.id),
            );
            if u > 0.0 && d > 0.0 {
                pairs.push((
                    Cand {
                        us: u,
                        ds: d,
                        pol: t.pol.clone(),
                        id: t.id,
                        rt: t.rt,
                    },
                    sb.clone(),
                ));
            }
        }
    }
    if pairs.is_empty() {
        let b = first_max(a, |t| t.us.max(t.ds));
        return Pick {
            us: b.us,
            ds: b.ds,
            pol: b.pol.clone(),
            id: b.id,
            rt: b.rt,
            l_ud: None,
            l_du: None,
            sorbent: None,
            discordant: false,
        };
    }
    let mut best = &pairs[0];
    for p in &pairs[1..] {
        if p.0.us.max(p.0.ds) > best.0.us.max(best.0.ds) {
            best = p;
        }
    }
    let (c, sb) = best;
    let (l_ud, l_du) = logs(c.us, c.ds);
    let discordant = other_sorbents_disagree(
        by_sb,
        sorbents,
        (&c.pol, c.id),
        sb,
        sites,
        l_ud.unwrap_or(f64::NAN),
        fold,
    );
    Pick {
        us: c.us,
        ds: c.ds,
        pol: c.pol.clone(),
        id: c.id,
        rt: c.rt,
        l_ud,
        l_du,
        sorbent: Some(sb.clone()),
        discordant,
    }
}

pub(super) fn pick_one_site(a: &[Cand], us: bool) -> Pick {
    let b = first_max(a, |t| if us { t.us } else { t.ds });
    let (au, ad) = if us { (b.us, 0.0) } else { (0.0, b.ds) };
    Pick {
        us: au,
        ds: ad,
        pol: b.pol.clone(),
        id: b.id,
        rt: b.rt,
        l_ud: None,
        l_du: None,
        sorbent: None,
        discordant: false,
    }
}

impl Pick {
    /// An identity with no area at either site: its first feature, zeros.
    pub(super) fn none(first: &Cand) -> Pick {
        Pick {
            us: 0.0,
            ds: 0.0,
            pol: first.pol.clone(),
            id: first.id,
            rt: first.rt,
            l_ud: None,
            l_du: None,
            sorbent: None,
            discordant: false,
        }
    }
}
