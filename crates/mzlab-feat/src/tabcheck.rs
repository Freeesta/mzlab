//! Deterministic checks of an aligned table, `tabcheck/1` (`docs/schemi/tabcheck-1.json`): fifteen measurements, each with its value, the threshold it is
//! read against, a flag (`ok|note|warn`), up to 20 example rows and the numbers used. Numbers only: nothing here says what a row is. The assistant reads this
//! text instead of the table (`docs/studi/assistente-dati/04-feature-table.md`); the areas are the measured ones, the rows those of the view when there is one.
//! Same table and options give the same bytes; the text is cut to `MAX_TOKENS` estimated tokens by shrinking the example rows and pairs, never the checks.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{round, Out};

use crate::aligned::Aligned;
use crate::norm::measured;
use crate::samples::Sample;
use crate::view::View;

mod basic;
mod pairs;
mod series;

pub const SCHEMA: &str = "tabcheck/1";
/// Estimated tokens of the whole text (`ceil(bytes / 3)`, as `contesto/1`).
pub const MAX_TOKENS: usize = 6000;
/// Example rows of a check in the full text.
pub const ROWS_CAP: usize = 20;
/// Pairs listed in the proof of a pair check in the full text.
pub const PAIRS_CAP: usize = 5;
/// FWHM of a Gaussian peak over its base width (right - left), the width the table keeps per cell.
const FWHM_OF_WIDTH: f64 = 0.59;
/// RT window, minutes, when the table has no width to take the median of.
const FALLBACK_FWHM: f64 = 0.1;
/// m/z tolerance from which a table is read as unit resolution (isotopes 1 Da, wide windows for the adducts).
const UNIT_TOL_DA: f64 = 0.1;

/// What the caller knows besides the table.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    /// Polarity the table was made with: negative for ESI-, otherwise ESI+ (it picks the adduct differences).
    pub polarity: i32,
    /// Polarity found by the inspection of the files (`ispezione/1`), when there is one.
    pub expected_polarity: Option<i32>,
    /// m/z tolerance of the alignment, Da (0.005 in HR; 0.3 for unit resolution).
    pub mz_tol_da: f64,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            polarity: 1,
            expected_polarity: None,
            mz_tol_da: 0.005,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    Ok,
    Note,
    Warn,
}

impl Flag {
    pub fn name(self) -> &'static str {
        match self {
            Flag::Ok => "ok",
            Flag::Note => "note",
            Flag::Warn => "warn",
        }
    }
}

/// One check. `proof` is a JSON object text; `rows` are ids of the table, worst first; `files` are names.
#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    pub id: &'static str,
    pub value: Option<f64>,
    pub threshold: Option<f64>,
    pub flag: Flag,
    pub rows: Vec<u32>,
    pub files: Vec<String>,
    pub proof: String,
}

impl Check {
    /// The check could not be made for lack of `what` (a role, files, a resolution): a note with no value, never a guess.
    fn missing(id: &'static str, what: &str) -> Check {
        let mut p = Proof::new();
        p.s("missing", what);
        Check {
            id,
            value: None,
            threshold: None,
            flag: Flag::Note,
            rows: Vec::new(),
            files: Vec::new(),
            proof: p.done(),
        }
    }
}

/// How many example rows and pairs the text carries.
#[derive(Debug, Clone, Copy)]
struct Caps {
    rows: usize,
    pairs: usize,
}

/// A JSON object under construction (the proof of a check).
struct Proof(Out, bool);

impl Proof {
    fn new() -> Proof {
        Proof(Out::default(), false)
    }

    fn key(&mut self, k: &str) -> &mut Out {
        self.0.raw(if self.1 { "," } else { "{" }).str(k).raw(":");
        self.1 = true;
        &mut self.0
    }

    fn n(&mut self, k: &str, v: f64) -> &mut Proof {
        self.key(k).num(round(v, 5));
        self
    }

    fn i(&mut self, k: &str, v: usize) -> &mut Proof {
        self.key(k).int(v as i64);
        self
    }

