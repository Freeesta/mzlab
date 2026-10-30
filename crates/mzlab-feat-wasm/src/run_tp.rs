//! What `tp.generate` does, shared by the native executor (`mzlab-cli`, `flow_generate.rs`) and the browser (the export `tp_generate` below): the parent file, the
//! parameters of the hypotheses and the registry `ipotesi/1`. The structure graph of a SMILES comes in as text (the CLI makes it with `chem`, the page with the
//! `grafo` .wasm), so this module needs no chemistry. The same goes for `tp.link@2` up to the proofs (`link_evidence`, export `tp_link_evidence`): the interpretation of
//! the rows comes after, from the `LinkRun` this gives (`interp.rs`).

use mzlab_core::budget::MemBudget;
use mzlab_core::elements;
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::{parse, Out, Value};
use mzlab_feat::hypo::{generate, HypoParams, Inputs, Rules, Structure};
use mzlab_feat::legacy::lr::rows::OFFSET_MAX;
use mzlab_feat::legacy::series;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::slabs::Stored;
use mzlab_feat::store::{read_table_from, Slice};
use mzlab_feat::tp::TpRoles;

use crate::run::{self, RowSources};

// `lib.rs` is at its line limit, so the Dig inputs of `tp.dig_lr` (shared by the CLI and the browser) hang here.
#[path = "run_dig.rs"]
pub mod dig;

/// The parent file as written: a JSON `{"formula"|"neutral", "smiles", "rt"}`, or one line with a formula (written as `elements::fmt` writes it) or a SMILES.
/// `formula` is empty when the file gives only a SMILES.
pub struct ParentFile {
    pub formula: String,
    pub smiles: String,
    pub rt: Option<f64>,
    /// The adduct of the parent ion when the file says a valid one, else `[M+H]+` (its sign is the polarity of the products).
    pub adduct: String,
}

pub fn read_parent(bytes: &[u8]) -> ParentFile {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim();
    let json = parse(text.as_bytes()).ok().filter(|v| {
        ["rt", "formula", "neutral", "smiles"]
            .iter()
            .any(|k| v.get(k).is_some())
    });
    if let Some(v) = &json {
        let s = |k: &str| {
            v.get(k)
                .and_then(Value::str)
                .unwrap_or("")
                .trim()
                .to_string()
        };
        let formula = [s("formula"), s("neutral")]
            .into_iter()
            .find(|x| !x.is_empty())
            .unwrap_or_default();
        let rt = v.get("rt").and_then(Value::f64).filter(|t| t.is_finite());
        let adduct = Some(s("adduct"))
            .filter(|a| elements::ion_mz(0.0, a).is_some())
            .unwrap_or_else(|| "[M+H]+".into());
        return ParentFile {
            formula,
            smiles: s("smiles"),
            rt,
            adduct,
        };
    }
    let line = text.lines().next().unwrap_or("").trim().to_string();
    // a formula is written as the engine writes it ("C13H24N4O3S"); "CCO" and "CC(C)O" are SMILES
    let is_formula = elements::parse_formula(&line).is_ok_and(|f| elements::fmt(&f) == line);
    if is_formula {
        ParentFile {
            formula: line,
            smiles: String::new(),
            rt: None,
            adduct: "[M+H]+".into(),
        }
    } else {
        ParentFile {
            formula: String::new(),
            smiles: line,
            rt: None,
            adduct: "[M+H]+".into(),
        }
    }
}

/// The neutral formula of the parent over its structure text (empty without a SMILES): the one of the file, else the one of the structure.
pub fn parent_formula(p: &ParentFile, structure: &str) -> Result<String> {
    let own = if structure.is_empty() {
        String::new()
    } else {
        Structure::parse(structure)?.formula.unwrap_or_default()
    };
    let formula = if p.formula.is_empty() {
        own
    } else {
        p.formula.clone()
    };
    if formula.is_empty() {
        return Err(Error::new("error.hypo.noParent"));
    }
    Ok(formula)
}

/// The parameters of `tp.generate` over `HypoParams::default()`; `get` gives the value a plan or a job set for a name.
pub fn hypo_params(get: &dyn Fn(&str) -> Option<Value>) -> HypoParams {
    let d = HypoParams::default();
    let num = |n: &str| get(n).and_then(|v| v.f64());
    let flag = |n: &str, dflt: bool| get(n).and_then(|v| v.bool()).unwrap_or(dflt);
    HypoParams {
        polarity: num("polarity").map_or(d.polarity, |v| if v < 0.0 { -1 } else { 1 }),
        max_steps: num("max_steps").map_or(d.max_steps, |v| v as u8),
        min_mz: num("min_mz").unwrap_or(d.min_mz),
        max_suspects: num("max_suspects").map_or(d.max_suspects, |v| v as usize),
        other_polarity: flag("other_polarity", d.other_polarity),
        nominal: flag("nominal", d.nominal),
    }
}

