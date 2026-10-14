//! The steps of the pipeline as plain Rust over any `ReadAt`: the bridge (`lib.rs`) gives them JavaScript readers, the native example gives them files, so the
//! `.mzfeat` of the browser and the one of the desktop come from the same code and the same bytes.

use mzlab_core::budget::{MemBudget, WriteAt};
use mzlab_core::elements;
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::{self, Value};
use mzlab_feat::evid::rowproofs::row_signs;
use mzlab_feat::evid::{build_graph, graph_json, GraphStream, Inputs, Kinetics, PointsTable};
use mzlab_feat::facts::{file_facts, FileFacts};
use mzlab_feat::hypo::{
    generate, HypoParams, Hypothesis, Inputs as HypoInputs, Registry, Rules, Structure,
};
use mzlab_feat::kinetic::{parent_kinetics, ChainBank, FormBank, Setup, K_PARENT_MIN};
use mzlab_feat::recipe::{Mode, Recipe};
use mzlab_feat::series::{align_and_fill, analyse_file, source_sha256};
use mzlab_feat::slabs::{align_and_fill_stored, Stored};
use mzlab_feat::store::{
    origin_from_meta, read_file, read_meta_mz, read_table_from, write_file_to, write_table_to,
    write_table_tp, write_table_with_evid, Origin, Slice,
};
use mzlab_feat::table::{FileFeatures, RowIdent};
use mzlab_feat::target::join::JoinParams;
use mzlab_feat::target::{run, TargetParams, TargetResult};
use mzlab_feat::tp::{build as build_tp, build_by_role, TpBlock, TpInputs, TpParams, TpRoles};

/// Recipe from `{"polarity": 1 | -1 | null, "mode": "ion" | "family"}`; anything else keeps the default of the crate.
pub fn recipe_from_json(text: &str) -> Result<Recipe> {
    recipe_on(text, Recipe::default())
}

/// `recipe_from_json` on the recipe that fits the index: the default one for high resolution, the unit-resolution one (m/z axis in Da) otherwise.
pub fn recipe_for<R: ReadAt>(text: &str, index: &mut Index<R>) -> Result<Recipe> {
    recipe_on(text, Recipe::for_index(index)?)
}

fn recipe_on(text: &str, base: Recipe) -> Result<Recipe> {
    let v = if text.trim().is_empty() {
        Value::Null
    } else {
        json::parse(text.as_bytes()).map_err(|_| Error::new("error.badRecipe"))?
    };
    let mut r = base;
    r.detect.polarity = match v.get("polarity").and_then(Value::f64) {
        Some(p) if p < 0.0 => Some(-1),
        Some(_) => Some(1),
        None => None,
    };
    match v.get("mode").and_then(Value::str) {
        Some("ion") => r.mode = Mode::Ion,
        Some("family") | None => {}
        Some(_) => return Err(Error::new("error.badRecipe")),
    }
    apply_params(&mut r, &v);
    Ok(r)
}

/// The numbers of the panel "Pipeline" (`floor`, `min_height`, `min_points`, `ppm`, `rt_tol`, `fill`); a missing key keeps the default.
fn apply_params(r: &mut Recipe, v: &Value) {
    let num = |k: &str| {
        v.get(k)
            .and_then(Value::f64)
            .filter(|x| x.is_finite() && *x > 0.0)
    };
    if let Some(x) = num("floor") {
        r.detect.floor = x;
        r.fill.floor = x;
    }
    if let Some(x) = num("min_height") {
        r.detect.min_height = x;
    }
    if let Some(x) = num("min_points") {
        r.detect.min_points = x as u32;
        r.fill.min_points = x as usize;
    }
    if let Some(x) = num("ppm").filter(|_| r.align.ion.ppm > 0.0) {
        r.align.ion.ppm = x;
        r.fill.ppm = x;
    }
    if let Some(x) = num("rt_tol") {
        r.align.ion.rt_gap = x;
        r.fill.rt_tol = x;
    }
    if let Some(b) = v.get("fill").and_then(Value::bool) {
        r.fill_gaps = b;
    }
}