    fn s(&mut self, k: &str, v: &str) -> &mut Proof {
        self.key(k).str(v);
        self
    }

    fn raw(&mut self, k: &str, v: &str) -> &mut Proof {
        self.key(k).raw(v);
        self
    }

    fn done(mut self) -> String {
        self.0.raw(if self.1 { "}" } else { "{}" });
        self.0 .0
    }
}

/// The table and what the checks share: the rows considered, the files by role and the RT window of one peak.
struct Env<'a> {
    t: &'a Aligned,
    s: &'a [Sample],
    opts: &'a Options,
    caps: Caps,
    /// Indices of the rows considered, ascending.
    rows: Vec<usize>,
    /// Files that are not `dark`.
    live: Vec<usize>,
    blanks: Vec<usize>,
    qcs: Vec<usize>,
    /// Live files that are neither blank nor QC.
    samples: Vec<usize>,
    /// Median FWHM of the peaks, minutes: the window inside which two rows co-elute.
    fwhm: f64,
}

impl<'a> Env<'a> {
    fn new(
        t: &'a Aligned,
        s: &'a [Sample],
        view: Option<&View>,
        opts: &'a Options,
        caps: Caps,
    ) -> Env<'a> {
        let rows: Vec<usize> = view.map_or_else(
            || (0..t.len()).collect(),
            |v| v.rows.iter().map(|&r| r as usize).collect(),
        );
        let of = |f: &dyn Fn(&Sample) -> bool| {
            (0..s.len()).filter(|&k| f(&s[k])).collect::<Vec<usize>>()
        };
        let live = of(&|x| x.kind != "dark");
        let blanks = of(&|x| x.kind == "blank");
        let qcs = of(&|x| x.kind == "qc");
        let samples = of(&|x| !matches!(x.kind.as_str(), "blank" | "dark" | "qc"));
        let widths: Vec<f64> = rows
            .iter()
            .flat_map(|&r| (0..t.n_files).map(move |k| (r, k)))
            .filter(|&(r, k)| t.state[r * t.n_files + k] == crate::aligned::CELL_DETECTED)
            .map(|(r, k)| f64::from(t.width[r * t.n_files + k]))
            .filter(|w| w.is_finite() && *w > 0.0)
            .collect();
        let fwhm = if widths.is_empty() {
            FALLBACK_FWHM
        } else {
            median(widths) * FWHM_OF_WIDTH
        };
        Env {
            t,
            s,
            opts,
            caps,
            rows,
            live,
            blanks,
            qcs,
            samples,
            fwhm,
        }
    }

    fn id(&self, row: usize) -> u32 {
        self.t.id(row)
    }

    fn area(&self, row: usize, file: usize) -> Option<f64> {
        measured(self.t, row, file)
    }

    /// Areas of `row` in `files`, 0 where the file has no peak.
    fn areas(&self, row: usize, files: &[usize]) -> Vec<f64> {
        files
            .iter()
            .map(|&k| self.area(row, k).unwrap_or(0.0))
            .collect()
    }

    fn present_in_all(&self, row: usize, files: &[usize]) -> bool {
        files.iter().all(|&k| self.area(row, k).is_some())
    }

    /// Ids of `rows` (indices), cut to the example cap.
    fn ids(&self, rows: &[usize]) -> Vec<u32> {
        rows.iter()
            .take(self.caps.rows)
            .map(|&r| self.id(r))
            .collect()
    }

    fn names(&self, files: &[usize]) -> Vec<String> {
        files
            .iter()
            .take(self.caps.rows)
            .map(|&k| self.s[k].file.clone())
            .collect()
    }

    /// Whether the table is read as unit resolution.
    fn unit_resolution(&self) -> bool {
        self.opts.mz_tol_da >= UNIT_TOL_DA
    }

    /// Rows considered by ascending m/z, then id.
    fn by_mz(&self) -> Vec<usize> {
        let mut v = self.rows.clone();
        crate::sortc::by(&mut v, |&a, &b| {
            self.t.mz[a].total_cmp(&self.t.mz[b]).then(a.cmp(&b))
        });
        v
    }

    fn rt_close(&self, a: usize, b: usize) -> bool {
        (self.t.rt[a] - self.t.rt[b]).abs() <= self.fwhm
    }
}

