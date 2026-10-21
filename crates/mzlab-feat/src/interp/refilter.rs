//! The registry of the filters (`filtri/1`) of a `.mzfeat` that `run_to` already interpreted, again under other thresholds (`tp.filter`): F1-F4 are read
//! from the table (`filtri::row_facts`), F5 from the rows of the block (`ms1.mz_obs` and its file, the neutral formula of the change) against the
//! calibration of the files in its meta, and the decoys from the hypotheses. With the default thresholds the answer is the one `build` stored.

use mzlab_core::elements::{fmt, parse_formula};
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::json::{parse, Value};

use super::filtri::{
    self, first_hidden, out_of_tolerance, table_decoys_with, DecoyParams, FileCal, Registry,
    TOL_MAX, TOL_MIN,
};
use super::store;
use crate::legacy::series::Hyp;
use crate::store_read::read_table_from;
use crate::tp::{ST_PARENT, ST_TP, ST_UNEXPLAINED};

/// The thresholds of `tp.filter`: the floor and the ceiling of the tolerance of a formula and the decoys.
#[derive(Debug, Clone, PartialEq)]
pub struct FilterParams {
    pub tol_min: f64,
    pub tol_max: f64,
    pub decoy: DecoyParams,
}

impl Default for FilterParams {
    fn default() -> Self {
        FilterParams {
            tol_min: TOL_MIN,
            tol_max: TOL_MAX,
            decoy: DecoyParams::default(),
        }
    }
}

fn bad() -> Error {
    Error::new("error.interp.failed").with("detail", "the block interp is not readable")
}

/// The calibration of file `k` from the meta of the block, with the tolerance of `p` (the stored one when `p` keeps the default).
fn cal_of(meta: &Value, k: usize, p: &FilterParams) -> Option<FileCal> {
    let c = meta.get("calibration")?.arr().get(k)?;
    let num = |key: &str| c.get(key).and_then(Value::f64);
    let (median_ppm, mad_ppm) = (num("median_ppm")?, num("mad_ppm")?);
    Some(FileCal {
        median_ppm,
        mad_ppm,
        n: num("n")? as usize,
        tolerance: (2.0 * mad_ppm).clamp(p.tol_min, p.tol_max),
    })
}

/// Whether F5 hides the row `row` (`id`) of the block: the error of its monoisotopic peak against the hypothesis of its change is out of tolerance in
/// the file where it was read. With the default thresholds this is what `build` decided and stored (`hidden`), exactly.
fn mass_error(r: &Value, mz_row: f64, hyps: &[Hyp], meta: &Value, p: &FilterParams) -> bool {
    if *p == FilterParams::default() {
        return r.get("hidden").and_then(Value::str) == Some("F5");
    }
    let formula = r
        .get("change")
        .and_then(|c| c.get("neutral_formula"))
        .and_then(Value::str);
    let ms1 = r.get("ms1");
    let obs = ms1.and_then(|m| m.get("mz_obs")).and_then(Value::f64);
    let file = ms1.and_then(|m| m.get("file")).and_then(Value::f64);
    let (Some(formula), Some(obs), Some(file)) = (formula, obs, file) else {
        return false;
    };
    let theo = hyps
        .iter()
        .find(|h| parse_formula(&h.formula).is_ok_and(|f| fmt(&f) == formula))
        .map_or(mz_row, |h| h.mz);
    let ppm = (obs - theo) / theo * 1e6;
    cal_of(meta, file as usize, p).is_some_and(|cal| out_of_tolerance(ppm, &cal))
}

/// `filtri/1` of the table `size` bytes long under `p`; `hyps` = the registry of hypotheses of the interpretation (`series::registry`, by id).
pub fn refilter<R: ReadAt>(
    r: &mut R,
    size: u64,
    hyps: &[Hyp],
    p: &FilterParams,
) -> Result<Registry> {
    let (t, _) = read_table_from(r, size)?;
    if t.tp.is_empty() {
        return Err(Error::new("error.interp.noTp"));
    }
    let (meta, ids) = store::read_meta(r, size)?.ok_or_else(|| Error::new("error.interp.noTp"))?;
    let mut facts = filtri::row_facts(&t);
    let mut reg = Registry::default();
    for (row, (fact, &status)) in facts.iter_mut().zip(&t.tp.status).enumerate() {
        match first_hidden(fact) {
            Some(i) if status != ST_PARENT => reg.hidden[i].push(t.id(row)),
            None if status == ST_TP || status == ST_UNEXPLAINED => {
                let id = t.id(row);
                if ids.binary_search(&id).is_err() {
                    continue;
                }
                let text = store::read_row(r, size, id)?.ok_or_else(bad)?;
                let doc = parse(text.as_bytes()).map_err(|_| bad())?;
                if mass_error(&doc, t.mz[row], hyps, &meta, p) {
                    fact.mass_error = true;
                    reg.hidden[4].push(id);
                }
            }
            _ => {}
        }
    }
    reg.decoys = table_decoys_with(hyps, &t.mz, &t.tp.status, &facts, &p.decoy);
    Ok(reg)
}