/// Layer L1-L3 of one file written to `sink`: the `.mzfeatf` (the name and SHA-256 of the source go in the meta; no times, so the bytes repeat).
pub fn detect_to<R: ReadAt, W: WriteAt>(
    index: &mut Index<R>,
    name: &str,
    r: &Recipe,
    sink: &mut W,
) -> Result<()> {
    let f = analyse_file(index, r)?;
    let o = Origin {
        inputs: vec![(name.to_string(), source_sha256(index))],
        ..Origin::default()
    };
    write_file_to(sink, &f, r, &o)
}

/// Layer L1-L3 of one file: the `.mzfeatf` bytes.
pub fn detect<R: ReadAt>(index: &mut Index<R>, name: &str, r: &Recipe) -> Result<Vec<u8>> {
    let mut v = Vec::new();
    detect_to(index, name, r, &mut v)?;
    Ok(v)
}

/// The per-file tables and the origin of a list of `.mzfeatf` (name and SHA-256 from their meta).
pub fn read_files<B: AsRef<[u8]>>(
    files: impl IntoIterator<Item = B>,
) -> Result<(Vec<FileFeatures>, Origin)> {
    let (mut out, mut origin) = (Vec::new(), Origin::default());
    for b in files {
        let (f, meta) = read_file(b.as_ref())?;
        origin.inputs.extend(origin_from_meta(&meta).inputs);
        out.push(f);
    }
    Ok((out, origin))
}

/// Alignment and gap filling of a series: the `.mzfeat` bytes with an empty `evid` section. `open(k)` gives the index of file `k`.
pub fn align<R: ReadAt>(
    feats: &[FileFeatures],
    origin: &Origin,
    r: &Recipe,
    mut open: impl FnMut(usize) -> Result<Index<R>>,
) -> Result<Vec<u8>> {
    let origin = Origin {
        facts: facts_of(feats.len(), &mut open)?,
        ..origin.clone()
    };
    let (table, _) = align_and_fill(feats, r, open)?;
    Ok(write_table_with_evid(&table, r, &origin, &[]))
}

/// How each of the `n` inputs was acquired, read from its index.
fn facts_of<R: ReadAt>(
    n: usize,
    open: &mut impl FnMut(usize) -> Result<Index<R>>,
) -> Result<Vec<FileFacts>> {
    (0..n).map(|k| Ok(file_facts(&open(k)?))).collect()
}

/// The origin (name and SHA-256 of every input) of a stored series, from the meta of its files.
pub fn origin_of(src: &mut dyn Stored) -> Result<Origin> {
    let mut origin = Origin::default();
    for k in 0..src.count() {
        let (meta, _) = read_meta_mz(&src.bytes(k)?)?;
        origin.inputs.extend(origin_from_meta(&meta).inputs);
    }
    Ok(origin)
}

/// Alignment and gap filling of a stored series under `budget`, written to `sink` as the `.mzfeat` with an empty `evid` section. The same bytes as
/// `align` whatever the budget (in memory, in slabs of m/z) or an orderly `error.memoryBudget`.
pub fn align_to<R: ReadAt, W: WriteAt>(
    src: &mut dyn Stored,
    budget: &MemBudget,
    r: &Recipe,
    mut open: impl FnMut(usize) -> Result<Index<R>>,
    sink: &mut W,
) -> Result<()> {
    let mut origin = origin_of(src)?;
    origin.facts = facts_of(src.count(), &mut open)?;
    let (table, _) = align_and_fill_stored(src, budget, r, open)?;
    write_table_to(sink, &table, r, &origin, &mut (&[][..]))
}

/// What the targeted pass leaves for the proofs: the registry of the hypotheses with their outcomes and the rows found.
pub struct HypoRun {
    pub registry: Registry,
    pub result: TargetResult,
    /// The rules as read (bytes of the file) and the profile chosen: the proofs look for their differences of mass among the rows (`tp::rete`).
    pub rules_src: Vec<u8>,
    pub profile: Option<String>,
}

