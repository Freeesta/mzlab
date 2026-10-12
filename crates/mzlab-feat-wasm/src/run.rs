//! The steps of the pipeline as plain Rust over any `ReadAt`: the bridge (`lib.rs`) gives them JavaScript readers, the native example gives them files, so the
//! `.mzfeat` of the browser and the one of the desktop come from the same code and the same bytes.

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::{self, Value};
use mzlab_feat::evid::{build_graph, graph_json, Inputs, Kinetics, PointsTable};
use mzlab_feat::hypo::{generate, HypoParams, Inputs as HypoInputs, Registry, Rules};
use mzlab_feat::kinetic::{parent_kinetics, ChainBank, FormBank, Setup, K_PARENT_MIN};
use mzlab_feat::recipe::{Mode, Recipe};
use mzlab_feat::series::{align_and_fill, analyse_file, source_sha256};
use mzlab_feat::store::{read_file, read_table, write_file, write_table_with_evid, Origin};
use mzlab_feat::table::FileFeatures;
use mzlab_feat::target::{run, TargetParams, TargetResult};

/// Recipe from `{"polarity": 1 | -1 | null, "mode": "ion" | "family"}`; anything else keeps the default of the crate.
pub fn recipe_from_json(text: &str) -> Result<Recipe> {
    let v = if text.trim().is_empty() {
        Value::Null
    } else {
        json::parse(text.as_bytes()).map_err(|_| Error::new("error.badRecipe"))?
    };
    let mut r = Recipe::default();
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
    Ok(r)
}

/// Layer L1-L3 of one file: the `.mzfeatf` bytes (the name and SHA-256 of the source go in the meta; no times, so the bytes repeat).
pub fn detect<R: ReadAt>(index: &mut Index<R>, name: &str, r: &Recipe) -> Result<Vec<u8>> {
    let f = analyse_file(index, r)?;
    let o = Origin {
        inputs: vec![(name.to_string(), source_sha256(index))],
        times: Vec::new(),
    };
    Ok(write_file(&f, r, &o))
}

/// The per-file tables and the origin of a list of `.mzfeatf` (name and SHA-256 from their meta).
pub fn read_files<B: AsRef<[u8]>>(
    files: impl IntoIterator<Item = B>,
) -> Result<(Vec<FileFeatures>, Origin)> {
    let (mut out, mut origin) = (Vec::new(), Origin::default());
    for b in files {
        let (f, meta) = read_file(b.as_ref())?;
        origin.inputs.extend(inputs_of(&meta));
        out.push(f);
    }
    Ok((out, origin))
}

/// `(name, SHA-256)` of the inputs named in the meta of a stored file.
fn inputs_of(meta: &Value) -> Vec<(String, String)> {
    let text = |i: &Value, k: &str| i.get(k).and_then(Value::str).unwrap_or("").to_string();
    meta.get("inputs")
        .map(|a| {
            a.arr()
                .iter()
                .map(|i| (text(i, "name"), text(i, "sha256")))
                .collect()
        })
        .unwrap_or_default()
}

/// Alignment and gap filling of a series: the `.mzfeat` bytes with an empty `evid` section. `open(k)` gives the index of file `k`.
pub fn align<R: ReadAt>(
    feats: &[FileFeatures],
    origin: &Origin,
    r: &Recipe,
    open: impl FnMut(usize) -> Result<Index<R>>,
) -> Result<Vec<u8>> {
    let (table, _) = align_and_fill(feats, r, open)?;
    Ok(write_table_with_evid(&table, r, origin, &[]))
}

/// What the targeted pass leaves for the proofs: the registry of the hypotheses with their outcomes and the rows found.
pub struct HypoRun {
    pub registry: Registry,
    pub result: TargetResult,
}

/// Hypotheses of the parent (rules as the bytes of `regole_tp.json`) searched in every file. `parent_rt` narrows the windows of RT.
pub fn hypotheses<R: ReadAt>(
    indexes: &mut [Index<R>],
    feats: &[&FileFeatures],
    rules: &[u8],
    parent: &str,
    parent_rt: Option<f64>,
    r: &Recipe,
) -> Result<HypoRun> {
    let polarity = r.detect.polarity.unwrap_or(1);
    let inp = HypoInputs {
        parent,
        suspects: None,
        families: &[],
        imported: &[],
        fragments: &[],
    };
    let hp = HypoParams {
        polarity,
        ..HypoParams::default()
    };
    let mut registry = generate(&Rules::parse(rules)?, &inp, &hp)?;
    let tp = TargetParams {
        polarity,
        parent_rt,
        ..TargetParams::default()
    };
    let result = run(indexes, feats, &mut registry, &tp)?;
    Ok(HypoRun { registry, result })
}

/// Graph of the proofs on the table of a series (`.mzfeat` bytes): `minutes[k]` is the treatment time of file `k` (NaN = a control).
/// Returns the same table with the `evid/1` document in its `evid` section. The family arcs are not stored in the files, so the graph has none.
pub fn evidence(
    table_bytes: &[u8],
    hypo: &HypoRun,
    minutes: &[f64],
    parent: Option<(f64, f64)>,
    r: &Recipe,
) -> Result<Vec<u8>> {
    let (aligned, meta) = read_table(table_bytes)?;
    let origin = Origin {
        inputs: inputs_of(&meta),
        times: Vec::new(),
    };
    let points = PointsTable::bundled()?;
    let treated: Vec<usize> = (0..minutes.len())
        .filter(|&k| minutes[k].is_finite())
        .collect();
    let controls: Vec<usize> = (0..minutes.len())
        .filter(|&k| !minutes[k].is_finite())
        .collect();
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
    let json = graph_json(&build_graph(&inp), &points);
    Ok(write_table_with_evid(&aligned, r, &origin, json.as_bytes()))
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
