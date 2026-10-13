//! MS-DIAL alignment (.txt, tab): metadata rows on top, then the header with «Alignment ID»; the MS/MS spectrum is «m/z:int m/z:int …».

use super::chem::{clean_name, norm_adduct, ppm_of};
use super::delim::{cell, each_row, js_number, jstrim, num, txt, Cols};
use super::model::{Ms2, Rec, Table, V};
use super::ImportError;

/// «m/z:int m/z:int …» → the two lists (a pair with a non-numeric part is skipped).
pub fn parse_spectrum(s: &str) -> Ms2 {
    let mut m = Ms2::default();
    for p in txt(s).split(char::is_whitespace) {
        let Some(k) = p.find(':').filter(|k| *k > 0) else {
            continue;
        };
        if let (Some(a), Some(b)) = (
            js_number(&p[..k]).filter(|v| v.is_finite()),
            js_number(&p[k + 1..]).filter(|v| v.is_finite()),
        ) {
            m.mz.push(a);
            m.it.push(b);
        }
    }
    m
}

struct Cx {
    ix: [Option<usize>; 14],
    samples: Vec<(String, usize)>,
}

const NAMES: [&[&str]; 14] = [
    &["alignment id"],
    &["average rt(min)", "average rt (min)", "average rt"],
    &["average mz", "average m/z"],
    &["metabolite name"],
    &["adduct type"],
    &["formula"],
    &["inchikey"],
    &["smiles"],
    &["dot product"],
    &["reverse dot product"],
    &["total score"],
    &["ms/ms spectrum"],
    &["reference m/z"],
    &["ms/ms assigned"],
];

fn columns(h: &[String]) -> Result<Cx, ImportError> {
    let c = Cols::new(h);
    let mut ix = [None; 14];
    for (i, n) in NAMES.iter().enumerate() {
        ix[i] = c.pick(n);
    }
    if ix[0].is_none() || ix[1].is_none() || ix[2].is_none() {
        return Err(ImportError::new(
            "columns",
            "MS-DIAL table: columns Alignment ID, Average Rt and Average Mz not found",
        ));
    }
    let start = [
        ix[11],
        c.pick(&["spectrum reference file name"]),
        c.pick(&["ms1 isotopic spectrum"]),
    ]
    .into_iter()
    .flatten()
    .max();
    let samples = h
        .iter()
        .enumerate()
        .filter(|(i, c)| start.is_none_or(|s| *i > s) && !c.is_empty())
        .map(|(i, c)| (c.clone(), i))
        .collect();
    Ok(Cx { ix, samples })
}

fn annotation(x: &Cx, r: &[String], mz: f64, nm: &str) -> Rec {
    let n = |i: usize| x.ix[i].and_then(|_| num(cell(r, x.ix[i])));
    let s = |i: usize| {
        if x.ix[i].is_some() {
            txt(cell(r, x.ix[i]))
        } else {
            ""
        }
    };
    let mut a = Rec::default();
    for (k, v) in [
        ("source", V::s("MS-DIAL")),
        ("type", V::s("dot product (0-1000)")),
        ("score", V::opt(n(8))),
        ("reverse", V::opt(n(9))),
        ("total", V::opt(n(10))),
        ("shared", V::Null),
        ("ppm", V::opt(n(12).and_then(|r| ppm_of(mz, r)))),
        ("name", V::s(nm)),
        ("cleanName", V::Str(clean_name(nm))),
        (
            "adduct",
            V::Str(if x.ix[4].is_some() {
                norm_adduct(cell(r, x.ix[4]))
            } else {
                String::new()
            }),
        ),
        ("formula", V::s(s(5))),
        ("smiles", V::s(s(7))),
        ("ik", V::s(s(6))),
        ("analog", V::Bool(false)),
        ("propagated", V::Bool(false)),
        ("external", V::Bool(true)),
        ("notes", V::List(vec![])),
    ] {
        a.set(k, v);
    }
    a
}

/// Names that are not annotations: «Unknown», «w/o MS2», «Low score», «No MS2».
fn is_placeholder(nm: &str) -> bool {
    let low = nm.to_lowercase();
    ["unknown", "w/o ms2", "low score", "no ms2"]
        .iter()
        .any(|p| low.starts_with(p))
}

pub fn read_msdial(text: &str, name: &str, pol: i32) -> Result<Table, ImportError> {
    let mut t = Table::new("msdial", name, pol);
    let accept = |c: &[String]| c.iter().any(|x| jstrim(x).to_lowercase() == "alignment id");
    let state = each_row(text, b'\t', Some(&accept), columns, |x, r, _| {
        let (id, mz, rt) = (
            txt(cell(r, x.ix[0])),
            num(cell(r, x.ix[2])),
            num(cell(r, x.ix[1])),
        );
        let (Some(mz), Some(rt)) = (mz, rt) else {
            return Ok(());
        };
        if id.is_empty() {
            return Ok(());
        }
        let nm = if x.ix[3].is_some() {
            txt(cell(r, x.ix[3]))
        } else {
            ""
        };
        let ann = (!nm.is_empty() && !is_placeholder(nm)).then(|| annotation(x, r, mz, nm));
        let f = t.add_feature(id, mz, rt);
        for (s, i) in &x.samples {
            if let Some(a) = num(cell(r, Some(*i))).filter(|a| *a != 0.0) {
                f.set_area(s, a);
            }
        }
        if x.ix[11].is_some() {
            let sp = parse_spectrum(cell(r, x.ix[11]));
            if !sp.mz.is_empty() {
                f.ms2 = Some(sp);
            }
        }
        f.ann.extend(ann);
        Ok(())
    })?;
    let Some(x) = state else {
        return Err(ImportError::new("columns", "MS-DIAL header not found"));
    };
    t.samples = x.samples.into_iter().map(|s| s.0).collect();
    t.stat("features", t.features.len() as f64);
    t.stat(
        "ms2",
        t.features.iter().filter(|f| f.ms2.is_some()).count() as f64,
    );
    Ok(t)
}