impl HypoRun {
    /// The rules of the run, parsed again from their bytes; None when the run has none.
    fn rules(&self) -> Result<Option<Rules>> {
        if self.rules_src.is_empty() {
            return Ok(None);
        }
        Rules::parse_profile(&self.rules_src, self.profile.as_deref()).map(Some)
    }
}

/// Offset of the calibration of a unit-resolution instrument, measured on the parent (0 when the parent is not seen or the formula is not valid).
fn parent_offset(feats: &[&FileFeatures], parent: &str, polarity: i8, rt: Option<f64>) -> f64 {
    let adduct = if polarity < 0 { "[M-H]-" } else { "[M+H]+" };
    elements::parse_formula(parent)
        .ok()
        .and_then(|f| elements::ion_mz(elements::mass(&f), adduct))
        .and_then(|mz| mzlab_feat::target::offset::estimate(feats, mz, rt))
        .unwrap_or(0.0)
}

/// The registry of the hypotheses of the parent (rules as the bytes of `regole_tp_2.json`; `structure` is the `grafo/1` JSON of its SMILES, or empty) and
/// the parameters of the targeted pass.
fn hypotheses_setup(
    rules: &[u8],
    structure: &str,
    parent: &str,
    parent_rt: Option<f64>,
    polarity: i8,
    unit: bool,
) -> Result<(Registry, TargetParams, Option<String>)> {
    let st = Structure::parse(structure)?;
    st.check_parent(parent)?;
    let inp = HypoInputs {
        parent,
        suspects: None,
        families: &[],
        imported: &[],
        fragments: &st.fragments,
        groups: st.groups.as_deref(),
    };
    let hp = HypoParams {
        polarity,
        nominal: unit,
        ..HypoParams::default()
    };
    let registry = generate(
        &Rules::parse_profile(rules, st.profile.as_deref())?,
        &inp,
        &hp,
    )?;
    let tp = TargetParams {
        polarity,
        parent_rt,
        ..if unit {
            TargetParams::lr(mzlab_feat::trace::FLOOR)
        } else {
            TargetParams::default()
        }
    };
    Ok((registry, tp, st.profile))
}

/// Hypotheses of the parent searched in every file. `parent_rt` narrows the windows of RT.
pub fn hypotheses<R: ReadAt>(
    indexes: &mut [Index<R>],
    feats: &[&FileFeatures],
    rules: &[u8],
    structure: &str,
    parent: &str,
    parent_rt: Option<f64>,
    r: &Recipe,
) -> Result<HypoRun> {
    let (mut registry, mut tp, profile) = hypotheses_setup(
        rules,
        structure,
        parent,
        parent_rt,
        r.detect.polarity.unwrap_or(1),
        r.align.ion.ppm < 0.0,
    )?;
    if r.align.ion.ppm < 0.0 {
        tp.mz_offset = parent_offset(feats, parent, tp.polarity, parent_rt);
    }
    let result = run(indexes, feats, &mut registry, &tp)?;
    Ok(HypoRun {
        registry,
        result,
        rules_src: rules.to_vec(),
        profile,
    })
}

/// `hypotheses` with the detection tables read from the stored files one at a time.
pub fn hypotheses_stored<R: ReadAt>(
    indexes: &mut [Index<R>],
    src: &mut dyn Stored,
    rules: &[u8],
    structure: &str,
    parent: &str,
    parent_rt: Option<f64>,
    r: &Recipe,
) -> Result<HypoRun> {
    let polarity = r.detect.polarity.unwrap_or(1);
    let (mut registry, mut tp, profile) = hypotheses_setup(
        rules,
        structure,
        parent,
        parent_rt,
        polarity,
        r.align.ion.ppm < 0.0,
    )?;
    let n = indexes.len().min(src.count());
    let mut load = |k: usize| Ok(read_file(&src.bytes(k)?)?.0);
    if r.align.ion.ppm < 0.0 {
        let feats = (0..n).map(&mut load).collect::<Result<Vec<_>>>()?;
        let refs: Vec<&FileFeatures> = feats.iter().collect();
        tp.mz_offset = parent_offset(&refs, parent, polarity, parent_rt);
    }
    let result = mzlab_feat::target::run_loading(indexes, n, &mut load, &mut registry, &tp)?;
    Ok(HypoRun {
        registry,
        result,
        rules_src: rules.to_vec(),
        profile,
    })
}

