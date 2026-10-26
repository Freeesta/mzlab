//! What `tp.link@2` does before the interpretation, shared by the native executor (`mzlab-cli`, `flow_link.rs`) and the browser (the export
//! `tp_link_evidence` below): the check of the hypotheses against the parent, the search of the hypotheses in the series, and the proofs on the table
//! (`tp/1`). The interpretation of the rows (`interp/1`) comes after, from the `LinkRun` this gives.

use mzlab_core::budget::MemBudget;
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::{Out, Value};
use mzlab_feat::hypo::HypoParams;
use mzlab_feat::legacy::lr::rows::OFFSET_MAX;
use mzlab_feat::legacy::series;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::slabs::Stored;
use mzlab_feat::store::{read_table_from, Slice};
use mzlab_feat::tp::TpRoles;

use crate::run::{self, RowSources};
use crate::run_tp;

/// The parent of a link: neutral formula, SMILES (empty for a formula), the structure text of the SMILES and the time the parent file gives.
pub struct LinkParent<'a> {
    pub formula: &'a str,
    pub smiles: &'a str,
    pub structure: &'a str,
    pub rt: Option<f64>,
}

/// What the proofs leave for the interpretation: the table with `tp/1`, the registry the search annotated, the parent as the interpretation reads it,
/// the polarity and the time of the parent, and the position of the infusion file in the series.
pub struct LinkRun {
    pub with_tp: Vec<u8>,
    pub registry: String,
    pub parent: String,
    pub polarity: i8,
    pub infusion: Option<usize>,
}

/// The files of the series the link reads: the indexes, the stored peaks (twice: the hypotheses and the proofs each read them once), and a way to open an index.
pub struct Series<'a, I: ReadAt> {
    pub indexes: &'a mut [Index<I>],
    pub peaks: &'a mut dyn Stored,
    pub peaks_again: &'a mut dyn Stored,
    pub open: &'a mut dyn FnMut(usize) -> Result<Index<I>>,
}

/// The polarity of the link: the one the plan sets, else the one of the detection (a detection that took both polarities, 0, or none is read as
/// positive, like a form left at its default).
pub fn polarity_of(given: Option<f64>, recipe: &Recipe) -> i8 {
    match given {
        Some(v) => v.signum() as i8,
        None => recipe.detect.polarity.filter(|&p| p != 0).unwrap_or(1),
    }
}

/// The parent as the interpretation reads it (`series::parent`): the SMILES or the neutral formula, the adduct of the polarity and the time.
pub fn parent_json(p: &LinkParent, polarity: i8, rt: f64) -> String {
    let mut o = Out::default();
    o.raw("{");
    if p.smiles.is_empty() {
        o.raw("\"neutral\":").str(p.formula);
    } else {
        o.raw("\"smiles\":").str(p.smiles);
    }
    o.raw(",\"adduct\":")
        .str(if polarity < 0 { "[M-H]-" } else { "[M+H]+" });
    o.raw(",\"rt\":").num(rt).raw("}");
    o.0
}

/// The m/z of the ion of the parent in the polarity.
fn parent_mz(p: &LinkParent, polarity: i8) -> Result<f64> {
    let parent = series::parent(&parent_json(p, polarity, 0.0))
        .map_err(|e| Error::new("error.interp.failed").with("detail", e))?;
    Ok(parent.mz)
}

/// The retention time of the parent when the file does not say it: the strongest row of the table within 5 ppm of its ion (on a unit-resolution
/// table, within the offset the low-resolution Dig accepts for the parent, `OFFSET_MAX` Da).
fn parent_rt(table: &[u8], mz: f64) -> Result<f64> {
    let (t, meta) = read_table_from(&mut Slice(table), table.len() as u64)?;
    let unit = meta
        .get("recipe")
        .and_then(Value::str)
        .is_some_and(mzlab_feat::recipe::is_unit_text);
    let tol = if unit { OFFSET_MAX } else { mz * 5e-6 };
    let near = (0..t.len()).filter(|&r| (t.mz[r] - mz).abs() <= tol);
    let total = |r: usize| {
        t.area[r * t.n_files..(r + 1) * t.n_files]
            .iter()
            .sum::<f64>()
    };
    let best = near.max_by(|&a, &b| total(a).total_cmp(&total(b)));
    best.map(|r| t.rt[r])
        .ok_or_else(|| Error::new("err.flow.link.parent").with("mz", mz))
}