/// `ipotesi/1` text of a parent under the rules (`regole_tp_2.json` bytes): what the engine could find, never a verdict.
pub fn generate_text(
    rules: &[u8],
    formula: &str,
    structure: &str,
    hp: &HypoParams,
) -> Result<String> {
    let st = Structure::parse(structure)?;
    st.check_parent(formula)?;
    let inp = Inputs {
        parent: formula,
        suspects: None,
        families: &[],
        imported: &[],
        fragments: &st.fragments,
        groups: st.groups.as_deref(),
    };
    let rules = Rules::parse_profile(rules, st.profile.as_deref())?;
    Ok(generate(&rules, &inp, hp)?.to_json())
}

/// A SMILES or a formula in the parent file, as `{"formula","smiles","rt"}`: the page asks the `grafo` .wasm for the structure of the SMILES before `tp_generate`.
#[cfg(feature = "proofs")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn tp_parent(parent: &str) -> String {
    let bytes = parent.as_bytes();
    let p = read_parent(bytes);
    let mut o = mzlab_core::json::Out::default();
    o.raw("{\"formula\":")
        .str(&p.formula)
        .raw(",\"smiles\":")
        .str(&p.smiles);
    o.raw(",\"rt\":");
    match p.rt {
        Some(t) => o.num(t),
        None => o.raw("null"),
    };
    o.raw("}");
    o.0
}

/// `tp.generate` in the browser: the registry of the hypotheses of the parent file `parent` (text) with the structure text of its SMILES (empty for a formula)
/// under the parameters `params` (JSON object of the step, `{}` for the defaults) and the rules.
#[cfg(feature = "proofs")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn tp_generate(
    rules: &[u8],
    parent: &str,
    structure: &str,
    params: &str,
) -> std::result::Result<String, wasm_bindgen::JsValue> {
    let run = || -> Result<String> {
        let p = read_parent(parent.as_bytes());
        let formula = parent_formula(&p, structure)?;
        let set = parse(params.as_bytes())?;
        generate_text(
            rules,
            &formula,
            structure,
            &hypo_params(&|n| set.get(n).cloned()),
        )
    };
    run().map_err(crate::js_err)
}

/// The parameters of `tp.generate` the files decide when the plan does not write them (`mzlab_feat::nature`, the rule of `flow run`): the names the plan writes
/// and the `.mzidx` of the series read as `index_read(k, offset, length)`; `[{"name","value","proof"}]`.
#[cfg(feature = "proofs")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn series_estimates(
    written: Vec<String>,
    index_read: js_sys::Function,
    index_sizes: Vec<f64>,
) -> std::result::Result<String, wasm_bindgen::JsValue> {
    let run = || -> Result<String> {
        let indexes = (0..index_sizes.len())
            .map(|k| crate::open_one(&index_read, k, index_sizes[k]))
            .collect::<Result<Vec<_>>>()?;
        let nature = mzlab_feat::nature::of_indexes(&indexes);
        let list = mzlab_feat::nature::estimates(&|n| written.iter().any(|w| w == n), &nature);
        Ok(mzlab_feat::nature::estimates_json(&list))
    };
    run().map_err(crate::js_err)
}

/// The facets of the `.mzidx` of a series read as `index_read(k, offset, length)` (`mzlab_feat::nature::facets`, the rule `flow run` puts on the scan
/// inputs before the check): `{"resolution":[...],"polarity":[...]}`.
#[cfg(feature = "proofs")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn series_facets(
    index_read: js_sys::Function,
    index_sizes: Vec<f64>,
) -> std::result::Result<String, wasm_bindgen::JsValue> {
    let run = || -> Result<String> {
        let mut files = Vec::new();
        for (k, size) in index_sizes.iter().enumerate() {
            let ix = crate::open_one(&index_read, k, *size)?;
            files.push(mzlab_feat::nature::index_facts(&ix));
        }
        Ok(mzlab_feat::nature::facets_json(
            &mzlab_feat::nature::facets(files),
        ))
    };
    run().map_err(crate::js_err)
}

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
/// parent in this polarity. `given` is the polarity the plan sets, if it does, else the files' when the detection took both; the recipe takes the polarity that results. `lap(name)` is called after each phase.
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
    // a plan that does not write the polarity over a detection that took both: the files decide (`mzlab_feat::nature`, as `flow run`)
    let open = mzlab_feat::nature::open_polarity(
        recipe.detect.polarity,
        &mzlab_feat::nature::of_indexes(series_of.indexes),
    );
    let polarity = polarity_of(given.or(open.map(f64::from)), recipe);
    recipe.detect.polarity = Some(polarity);
    // the engine searches the hypotheses of the recipe (merged by nominal m/z at unit resolution): the registry of `tp.generate` has to be that one
    // (`nominal` taken from the files) or, as before the step read the files, the exact-mass one
    let unit = recipe.align.ion.ppm < 0.0;
    let same = |nominal: bool| -> Result<bool> {
        let hp = HypoParams {
            polarity,
            nominal,
            ..HypoParams::default()
        };
        Ok(hypotheses.trim() == generate_text(rules, p.formula, p.structure, &hp)?)
    };
    if !same(unit)? && !(unit && same(false)?) {
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
        let file = read_parent(parent.as_bytes());
        let formula = parent_formula(&file, structure)?;
        let p = LinkParent {
            formula: &formula,
            smiles: &file.smiles,
            structure,
            rt: file.rt,
        };
        let roles = TpRoles::for_table(samples, table)?;
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
