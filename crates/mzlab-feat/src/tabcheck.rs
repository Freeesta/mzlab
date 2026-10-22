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
mod cells;
mod env;
mod num;
mod pairs;
#[cfg(not(target_arch = "wasm32"))]
mod series;
/// The page's .wasm has no room for the Kendrick analysis and the PCA: those two checks answer `unavailable` with the i18n key of the reason.
#[cfg(target_arch = "wasm32")]
mod series {
    use super::*;

    pub(super) fn homologous(_: &Env) -> Check {
        Check::unavailable("homologous")
    }

    pub(super) fn structure(_: &Env) -> Check {
        Check::unavailable("structure")
    }
}

use env::{Caps, Env, Proof};
pub(crate) use num::median;
use num::{cv, mean, pearson, rank, slope};

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
    /// The check is left out of this build (the page's .wasm); the proof names the reason.
    Unavailable,
}

impl Flag {
    pub fn name(self) -> &'static str {
        match self {
            Flag::Ok => "ok",
            Flag::Note => "note",
            Flag::Warn => "warn",
            Flag::Unavailable => "unavailable",
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
    /// A check this build does not carry: flag `unavailable`, no value, the i18n key of the reason in the proof.
    #[cfg(target_arch = "wasm32")]
    fn unavailable(id: &'static str) -> Check {
        let mut p = Proof::new();
        p.s("unavailable", "tabcheck.unavailable.wasm");
        Check {
            id,
            value: None,
            threshold: None,
            flag: Flag::Unavailable,
            rows: Vec::new(),
            files: Vec::new(),
            proof: p.done(),
        }
    }

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
    let e = env::build(t, samples, view, opts, caps);
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
            cells::long_traces(&e),
            series::homologous(&e),
            cells::missing_cells(&e),
            series::structure(&e),
            cells::polarity_units(&e),
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
    for (i, f) in [Flag::Ok, Flag::Note, Flag::Warn, Flag::Unavailable]
        .iter()
        .enumerate()
    {
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
