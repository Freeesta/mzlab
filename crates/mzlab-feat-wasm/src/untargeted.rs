//! The pipeline after mzmine 3.9.0 (`mzlab_feat::untargeted::pipeline`) as steps over any `ReadAt`/`WriteAt`: the bridge (`lib.rs`) gives them the readers and
//! sinks of the worker, the native example (`examples/untargeted.rs`) gives them files, so the browser and the desktop write the same bytes.
//!
//! `detect_to` writes the list of features of one file as the usual `.mzfeatf` (m/z, RT, limits, area in counts x seconds, height; the columns that mzmine does
//! not give stay neutral); `align_to` reads those files, joins them, fills the gaps from the indexes and writes the usual `.mzfeat` (one row per ion, state
//! detected or filled). Nothing here is a new format.

use mzlab_core::budget::{MemBudget, WriteAt};
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_feat::recipe::{Mode, Recipe};
use mzlab_feat::series::source_sha256;
use mzlab_feat::slabs::Stored;
use mzlab_feat::store::{read_file, write_file_to, write_table_to, Origin};
use mzlab_feat::untargeted::pipeline::{
    from_file_features, run_file, run_series, to_file_features, UntParams,
};
use mzlab_feat::untargeted::Feature;

/// Bytes a feature costs while the lists of the series are held (the list, the rows and the cells).
const FEATURE_BYTES: usize = 160;

/// The recipe that goes in the meta of the files: per ion, the tolerances of the join (the rest of the recipe is not used by this engine).
fn recipe_of(p: &UntParams) -> Recipe {
    let mut r = Recipe {
        mode: Mode::Ion,
        ..Recipe::default()
    };
    r.detect.polarity = p.polarity;
    r.align.ion.ppm = p.join.tol_ppm;
    r.align.ion.rt_gap = p.join.rt_tol;
    r.fill.ppm = p.gap.tol_ppm;
    r.fill.rt_tol = p.gap.rt_tol;
    r
}

/// Median interval between MS1 scans, seconds (the `dt` that the rows of the table use to turn heights into areas).
fn scan_interval<R: ReadAt>(ix: &mut Index<R>, p: &UntParams) -> Result<f64> {
    let Some(si) = ix.section_of(1, p.polarity) else {
        return Ok(1.0);
    };
    let rt = &ix.arrays(si)?.rt;
    let mut d: Vec<f64> = rt.windows(2).map(|w| w[1] - w[0]).collect();
    d.sort_by(|a, b| a.total_cmp(b));
    Ok(d.get(d.len() / 2).map_or(1.0, |m| m * 60.0))
}

/// ADAP + resolver on one file written to `sink` as a `.mzfeatf` (the name and SHA-256 of the source go in the meta; no times, so the bytes repeat).
pub fn detect_to<R: ReadAt, W: WriteAt>(
    ix: &mut Index<R>,
    name: &str,
    p: &UntParams,
    sink: &mut W,
) -> Result<()> {
    let list = run_file(ix, p)?;
    let mut f = to_file_features(&list);
    f.dt = scan_interval(ix, p)?;
    let o = Origin {
        inputs: vec![(name.to_string(), source_sha256(ix))],
        ..Origin::default()
    };
    write_file_to(sink, &f, &recipe_of(p), &o)
}

/// Join + gap filling of the `.mzfeatf` of a series written to `sink` as the `.mzfeat` (empty `evid`). `open(k)` gives the index of file `k`; the lists of
/// the series must fit in `budget` or the step is refused in order (`error.memoryBudget`).
pub fn align_to<R: ReadAt, W: WriteAt>(
    src: &mut dyn Stored,
    budget: &MemBudget,
    p: &UntParams,
    mut open: impl FnMut(usize) -> Result<Index<R>>,
    sink: &mut W,
) -> Result<()> {
    let n = src.count();
    let (mut lists, mut origin): (Vec<Vec<Feature>>, Origin) = (Vec::new(), Origin::default());
    let mut held = 0usize;
    for k in 0..n {
        let (f, meta) = read_file(&src.bytes(k)?)?;
        held += f.len() * FEATURE_BYTES;
        if !budget.in_memory(held) {
            return Err(budget.refuse("align", held * 2));
        }
        origin
            .inputs
            .extend(mzlab_feat::store::origin_from_meta(&meta).inputs);
        lists.push(from_file_features(&f));
    }
    let mut ixs = (0..n).map(&mut open).collect::<Result<Vec<_>>>()?;
    origin.facts = ixs.iter().map(mzlab_feat::facts::file_facts).collect();
    let names: Vec<String> = origin.inputs.iter().map(|i| i.0.clone()).collect();
    let table = run_series(&lists, &mut ixs, &names, p)?;
    write_table_to(sink, &table, &recipe_of(p), &origin, &mut (&[][..]))
}