/// Bytes of the memory that an identity of the graph takes while the graph is built whole (its proofs, members and arcs), for the estimate.
const IDENTITY_BYTES: usize = 2400;

/// Graph of the proofs on the table of a series (`.mzfeat` bytes): `minutes[k]` is the treatment time of file `k` (NaN = a control).
/// Returns the same table with the `evid/1` document in its `evid` section. The family arcs are not stored in the files, so the graph has none.
pub fn evidence(
    table_bytes: &[u8],
    hypo: &HypoRun,
    minutes: &[f64],
    parent: Option<(f64, f64)>,
    r: &Recipe,
) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let budget = MemBudget::mb(mzlab_core::budget::NATIVE_BUDGET_MB as f64);
    let mut at = Slice(table_bytes);
    let size = table_bytes.len() as u64;
    evidence_to(&mut at, size, hypo, minutes, parent, r, &budget, &mut out)?;
    Ok(out)
}

/// Computes the identity columns of the table and gives the table of points: the unit-resolution one (windows in Da) when the recipe in the meta says so.
fn points_and_identity(
    aligned: &mut mzlab_feat::table::Aligned,
    hypo: &HypoRun,
    meta: &Value,
) -> Result<PointsTable> {
    let unit = meta
        .get("recipe")
        .and_then(Value::str)
        .is_some_and(mzlab_feat::recipe::is_unit_text);
    let win = if unit {
        mzlab_feat::axis::da(mzlab_feat::axis::LR_TOL)
    } else {
        mzlab_feat::evid::HYPOTHESIS_PPM
    };
    aligned.ident = RowIdent::compute_in(aligned, &hypo.registry.hyps, hypo.registry.polarity, win);
    let points = PointsTable::bundled()?;
    Ok(if unit { points.with_unit() } else { points })
}

/// `evidence` over a table read by offsets and an output written to `sink`. The graph is built whole and written when its estimate fits in half of the
/// budget; else its identities are made twice, once for the order and the length, once for the bytes (`GraphStream`), and only one is held. Same bytes.
#[allow(clippy::too_many_arguments)]
pub fn evidence_to<R: ReadAt, W: WriteAt>(
    table: &mut R,
    size: u64,
    hypo: &HypoRun,
    minutes: &[f64],
    parent: Option<(f64, f64)>,
    r: &Recipe,
    budget: &MemBudget,
    sink: &mut W,
) -> Result<()> {
    evidence_roles_to(table, size, hypo, minutes, None, parent, r, budget, sink)
}

