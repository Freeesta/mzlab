//! `mzlab/statistica.py` (part 2): the difference of every feature of a table from a reference (`reference_difference`) and the links
//! between features that could share an identity (`identity_links`). Numbers to sort and filter, never a verdict.

use std::cmp::Ordering::Equal;

use crate::elements::{mass, parse_formula};
use crate::json::Value;
use crate::jsonb::{int, num, opt, text, Obj};
use crate::roles::{ADDUCTS, NEUTRAL_LOSSES};
use crate::stats::{psum, rankdata};
use crate::statistica::{blank_filter, bad, mean, ncols, user, BlankOut, R};

pub const FLOOR: f64 = 1000.0;
const NOISE_FACTOR: f64 = 3.0;
const NOT_TREATED: [&str; 4] = ["blank", "dark", "qc", "standard"];

pub struct RefOpts {
    pub factor: f64,
    pub ref_stat: String,
    pub min_detect: i64,
    pub pseudo: Option<f64>,
    pub floor: f64,
    pub rho_min: f64,
    pub drop: f64,
}

impl Default for RefOpts {
    fn default() -> Self {
        RefOpts { factor: 3.0, ref_stat: "max".into(), min_detect: 1, pseudo: None, floor: FLOOR, rho_min: 0.5, drop: 0.5 }
    }
}

/// The table and the metadata of its files: roles, times (minutes), replicate labels, the columns chosen as reference (None = by rule).
pub struct RefIn<'a> {
    pub matrix: &'a [Vec<f64>],
    pub roles: &'a [String],
    pub times: &'a [Option<f64>],
    pub groups: Option<&'a [Option<String>]>,
    pub reference: Option<&'a [i64]>,
}

/// What every feature needs, worked out once.
struct Ctx<'a> {
    inp: &'a RefIn<'a>,
    tr: Vec<usize>,
    ser: Vec<f64>,
    cols_at: Vec<Vec<usize>>,
    lab: Vec<Option<String>>,
    pseudo: f64,
    bf: BlankOut,
    o: &'a RefOpts,
}

fn median(mut v: Vec<f64>) -> f64 {
    // std on purpose: partial_cmp(..).unwrap_or(Equal) is not a total order if a NaN arrives, and sortc could order those rows differently
    #[allow(clippy::disallowed_methods)]
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Equal));
    let n = v.len();
    if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) / 2.0 }
}

fn ptp(v: &[f64]) -> f64 {
    v.iter().cloned().fold(f64::NEG_INFINITY, f64::max) - v.iter().cloned().fold(f64::INFINITY, f64::min)
}

/// `_spearman`: Pearson of the ranks (`np.corrcoef`); None below three points or when a series is constant.
fn spearman(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() < 3 || ptp(x) == 0.0 || ptp(y) == 0.0 {
        return None;
    }
    let (a, b) = (rankdata(x), rankdata(y));
    let (ma, mb) = (mean(&a), mean(&b));
    let (da, db): (Vec<f64>, Vec<f64>) = (a.iter().map(|v| v - ma).collect(), b.iter().map(|v| v - mb).collect());
    let k = (a.len() - 1) as f64;
    let dot = |p: &[f64], q: &[f64]| psum(&p.iter().zip(q).map(|(u, w)| u * w).collect::<Vec<_>>()) / k;
    let (sa, sb) = (dot(&da, &da).sqrt(), dot(&db, &db).sqrt());
    Some(((dot(&da, &db) / sa) / sb).clamp(-1.0, 1.0))
}

/// `_shape`: a descriptive label of a time course, only to sort and filter.
fn shape(y: &[f64], rho: Option<f64>, floor: f64, rho_min: f64, drop: f64) -> Option<&'static str> {
    let top = y.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let rho = rho?;
    if y.len() < 3 || top <= floor {
        return None;
    }
    let m = argmax(y);
    let last = y.len() - 1;
    Some(if m > 0 && m < last && y[last] <= drop * y[m] {
        "rises_falls"
    } else if rho >= rho_min && y[last] > y[0] {
        "rises"
    } else if rho <= -rho_min && y[last] < y[0] {
        "falls"
    } else {
        "flat"
    })
}

