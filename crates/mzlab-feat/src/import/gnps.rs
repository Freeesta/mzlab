//! GNPS2 FBMN: the library matches (external evidence only, never a verdict) and the components of the network.

use super::chem::{clean_name, is_propagated, norm_adduct};
use super::delim::{cell, each_row, num, txt, Cols};
use super::model::{Rec, Table, V};
use super::ImportError;
use std::cmp::Ordering;

/// A match whose |mass difference| is above this is an analogue, not the compound.
pub const ANALOG_PPM: f64 = 15.0;

/// The sort of the browser engine (binary insertion sort of its merge sort for short lists) with a comparator that is not a total order:
/// annotations of other sources compare equal to everything, so the result depends on the algorithm and must be the same one.
pub fn engine_sort<T: Clone>(a: &mut [T], cmp: impl Fn(&T, &T) -> i32) {
    let n = a.len();
    if n < 2 {
        return;
    }
    let mut run = 2;
    let desc = cmp(&a[1], &a[0]) < 0;
    for i in 2..n {
        let o = cmp(&a[i], &a[i - 1]);
        if (desc && o >= 0) || (!desc && o < 0) {
            break;
        }
        run += 1;
    }
    if desc {
        a[..run].reverse();
    }
    for start in run..n {
        let (mut left, mut right) = (0, start);
        let pivot = a[start].clone();
        while left < right {
            let mid = left + ((right - left) >> 1);
            if cmp(&pivot, &a[mid]) < 0 {
                right = mid
            } else {
                left = mid + 1
            }
        }
        for p in (left + 1..=start).rev() {
            a[p] = a[p - 1].clone();
        }
        a[left] = pivot;
    }
}

fn order(a: &Rec, b: &Rec) -> i32 {
    if a.text("source") != "GNPS2" || b.text("source") != "GNPS2" {
        return 0;
    }
    let d = a.flag("analog") as i32 - b.flag("analog") as i32;
    if d != 0 {
        return d;
    }
    let (x, y) = (a.num("score").unwrap_or(0.0), b.num("score").unwrap_or(0.0));
    match (y - x).partial_cmp(&0.0) {
        Some(Ordering::Less) => -1,
        Some(Ordering::Greater) => 1,
        _ => 0,
    }
}

type Ix = [Option<usize>; 13];

const NAMES: [&[&str]; 13] = [
    &["#scan#", "scan"],
    &["compound_name"],
    &["mqscore"],
    &["sharedpeaks"],
    &["specmz"],
    &["massdiff"],
    &["inchikey-planar"],
    &["inchikey"],
    &["smiles"],
    &["inchi"],
    &["libraryname"],
    &["adduct"],
    &["mzerrorppm"],
];

fn match_ann(
    x: &Ix,
    r: &[String],
    nm: &str,
    mzs: Option<f64>,
    ppm: Option<f64>,
    analog: bool,
) -> Rec {
    let n = |i: usize| x[i].and_then(|_| num(cell(r, x[i])));
    let s = |i: usize| {
        if x[i].is_some() {
            txt(cell(r, x[i]))
        } else {
            ""
        }
    };
    let ik = if x[6].is_some() && !txt(cell(r, x[6])).is_empty() {
        txt(cell(r, x[6]))
    } else {
        s(7)
    };
    let mut a = Rec::default();
    for (k, v) in [
        ("source", V::s("GNPS2")),
        ("type", V::s("MQScore")),
        ("score", V::opt(n(2))),
        ("shared", V::opt(n(3))),
        ("ppm", V::opt(ppm)),
        ("name", V::s(nm)),
        ("cleanName", V::Str(clean_name(nm))),
        ("ik", V::s(ik)),
        ("smiles", V::s(s(8))),
        ("inchi", V::s(s(9))),
        (
            "adduct",
            V::Str(if x[11].is_some() {
                norm_adduct(cell(r, x[11]))
            } else {
                String::new()
            }),
        ),
        ("library", V::s(s(10))),
        ("precursor", V::opt(mzs)),
        ("analog", V::Bool(analog)),
        ("propagated", V::Bool(false)),
        (
            "notes",
            V::List(if analog {
                vec![format!("analogue: |mass difference| > {ANALOG_PPM} ppm")]
            } else {
                vec![]
            }),
        ),
    ] {
        a.set(k, v);
    }
    a
}

/// The library matches of a GNPS2 job; per feature the exact ones come first, then by score.
pub fn read_gnps_library(t: &mut Table, text: &str) -> Result<(), ImportError> {
    let (mut rows, mut analogs, mut prop, mut lost) = (0.0, 0.0, 0.0, 0.0);
    each_row(
        text,
        b'\t',
        None,
        |h| {
            let c = Cols::new(h);
            let mut x: Ix = [None; 13];
            for (i, n) in NAMES.iter().enumerate() {
                x[i] = c.pick(n);
            }
            Ok(x)
        },
        |x, r, _| {
            rows += 1.0;
            let (id, nm) = (txt(cell(r, x[0])), txt(cell(r, x[1])));
            let mzs = x[4].and_then(|_| num(cell(r, x[4])));
            let md = x[5].and_then(|_| num(cell(r, x[5])));
            let ppm = match (mzs.filter(|m| *m != 0.0), md) {
                (Some(m), Some(d)) => Some(d / m * 1e6),
                _ => x[12].and_then(|_| num(cell(r, x[12]))),
            };
            let analog = ppm.is_some_and(|p| p.abs() > ANALOG_PPM);
            if analog {
                analogs += 1.0;
            }
            let Some(fi) = t.feature(id) else {
                lost += 1.0;
                return Ok(());
            };
            if is_propagated(nm) {
                prop += 1.0;
                return Ok(());
            }
            t.features[fi]
                .ann
                .push(match_ann(x, r, nm, mzs, ppm, analog));
            Ok(())
        },
    )?;
    for f in t.features.iter_mut() {
        engine_sort(&mut f.ann, order);
    }
    t.stat("gnpsRows", rows);
    t.stat("gnpsAnalogs", analogs);
    t.stat("gnpsPropagated", prop);
    t.stat("gnpsUnmatched", lost);
    Ok(())
}

/// The component of each feature in the network (-1 = isolated node, kept as no component).
pub fn read_gnps_network(t: &mut Table, text: &str) -> Result<usize, ImportError> {
    let mut n = 0;
    each_row(
        text,
        b'\t',
        None,
        |h| {
            let c = Cols::new(h);
            Ok((c.pick(&["cluster index", "#scan#"]), c.pick(&["component"])))
        },
        |(id, comp), r, _| {
            let Some(fi) = t.feature(txt(cell(r, *id))) else {
                return Ok(());
            };
            if comp.is_none() {
                return Ok(());
            }
            t.features[fi].component = num(cell(r, *comp)).filter(|c| *c >= 0.0);
            n += 1;
            Ok(())
        },
    )?;
    t.stat("networkRows", n as f64);
    Ok(n)
}