/// The hypotheses, the search in the series and the proofs on `table` for the parent `p`; `hypotheses` has to be the `ipotesi/1` of `tp.generate` for this
/// parent in this polarity. `given` is the polarity the plan sets, if it does; the recipe takes the polarity that results. `lap(name)` is called after each phase.
#[allow(clippy::too_many_arguments)]
pub fn link_evidence<I: ReadAt>(
    rules: &[u8],
    p: &LinkParent,
    hypotheses: &str,
    table: &[u8],
    roles: &TpRoles,
    recipe: &mut Recipe,
    given: Option<f64>,
    series_of: Series<I>,
    budget: &MemBudget,
    lap: &mut dyn FnMut(&str),
) -> Result<LinkRun> {
    let polarity = polarity_of(given, recipe);
    recipe.detect.polarity = Some(polarity);
    let hp = HypoParams {
        polarity,
        ..HypoParams::default()
    };
    if hypotheses.trim() != run_tp::generate_text(rules, p.formula, p.structure, &hp)? {
        return Err(Error::new("err.flow.link.hypotheses").with("parent", p.formula));
    }
    let mz = parent_mz(p, polarity)?;
    let rt = p.rt.map_or_else(|| parent_rt(table, mz), Ok)?;
    let hypo = run::hypotheses_stored(
        series_of.indexes,
        series_of.peaks,
        rules,
        p.structure,
        p.formula,
        p.rt,
        recipe,
    )?;
    lap("hypotheses");
    let mut with_tp = Vec::new();
    run::evidence_sources_to(
        &mut Slice(table),
        table.len() as u64,
        &hypo,
        &roles.minutes,
        Some(roles),
        Some((mz, rt)),
        recipe,
        budget,
        Some(RowSources {
            stored: series_of.peaks_again,
            open: series_of.open,
        }),
        &mut with_tp,
    )?;
    lap("evidence");
    Ok(LinkRun {
        with_tp,
        registry: hypo.registry.to_json(),
        parent: parent_json(p, polarity, rt),
        polarity,
        infusion: roles.infusion.first().copied(),
    })
}

/// What `tp_link_evidence` gives the page: the table with `tp/1` and what `tp_link_interp` (the .wasm `interp`) needs next.
#[cfg(feature = "proofs")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub struct LinkEvidence {
    inner: LinkRun,
}

#[cfg(feature = "proofs")]
#[wasm_bindgen::prelude::wasm_bindgen]
impl LinkEvidence {
    /// The `.mzfeat` bytes with the block `tp/1`.
    pub fn table(&self) -> Vec<u8> {
        self.inner.with_tp.clone()
    }
    /// The registry `ipotesi/1` the search annotated.
    pub fn registry(&self) -> String {
        self.inner.registry.clone()
    }
    /// The parent as the interpretation reads it.
    pub fn parent(&self) -> String {
        self.inner.parent.clone()
    }
    pub fn polarity(&self) -> i32 {
        self.inner.polarity as i32
    }
    /// Position of the infusion file in the series, -1 without one.
    pub fn infusion(&self) -> i32 {
        self.inner.infusion.map_or(-1, |k| k as i32)
    }
}

/// `tp.link@2` up to the proofs, in the browser: the parent file text and the structure text of its SMILES, the `ipotesi/1` of `tp.generate`, the `.mzfeat` of
/// the series (`table`), the `campioni/2` `samples`, the recipe text of the run, the polarity the plan sets (NaN = not set), the memory budget (MB; 0/NaN = the default),
/// the stored peaks of each file (Uint8Arrays in the order of the indexes) and the `.mzidx` read as `index_read(k, offset, length)` with `index_sizes[k]` bytes.
#[cfg(feature = "proofs")]
#[wasm_bindgen::prelude::wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn tp_link_evidence(
    rules: &[u8],
    parent: &str,
    structure: &str,
    hypotheses: &str,
    table: &[u8],
    samples: &str,
    recipe: &str,
    polarity: f64,
    budget_mb: f64,
    peaks: js_sys::Array,
    index_read: js_sys::Function,
    index_sizes: Vec<f64>,
) -> std::result::Result<LinkEvidence, wasm_bindgen::JsValue> {
    let run = || -> Result<LinkRun> {
        let file = run_tp::read_parent(parent.as_bytes());
        let formula = run_tp::parent_formula(&file, structure)?;
        let p = LinkParent {
            formula: &formula,
            smiles: &file.smiles,
            structure,
            rt: file.rt,
        };
        let roles = TpRoles::from_json(samples)?;
        let mut indexes = (0..index_sizes.len())
            .map(|k| crate::open_one(&index_read, k, index_sizes[k]))
            .collect::<Result<Vec<_>>>()?;
        let mut recipe = match indexes.first_mut() {
            Some(first) => run::recipe_for(recipe, first)?,
            None => run::recipe_from_json(recipe)?,
        };
        let mut hyp_src = crate::JsStored {
            files: peaks.clone(),
        };
        let mut row_src = crate::JsStored { files: peaks };
        let mut open = |k: usize| crate::open_one(&index_read, k, index_sizes[k]);
        link_evidence(
            rules,
            &p,
            hypotheses,
            table,
            &roles,
            &mut recipe,
            polarity.is_finite().then_some(polarity),
            Series {
                indexes: &mut indexes,
                peaks: &mut hyp_src,
                peaks_again: &mut row_src,
                open: &mut open,
            },
            &MemBudget::from_request(budget_mb),
            &mut |_| {},
        )
    };
    run()
        .map(|inner| LinkEvidence { inner })
        .map_err(crate::js_err)
}