fn argmax(y: &[f64]) -> usize {
    (0..y.len()).fold(0, |b, i| if y[i] > y[b] { i } else { b })
}

fn check_inputs(inp: &RefIn, o: &RefOpts) -> R<()> {
    if o.ref_stat != "max" && o.ref_stat != "mean" {
        return user("err.stat.blank.stat", "stat must be 'max' or 'mean'");
    }
    if !(o.factor >= 0.0) || o.min_detect < 1 || !(o.floor >= 0.0) || o.pseudo.map_or(false, |p| !(p >= 0.0)) {
        return user("err.stat.blank.param", "factor must be >= 0 and the minimum of detections >= 1");
    }
    let n = inp.roles.len();
    let wrong = inp.times.len() != n || (!inp.matrix.is_empty() && ncols(inp.matrix) != n) || inp.groups.map_or(false, |g| g.len() != n);
    if wrong {
        return user("err.stat.ref.shape", "the table, the roles and the times must have one entry per file");
    }
    Ok(())
}

/// The columns that are the reference: chosen by the user, else `default_reference` (time <= 0, blank, dark).
fn reference_columns(inp: &RefIn) -> R<Vec<usize>> {
    let n = inp.roles.len() as i64;
    let mut ref_cols: Vec<usize> = match inp.reference {
        Some(r) => {
            if r.iter().any(|&i| i < 0 || i >= n) {
                return user("err.stat.ref.shape", "the table, the roles and the times must have one entry per file");
            }
            r.iter().map(|&i| i as usize).collect()
        }
        None => (0..n as usize).filter(|&i| inp.roles[i] == "blank" || inp.roles[i] == "dark" || inp.times[i].map_or(false, |t| t <= 0.0)).collect(),
    };
    crate::sortc::slice_ord(&mut ref_cols);
    ref_cols.dedup();
    if ref_cols.is_empty() {
        return user("err.stat.ref.none", "no reference: mark a blank or give a file at time 0");
    }
    Ok(ref_cols)
}

/// The pseudocount (area units) and where it comes from: given, the noise of the blanks, of the reference, or the floor.
fn pseudocount(inp: &RefIn, ref_cols: &[usize], o: &RefOpts) -> (f64, &'static str) {
    if let Some(p) = o.pseudo {
        return (p, "given");
    }
    let mut bl: Vec<usize> = ref_cols.iter().cloned().filter(|&i| inp.roles[i] == "blank").collect();
    if bl.is_empty() {
        bl = (0..inp.roles.len()).filter(|&i| inp.roles[i] == "blank").collect();
    }
    for (cols, name) in [(bl.as_slice(), "blanks"), (ref_cols, "reference")] {
        let pos: Vec<f64> = inp.matrix.iter().flat_map(|r| cols.iter().map(move |&c| r[c])).filter(|&v| v > 0.0).collect();
        if !pos.is_empty() {
            return (o.floor.max(NOISE_FACTOR * median(pos)), name);
        }
    }
    (o.floor, "floor")
}

fn setup<'a>(inp: &'a RefIn<'a>, o: &'a RefOpts, ref_cols: &[usize]) -> R<(Ctx<'a>, &'static str)> {
    let n = inp.roles.len();
    let tr: Vec<usize> = (0..n).filter(|i| !ref_cols.contains(i) && !NOT_TREATED.contains(&inp.roles[*i].as_str())).collect();
    if tr.is_empty() {
        return user("err.stat.ref.notreated", "no treated file besides the reference");
    }
    let (pseudo, src) = pseudocount(inp, ref_cols, o);
    let mut r2 = vec!["dark".to_string(); n];
    ref_cols.iter().for_each(|&i| r2[i] = "blank".into());
    tr.iter().for_each(|&i| r2[i] = "sample".into());
    let bf = if inp.matrix.is_empty() {
        BlankOut { kept: vec![], discarded: vec![], rows: vec![], n_blank: 0, n_sample: 0 }
    } else {
        blank_filter(inp.matrix, &r2, o.factor, &o.ref_stat, o.min_detect)?
    };
    let samples = |i: usize| inp.roles[i] == "sample" && inp.times[i].is_some();
    let mut ser: Vec<f64> = (0..n).filter(|&i| samples(i)).map(|i| inp.times[i].unwrap()).collect();
    // std on purpose: partial_cmp(..).unwrap_or(Equal) is not a total order if a NaN arrives, and sortc could order those rows differently
    #[allow(clippy::disallowed_methods)]
    ser.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Equal));
    ser.dedup();
    let cols_at = ser.iter().map(|&t| (0..n).filter(|&i| samples(i) && inp.times[i] == Some(t)).collect()).collect();
    let lab = inp.groups.map_or(vec![None; n], |g| g.iter().map(|x| x.clone().filter(|s| !s.is_empty())).collect());
    Ok((Ctx { inp, tr, ser, cols_at, lab, pseudo, bf, o }, src))
}

