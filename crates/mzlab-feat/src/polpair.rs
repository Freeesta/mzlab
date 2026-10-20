//! The proof "same neutral mass in the other polarity" (`polpair/1`): rows of a positive and a negative table of the same files whose neutral masses agree
//! under the usual adduct pairs, at about the same RT and with the same course along the series. One proof more, never a verdict; the two tables stay
//! separate. The adduct forms and the Pearson r are the ones of `legacy::identity::polarity_pairs`.

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::json::{self, Out, Value};

use crate::legacy::identity::{form_shift, shape_r};
use crate::recipe::is_unit_text;
use crate::sortc::SortC;
use crate::store::read_table_from;
use crate::store_read::origin_from_meta;
use crate::table::Aligned;

const POS_FORMS: [&str; 3] = ["[M+H]+", "[M+Na]+", "[M+NH4]+"];
const NEG_FORMS: [&str; 3] = ["[M-H]-", "[M+FA-H]-", "[M+Cl]-"];

/// Tolerances of the match: ppm of the neutral mass, minutes of RT, minimum Pearson r over at least `min_common` files (`trend` off: no filter on the course).
#[derive(Debug, Clone, PartialEq)]
pub struct PolParams {
    pub ppm: f64,
    pub rt_tol: f64,
    pub trend: bool,
    pub min_r: f64,
    pub min_common: usize,
}

impl Default for PolParams {
    fn default() -> Self {
        PolParams {
            ppm: 5.0,
            rt_tol: 0.15,
            trend: true,
            min_r: 0.8,
            min_common: 4,
        }
    }
}

/// One pair: rows (positions in the tables), ion forms, difference of neutral mass (ppm, negative minus positive), of RT (min), and the correlation.
#[derive(Debug, Clone, PartialEq)]
pub struct Pair {
    pub row_pos: usize,
    pub row_neg: usize,
    pub ion_pos: &'static str,
    pub ion_neg: &'static str,
    pub ppm: f64,
    pub d_rt: f64,
    pub r: Option<f64>,
    pub n_common: usize,
}

fn areas(t: &Aligned, row: usize) -> &[f64] {
    &t.area[row * t.n_files..(row + 1) * t.n_files]
}

/// Every pair of a row of `pos` and a row of `neg`, ordered by positive row and then by |ppm|.
pub fn pairs(pos: &Aligned, neg: &Aligned, p: &PolParams) -> Vec<Pair> {
    let shifts = |pol: i32, names: &[&'static str]| -> Vec<(&'static str, f64, f64)> {
        names
            .iter()
            .filter_map(|&n| form_shift(pol, n).map(|(s, m)| (n, s, m)))
            .collect()
    };
    let (fp, fneg) = (shifts(1, &POS_FORMS), shifts(-1, &NEG_FORMS));
    // neutral masses of the negative rows under each form, sorted
    let mut ns: Vec<(f64, &'static str, usize)> = Vec::new();
    for row in 0..neg.len() {
        for &(name, sh, mult) in &fneg {
            ns.push(((neg.mz[row] - sh) / mult, name, row));
        }
    }
    ns.sortc_by(|a, b| a.0.total_cmp(&b.0));
    let mut out: Vec<Pair> = Vec::new();
    for row in 0..pos.len() {
        let mut mine: Vec<Pair> = Vec::new();
        for &(name, sh, mult) in &fp {
            let m = (pos.mz[row] - sh) / mult;
            let tol = m * p.ppm * 1e-6;
            let a = ns.partition_point(|r| r.0 < m - tol);
            let b = ns.partition_point(|r| r.0 <= m + tol);
            for &(mn, nname, nrow) in ns.get(a..b.max(a)).unwrap_or(&[]) {
                let d_rt = neg.rt[nrow] - pos.rt[row];
                if d_rt.abs() > p.rt_tol {
                    continue;
                }
                let (r, n_common, _) = shape_r(
                    areas(pos, row),
                    areas(neg, nrow),
                    if p.trend { p.min_common } else { 2 },
                );
                if p.trend && !r.is_some_and(|r| r >= p.min_r) {
                    continue;
                }
                mine.push(Pair {
                    row_pos: row,
                    row_neg: nrow,
                    ion_pos: name,
                    ion_neg: nname,
                    ppm: (mn - m) / m * 1e6,
                    d_rt,
                    r,
                    n_common,
                });
            }
        }
        mine.sortc_by(|a, b| a.ppm.abs().total_cmp(&b.ppm.abs()));
        out.extend(mine);
    }
    out
}

fn params_of(text: &str) -> Result<PolParams> {
    let mut p = PolParams::default();
    if text.trim().is_empty() {
        return Ok(p);
    }
    let v = json::parse(text.as_bytes()).map_err(|_| Error::new("error.badParams"))?;
    let num = |k: &str| v.get(k).and_then(Value::f64).filter(|x| x.is_finite());
    p.ppm = num("ppm").unwrap_or(p.ppm);
    p.rt_tol = num("rt_tol").unwrap_or(p.rt_tol);
    p.min_r = num("min_r").unwrap_or(p.min_r);
    p.min_common = num("min_common").map_or(p.min_common, |x| x.max(2.0) as usize);
    p.trend = v.get("trend").and_then(Value::bool).unwrap_or(true);
    Ok(p)
}

/// The `polpair/1` of two `.mzfeat` of the same files in the same order, one per polarity. Tables at unit resolution give no pair; files that differ are
/// `error.polpair.files`.
pub fn cross_polarity<A: ReadAt, B: ReadAt>(
    rp: &mut A,
    size_p: u64,
    rn: &mut B,
    size_n: u64,
    params_json: &str,
) -> Result<String> {
    let p = params_of(params_json)?;
    let (pos, mp) = read_table_from(rp, size_p)?;
    let (neg, mn) = read_table_from(rn, size_n)?;
    let (ip, in_) = (origin_from_meta(&mp).inputs, origin_from_meta(&mn).inputs);
    if pos.n_files != neg.n_files || ip != in_ {
        return Err(Error::new("error.polpair.files"));
    }
    let unit = |m: &Value| {
        m.get("recipe")
            .and_then(Value::str)
            .is_some_and(is_unit_text)
    };
    let found = if unit(&mp) || unit(&mn) {
        Vec::new()
    } else {
        pairs(&pos, &neg, &p)
    };
    let mut o = Out::default();
    o.raw("{\"schema\":\"polpair/1\",\"pairs\":[");
    for (i, q) in found.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        o.raw("{\"row_pos\":")
            .num(q.row_pos as f64)
            .raw(",\"row_neg\":")
            .num(q.row_neg as f64);
        o.raw(",\"id_pos\":")
            .num(f64::from(pos.id(q.row_pos)))
            .raw(",\"id_neg\":")
            .num(f64::from(neg.id(q.row_neg)));
        o.raw(",\"ion_pos\":")
            .str(q.ion_pos)
            .raw(",\"ion_neg\":")
            .str(q.ion_neg);
        o.raw(",\"ppm\":")
            .num(json::round(q.ppm, 2))
            .raw(",\"d_rt\":")
            .num(json::round(q.d_rt, 3));
        o.raw(",\"r\":");
        match q.r {
            Some(r) => o.num(json::round(r, 3)),
            None => o.raw("null"),
        };
        o.raw(",\"n_common\":").num(q.n_common as f64).raw("}");
    }
    o.raw("]}");
    Ok(o.0)
}