/// `evidence_to` with the roles of the files (`campioni/2`): gate 1 of the view of the products reads them (`tp::build_by_role`).
#[allow(clippy::too_many_arguments)]
pub fn evidence_roles_to<R: ReadAt, W: WriteAt>(
    table: &mut R,
    size: u64,
    hypo: &HypoRun,
    minutes: &[f64],
    roles: Option<&TpRoles>,
    parent: Option<(f64, f64)>,
    r: &Recipe,
    budget: &MemBudget,
    sink: &mut W,
) -> Result<()> {
    let (mut aligned, meta) = read_table_from(table, size)?;
    hypo.result
        .add_missing_to(&mut aligned, &JoinParams::default());
    aligned.sort_rows();
    let origin = origin_from_meta(&meta);
    let points = points_and_identity(&mut aligned, hypo, &meta)?;
    let minutes = roles.map_or(minutes, |ro| &ro.minutes);
    let (treated, controls) = TpRoles::kinetic_files(minutes, roles);
    let times: Vec<f64> = treated.iter().map(|&k| minutes[k]).collect();
    let (forms, chain) = (FormBank::new(&times), ChainBank::new(&times, K_PARENT_MIN));
    let y = parent
        .and_then(|(mz, rt)| nearest_row(&aligned, mz, rt))
        .map(|row| {
            treated
                .iter()
                .map(|&k| aligned.area[row * aligned.n_files + k])
                .collect()
        })
        .unwrap_or_else(|| vec![1.0; times.len()]);
    let setup = Setup {
        times: &times,
        forms: &forms,
        chain: &chain,
        parent: parent_kinetics(&forms, &times, &y, None),
        pseudo: mzlab_feat::trace::FLOOR * 3.0,
    };
    let inp = Inputs {
        table: &points,
        aligned: &aligned,
        facts: &[],
        arcs: &[],
        registry: &hypo.registry,
        libs: &[],
        external: &[],
        kinetics: Some(Kinetics {
            setup: &setup,
            treated: &treated,
            controls: &controls,
        }),
        parent,
    };
    let rules = hypo.rules()?;
    let tp = tp_block(&inp, minutes, roles, &hypo.registry.hyps, parent, rules.as_ref());
    let rows = aligned.len();
    let held = aligned.heap_bytes() + tp.heap_bytes();
    if budget.in_memory(held + rows * IDENTITY_BYTES) {
        let json = graph_json(&build_graph(&inp), &points);
        return write_table_tp(sink, &aligned, &tp, r, &origin, &mut json.as_bytes());
    }
    let need = held + rows * GraphStream::bytes_per_row();
    let _g = budget.reserve("proofs", need)?;
    let mut graph = GraphStream::new(&inp);
    write_table_tp(sink, &aligned, &tp, r, &origin, &mut graph)
}

/// The view of the products of the table (`tp/1`): empty without a parent in the table or without controls to read the gates against.
fn tp_block(
    inp: &Inputs,
    minutes: &[f64],
    roles: Option<&TpRoles>,
    hyps: &[Hypothesis],
    parent: Option<(f64, f64)>,
    rules: Option<&Rules>,
) -> TpBlock {
    let Some(parent) = parent else {
        return TpBlock::default();
    };
    let signs = row_signs(inp);
    let tp = TpInputs {
        aligned: inp.aligned,
        minutes,
        hyps,
        signs: &signs,
        parent,
        fragments: None,
        params: &TpParams::default(),
        rules,
    };
    match roles {
        Some(ro) => build_by_role(&tp, ro),
        None => build_tp(&tp),
    }
    .ok()
    .flatten()
    .unwrap_or_default()
}

/// The row within 5 ppm and 0.3 min of the parent with the largest area (the parent decay is read from it).
fn nearest_row(t: &mzlab_feat::table::Aligned, mz: f64, rt: f64) -> Option<usize> {
    let area = |r: usize| {
        t.area[r * t.n_files..(r + 1) * t.n_files]
            .iter()
            .copied()
            .filter(|a| a.is_finite())
            .fold(0.0, f64::max)
    };
    (0..t.len())
        .filter(|&r| (t.mz[r] - mz).abs() <= mz * 5e-6 && (t.rt[r] - rt).abs() <= 0.3)
        .max_by(|&a, &b| area(a).total_cmp(&area(b)))
}

/// Preview of one file (see `mzlab_feat::params`): the floor of the recipe is used if it was given, else the one estimated from the intensities.
pub fn preview<R: ReadAt>(index: &mut Index<R>, r: &Recipe, floor_given: bool) -> Result<String> {
    let mut p = r.detect.clone();
    if !floor_given {
        if let Some(si) = index.section_of(1, p.polarity) {
            let h = mzlab_feat::params::intensity_hist(index, si)?;
            if let Some(n) = mzlab_feat::params::noise_from_hist(&h) {
                p.floor = n.floor;
            }
        }
    }
    mzlab_feat::params::preview(index, &p)
}

/// True if the recipe text names a `floor` (else the preview estimates it).
pub fn floor_given(text: &str) -> bool {
    json::parse(text.as_bytes()).is_ok_and(|v| v.get("floor").and_then(Value::f64).is_some())
}