impl Ctx<'_> {
    /// The replicates of file `i0`: same label, or (both unlabelled) same time; else the file alone.
    fn replicates(&self, i0: usize) -> Vec<usize> {
        let (lab, tm) = (&self.lab, self.inp.times);
        let same: Vec<usize> = self.tr.iter().cloned().filter(|&i| {
            (lab[i0].is_some() && lab[i] == lab[i0]) || (lab[i0].is_none() && lab[i].is_none() && tm[i0].is_some() && tm[i] == tm[i0])
        }).collect();
        if same.is_empty() { vec![i0] } else { same }
    }

    fn feature(&self, f: usize) -> Value {
        let (a, q) = (&self.inp.matrix[f], &self.bf.rows[f]);
        let top_i = self.tr.iter().cloned().fold(self.tr[0], |b, i| if a[i] > a[b] { i } else { b });
        let top = a[top_i];
        let y: Vec<f64> = self.cols_at.iter().map(|c| mean(&c.iter().map(|&i| a[i]).collect::<Vec<_>>())).collect();
        let rho = spearman(&self.ser, &y);
        let rg: Vec<f64> = self.replicates(top_i).iter().map(|&i| a[i]).collect();
        let m = mean(&rg);
        let absent = q.blank <= self.pseudo;
        let kept = self.bf.kept.contains(&f);
        let reason = if kept {
            if absent { "absent_in_reference" } else { "above_threshold" }
        } else if top <= 0.0 {
            "no_signal"
        } else if q.n_above == 0 {
            "below_reference"
        } else {
            "too_few_files"
        };
        let ytop = y.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let cv = (rg.len() > 1 && m > 0.0).then(|| {
            let ss = psum(&rg.iter().map(|v| (v - m) * (v - m)).collect::<Vec<_>>());
            (ss / (rg.len() - 1) as f64).sqrt() / m * 100.0
        });
        let sh = shape(&y, rho, self.pseudo, self.o.rho_min, self.o.drop);
        Obj::new().n("ref_level", q.blank).n("threshold", q.threshold).n("treated_max", top).i("max_file", top_i as i64)
            .put("ratio", opt((q.blank != 0.0).then(|| top / q.blank))).n("log2fc", ((top + self.pseudo) / (q.blank + self.pseudo)).log2())
            .b("absent_in_reference", absent).i("n_above", q.n_above as i64).i("n_treated", self.tr.len() as i64)
            .put("shape", sh.map_or(Value::Null, text)).put("rho", opt(rho))
            .put("t_max", opt((!y.is_empty() && ytop > 0.0).then(|| self.ser[argmax(&y)]))).n("rep_mean", m).put("rep_cv", opt(cv))
            .i("rep_n_detected", rg.iter().filter(|&&v| v > 0.0).count() as i64).i("rep_n", rg.len() as i64).b("kept", kept).put("reason", text(reason)).done()
    }
}