/// Sorts (id, key) pairs by key, descending if asked, ties by id; one instance for every ranking of the checks.
fn rank(v: &mut [(usize, f64)], desc: bool) {
    crate::sortc::by(v, |a, b| {
        let o = a.1.total_cmp(&b.1);
        (if desc { o.reverse() } else { o }).then(a.0.cmp(&b.0))
    });
}

pub(crate) fn median(mut v: Vec<f64>) -> f64 {
    crate::sortc::by(&mut v, f64::total_cmp);
    match v.len() {
        0 => f64::NAN,
        n if n % 2 == 1 => v[n / 2],
        n => (v[n / 2 - 1] + v[n / 2]) / 2.0,
    }
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

/// Standard deviation over the mean (sample standard deviation, as the `qc_cv_max` rule); NaN with fewer than two values or a zero mean.
fn cv(v: &[f64]) -> f64 {
    if v.len() < 2 {
        return f64::NAN;
    }
    let m = mean(v);
    (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (v.len() - 1) as f64).sqrt() / m
}

/// Pearson correlation; None for fewer than two points or no variance in either series.
fn pearson(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() < 2 || x.len() != y.len() {
        return None;
    }
    let (mx, my) = (mean(x), mean(y));
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (a, b) in x.iter().zip(y) {
        sxy += (a - mx) * (b - my);
        sxx += (a - mx).powi(2);
        syy += (b - my) * (b - my);
    }
    (sxx > 0.0 && syy > 0.0).then(|| sxy / (sxx * syy).sqrt())
}

/// Slope of the least-squares line of `y` on `x`; None without variance in `x`.
fn slope(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() < 2 {
        return None;
    }
    let (mx, my) = (mean(x), mean(y));
    let sxx: f64 = x.iter().map(|a| (a - mx).powi(2)).sum();
    (sxx > 0.0).then(|| {
        x.iter()
            .zip(y)
            .map(|(a, b)| (a - mx) * (b - my))
            .sum::<f64>()
            / sxx
    })
}

/// The fifteen checks of `t` in the fixed order of `IDS` (one `Sample` per file; `view` limits the rows).
pub fn checks(
    t: &Aligned,
    samples: &[Sample],
    view: Option<&View>,
    opts: &Options,
) -> Result<Vec<Check>> {
    let (_, c) = analyse(
        t,
        samples,
        view,
        opts,
        Caps {
            rows: ROWS_CAP,
            pairs: PAIRS_CAP,
        },
    )?;
    Ok(c)
}

/// The summary of the table and the checks under the caps.
fn analyse(
    t: &Aligned,
    samples: &[Sample],
    view: Option<&View>,
    opts: &Options,
    caps: Caps,
) -> Result<(Summary, Vec<Check>)> {
    if samples.len() != t.n_files {
        return Err(Error::new("error.badView"));
    }
    let e = Env::new(t, samples, view, opts, caps);
    let list = if e.rows.is_empty() {
        IDS.iter().map(|&id| Check::missing(id, "rows")).collect()
    } else {
        vec![
            basic::size(&e),
            basic::blank(&e),
            basic::contaminants(&e),
            basic::qc_cv(&e),
            basic::qc_intensity(&e),
            basic::rt_drift(&e),
            pairs::duplicates(&e),
            pairs::adducts(&e),
            pairs::isotopes(&e),
            pairs::insource(&e),
            basic::long_traces(&e),
            series::homologous(&e),
            basic::missing_cells(&e),
            series::structure(&e),
            basic::polarity_units(&e),
        ]
    };
    let sum = Summary {
        rows: t.len(),
        considered: e.rows.len(),
        files: t.n_files,
        blanks: e.blanks.len(),
        qc: e.qcs.len(),
        samples: e.samples.len(),
        fwhm: e.fwhm,
        high_resolution: !e.unit_resolution(),
        polarity: if opts.polarity < 0 { -1 } else { 1 },
    };
    Ok((sum, list))
}

/// Ids of the checks in the order of the text.
pub const IDS: [&str; 15] = [
    "size",
    "blank",
    "contaminants",
    "qc_cv",
    "qc_intensity",
    "rt_drift",
    "duplicates",
    "adducts",
    "isotopes",
    "insource",
    "long_traces",
    "homologous",
    "missing",
    "structure",
    "polarity_units",
];

struct Summary {
    rows: usize,
    considered: usize,
    files: usize,
    blanks: usize,
    qc: usize,
    samples: usize,
    fwhm: f64,
    high_resolution: bool,
    polarity: i32,
}

fn write(sum: &Summary, list: &[Check], truncated: bool) -> String {
    let mut o = Out::default();
    o.raw("{\"schema\":")
        .str(SCHEMA)
        .raw(",\"table\":{\"rows\":")
        .int(sum.rows as i64);
    o.raw(",\"considered\":")
        .int(sum.considered as i64)
        .raw(",\"files\":")
        .int(sum.files as i64);
    o.raw(",\"blanks\":")
        .int(sum.blanks as i64)
        .raw(",\"qc\":")
        .int(sum.qc as i64)
        .raw(",\"samples\":")
        .int(sum.samples as i64);
    o.raw(",\"fwhm_min\":")
        .num(round(sum.fwhm, 5))
        .raw(",\"high_resolution\":")
        .raw(if sum.high_resolution { "true" } else { "false" });
    o.raw(",\"polarity\":")
        .int(i64::from(sum.polarity))
        .raw("},\"truncated\":")
        .raw(if truncated { "true" } else { "false" });
    o.raw(",\"summary\":{");
    for (i, f) in [Flag::Ok, Flag::Note, Flag::Warn].iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .str(f.name())
            .raw(":")
            .int(list.iter().filter(|c| c.flag == *f).count() as i64);
    }
    o.raw("},\"checks\":[");
    for (i, c) in list.iter().enumerate() {
        o.raw(if i > 0 { ",{\"id\":" } else { "{\"id\":" })
            .str(c.id);
        o.raw(",\"value\":")
            .opt(c.value.map(|v| round(v, 5)))
            .raw(",\"threshold\":")
            .opt(c.threshold.map(|v| round(v, 5)));
        o.raw(",\"flag\":").str(c.flag.name()).raw(",\"rows\":[");
        for (j, r) in c.rows.iter().enumerate() {
            o.raw(if j > 0 { "," } else { "" }).int(i64::from(*r));
        }
        o.raw("],\"files\":[");
        for (j, f) in c.files.iter().enumerate() {
            o.raw(if j > 0 { "," } else { "" }).str(f);
        }
        o.raw("],\"proof\":").raw(&c.proof).raw("}");
    }
    o.raw("]}");
    o.0
}

/// Estimated tokens of a text (`ceil(bytes / 3)`).
pub fn tokens(text: &str) -> usize {
    text.len().div_ceil(3)
}

/// The `tabcheck/1` text of `t`. Over `MAX_TOKENS` the example rows and pairs are halved until it fits (`truncated` true); the checks stay.
pub fn to_json(
    t: &Aligned,
    samples: &[Sample],
    view: Option<&View>,
    opts: &Options,
) -> Result<String> {
    let mut caps = Caps {
        rows: ROWS_CAP,
        pairs: PAIRS_CAP,
    };
    loop {
        let (sum, list) = analyse(t, samples, view, opts, caps)?;
        let truncated = caps.rows < ROWS_CAP || caps.pairs < PAIRS_CAP;
        let text = write(&sum, &list, truncated);
        if tokens(&text) <= MAX_TOKENS || (caps.rows == 0 && caps.pairs == 0) {
            return Ok(text);
        }
        caps = Caps {
            rows: caps.rows / 2,
            pairs: caps.pairs.saturating_sub(1),
        };
    }
}