/// `reference_difference`: one row per feature (see `statistica.py` for the meaning of every number); both lists are returned.
pub fn reference_difference(inp: &RefIn, o: &RefOpts) -> R<Value> {
    check_inputs(inp, o)?;
    let explicit = inp.reference.is_some();
    let ref_cols = reference_columns(inp)?;
    let (c, src) = setup(inp, o, &ref_cols)?;
    let idx = |v: &[usize]| Value::Arr(v.iter().map(|&i| int(i as i64)).collect());
    let rows: Vec<Value> = (0..inp.matrix.len()).map(|f| c.feature(f)).collect();
    let params = Obj::new().n("factor", o.factor).put("ref_stat", text(&o.ref_stat)).i("min_detect", o.min_detect).n("pseudo", c.pseudo)
        .put("pseudo_source", text(src)).n("floor", o.floor).n("noise_factor", NOISE_FACTOR).n("rho_min", o.rho_min).n("drop", o.drop).done();
    Ok(Obj::new().put("reference", idx(&ref_cols)).put("reference_rule", text(if explicit { "explicit" } else { "metadata" })).put("treated", idx(&c.tr))
        .put("times", Value::Arr(c.ser.iter().map(|&t| num(t)).collect())).put("kept", idx(&c.bf.kept)).put("discarded", idx(&c.bf.discarded))
        .put("rows", Value::Arr(rows)).put("params", params).done())
}

// ------------------------------------------------------------------------------------------------ identity links
/// For every feature, the other features it could share an identity with (adducts of the same neutral, in-source fragments); only shown.
pub fn identity_links(mz: &[f64], rt: &[f64]) -> R<Value> {
    let (ppm, rt_tol) = (5.0, 0.1);
    let n = mz.len();
    if rt.len() != n {
        return bad("mz and rt must have the same length");
    }
    let singles: Vec<(&str, f64)> = ADDUCTS.iter().filter(|a| a.2 == 1.0).map(|a| (a.0, a.1)).collect();
    let pairs: Vec<(&str, &str, f64)> = (0..singles.len()).flat_map(|i| (i + 1..singles.len()).map(move |j| (i, j)))
        .filter(|&(i, j)| singles[j].1 != singles[i].1).map(|(i, j)| (singles[i].0, singles[j].0, singles[j].1 - singles[i].1)).collect();
    let losses: Vec<(&str, f64)> = NEUTRAL_LOSSES.iter().map(|&f| (f, mass(&parse_formula(f).unwrap_or_default()))).collect();
    let mut order: Vec<usize> = (0..n).collect();
    // std on purpose: partial_cmp(..).unwrap_or(Equal) is not a total order if a NaN arrives, and sortc could order those rows differently
    #[allow(clippy::disallowed_methods)]
    order.sort_by(|&a, &b| mz[a].partial_cmp(&mz[b]).unwrap_or(Equal));
    let sm: Vec<f64> = order.iter().map(|&i| mz[i]).collect();
    let near = |tgt: f64| {
        let lo = sm.partition_point(|&v| v < tgt * (1.0 - ppm * 1e-6));
        let hi = sm.partition_point(|&v| v <= tgt * (1.0 + ppm * 1e-6));
        order[lo..hi.max(lo)].to_vec()
    };
    let mut out: Vec<Vec<Value>> = vec![Vec::new(); n];
    let link = |kind: &str, other: usize, a: &str, b: &str| Obj::new().put("kind", text(kind)).i("other", other as i64).put("as", text(a)).put("other_as", text(b)).done();
    for i in 0..n {
        for &(lo_name, hi_name, d) in &pairs {
            for j in near(mz[i] + d).into_iter().filter(|&j| j != i && (rt[j] - rt[i]).abs() <= rt_tol) {
                out[i].push(link("adduct", j, lo_name, hi_name));
                out[j].push(link("adduct", i, hi_name, lo_name));
            }
        }
        for &(f, ml) in &losses {
            for j in near(mz[i] + ml).into_iter().filter(|&j| j != i && (rt[j] - rt[i]).abs() <= rt_tol) {
                out[i].push(Obj::new().put("kind", text("isf")).i("other", j as i64).put("loss", text(f)).done());
            }
        }
    }
    Ok(Value::Arr(out.into_iter().map(Value::Arr).collect()))
}
