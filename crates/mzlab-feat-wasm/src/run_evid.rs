//! The proofs step with the files of the series: the graph of the proofs and the view of the products, with the facts of the rows measured in the `.mzidx`
//! (`evid::rowfacts`) when the caller hands the files in. Called from `run.rs` (re-exported there) and from the bridge.

use mzlab_core::budget::{MemBudget, WriteAt};
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Value;
use mzlab_feat::evid::producers::{FeatIndex, Peaks};
use mzlab_feat::evid::rowfacts::{row_facts, row_near};
use mzlab_feat::evid::rowproofs::{row_signs, signs_of, Signs};
use mzlab_feat::evid::{
    graph_json, identities, sort_graph, Facts, GraphStream, Identity, Inputs, Kinetics, PointsTable,
};
use mzlab_feat::hypo::{Hypothesis, Rules};
use mzlab_feat::kinetic::{parent_kinetics, ChainBank, FormBank, Setup, K_PARENT_MIN};
use mzlab_feat::prof::{report, time};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::slabs::Stored;
use mzlab_feat::store::origin_from_meta;
use mzlab_feat::store::{read_file, read_table_from, write_table_tp, Origin, Slice};
use mzlab_feat::table::{Aligned, FileFeatures, FLAG_GOOD};
use mzlab_feat::target::join::JoinParams;
use mzlab_feat::tp::{build as build_tp, build_by_role, TpBlock, TpInputs, TpParams, TpRoles};

use crate::run::{is_unit, points_and_identity, HypoRun, IDENTITY_BYTES};

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
    evidence_sources_to(
        table,
        size,
        hypo,
        minutes,
        roles,
        parent,
        r,
        budget,
        None::<RowSources<Slice>>,
        sink,
    )
}

/// What the proofs read from the files to measure the rows: the stored `.mzfeatf` of the series (isotope patterns) and a way to open the `.mzidx` of file `k`
/// (the MS2 of each row, from the file that holds its best one).
pub struct RowSources<'a, O: ReadAt> {
    pub stored: &'a mut dyn Stored,
    pub open: &'a mut dyn FnMut(usize) -> Result<Index<O>>,
}

/// What `measured_facts` gives: the facts of every row, the fragments of the MS2 of the parent, the MS2 chosen for each row and the MS1 ions of the
/// infusion of the parent (sorted, empty without one).
type RowMeasures = (
    Vec<Facts>,
    Option<Vec<f64>>,
    Vec<Option<(Peaks, f32)>>,
    Vec<f64>,
);

/// The facts of every row measured in the files, and the fragments of the MS2 of the parent. The `.mzfeatf` read take their bytes from `budget` while the
/// facts are made (`error.memoryBudget` past it).
fn measured_facts<O: ReadAt>(
    aligned: &mzlab_feat::table::Aligned,
    hypo: &HypoRun,
    parent: Option<(f64, f64)>,
    unit: bool,
    spec: (&Recipe, Option<&TpRoles>),
    src: RowSources<O>,
    budget: &MemBudget,
) -> Result<RowMeasures> {
    let RowSources { stored, mut open } = src;
    let mut feats = Vec::new();
    let mut held = 0;
    for k in 0..stored.count() {
        let bytes = time("proofs: read stored", || stored.bytes(k))?;
        held += bytes.len();
        feats.push(time("proofs: parse stored", || read_file(&bytes))?.0);
    }
    let _g = budget.reserve("proofs", held)?;
    let ctx = mzlab_feat::export::Ctx {
        table: aligned,
        samples: &[],
        view: None,
        table_sha256: "",
        polarity: hypo.registry.polarity,
    };
    let purity = |row: usize| {
        let m = &aligned.meta;
        if m.len() == aligned.len() {
            m.get(row).ms2_purity
        } else {
            -1.0
        }
    };
    let ms2 = mzlab_feat::ms2spec::SpecOpts {
        unit,
        ..spec.0.ms2.clone()
    };
    let blanks = spec.1.map_or(&[][..], |x| &x.blank[..]);
    let spectra: Vec<Option<(Peaks, f32)>> = time("proofs: ms2 spectra", || {
        mzlab_feat::ms2spec::spectra_with(&ctx, &mut open, &ms2, blanks)
    })?
    .0
    .into_iter()
    .enumerate()
    .map(|(row, m)| m.map(|m| (Peaks { mz: m.mz, it: m.it }, purity(row))))
    .collect();
    let idx: Vec<FeatIndex> = feats.iter().map(FeatIndex::new).collect();
    let (facts, parent_spec) = time("proofs: row facts", || {
        row_facts(aligned, &hypo.registry, parent, &spectra, &idx, unit)
    });
    let ions = spec.1.map_or_else(Vec::new, |ro| {
        infusion_ions(&feats, &ro.infusion, spec.0.detect.min_height, unit)
    });
    Ok((facts, parent_spec.map(|p| p.mz), spectra, ions))
}

/// `measured_facts` when the files are given, else nothing measured.
fn measured_or_none<O: ReadAt>(
    aligned: &Aligned,
    hypo: &HypoRun,
    parent: Option<(f64, f64)>,
    spec: (&Recipe, Option<&TpRoles>),
    src: Option<RowSources<O>>,
    budget: &MemBudget,
    unit: bool,
) -> Result<RowMeasures> {
    match src {
        Some(src) => measured_facts(aligned, hypo, parent, unit, spec, src, budget),
        None => Ok(Default::default()),
    }
}

/// The m/z of the MS1 ions detected in the files of the infusion (good peaks or above `floor`), sorted, those within the tolerance of the one before
/// dropped.
fn infusion_ions(feats: &[FileFeatures], files: &[usize], floor: f64, unit: bool) -> Vec<f64> {
    let ppm = if unit {
        -mzlab_feat::axis::LR_TOL
    } else {
        TpParams::default().ppm
    };
    let mut mz: Vec<f64> = files
        .iter()
        .filter_map(|&k| feats.get(k))
        .flat_map(|f| {
            (0..f.mz.len())
                .filter(|&i| f.flags[i] & FLAG_GOOD != 0 || f64::from(f.height[i]) >= floor)
                .map(|i| f.mz[i])
        })
        .collect();
    mz.sort_by(f64::total_cmp);
    mz.dedup_by(|b, a| (*b - *a).abs() <= mzlab_feat::axis::half(*a, ppm));
    mz
}

/// The formula candidates of the rows formed and not explained, scored on the MS2 chosen for each row (none at unit resolution).
fn add_candidates(
    tp: &mut TpBlock,
    hypo: &HypoRun,
    unit: bool,
    aligned: &mzlab_feat::table::Aligned,
    spectra: &[Option<(Peaks, f32)>],
) {
    tp.add_fit_candidates(&hypo.parent, hypo.registry.polarity, unit, |r| {
        let peaks = spectra
            .get(r)
            .and_then(Option::as_ref)
            .map(|s| (&s.0.mz[..], &s.0.it[..]));
        (aligned.mz[r], peaks)
    });
}

/// The areas of the parent along the treated files (ones when the parent is not in the table).
fn parent_series(
    aligned: &mzlab_feat::table::Aligned,
    parent: Option<(f64, f64)>,
    treated: &[usize],
) -> Vec<f64> {
    parent
        .and_then(|(mz, rt)| row_near(aligned, mz, rt, 0.3))
        .map(|row| {
            treated
                .iter()
                .map(|&k| aligned.area[row * aligned.n_files + k])
                .collect()
        })
        .unwrap_or_else(|| vec![1.0; treated.len()])
}

/// `evidence_roles_to` that also measures the rows in the files when `src` is given (isotope patterns, purity and shared fragments of the MS2, the
/// fragments of the parent for gate 2); without it the output is the one of `evidence_roles_to`.
#[allow(clippy::too_many_arguments)]
pub fn evidence_sources_to<R: ReadAt, W: WriteAt, O: ReadAt>(
    table: &mut R,
    size: u64,
    hypo: &HypoRun,
    minutes: &[f64],
    roles: Option<&TpRoles>,
    parent: Option<(f64, f64)>,
    r: &Recipe,
    budget: &MemBudget,
    src: Option<RowSources<O>>,
    sink: &mut W,
) -> Result<()> {
    let (aligned, meta, points) = read_aligned(table, size, hypo)?;
    let origin = origin_from_meta(&meta);
    let minutes = roles.map_or(minutes, |ro| &ro.minutes);
    let (treated, controls) = TpRoles::kinetic_files(minutes, roles);
    let times: Vec<f64> = treated.iter().map(|&k| minutes[k]).collect();
    let (forms, chain) = (FormBank::new(&times), ChainBank::new(&times, K_PARENT_MIN));
    let y = parent_series(&aligned, parent, &treated);
    let setup = Setup {
        times: &times,
        forms: &forms,
        chain: &chain,
        parent: parent_kinetics(&forms, &times, &y, None),
        pseudo: mzlab_feat::trace::FLOOR * 3.0,
    };
    let (facts, fragments, spectra, infusion) = measured_or_none(
        &aligned,
        hypo,
        parent,
        (r, roles),
        src,
        budget,
        is_unit(&meta),
    )?;
    let inp = Inputs {
        table: &points,
        aligned: &aligned,
        facts: &facts,
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
    let (mut ids, signs) = identities_and_signs(&inp, budget, parent.is_some());
    let mut tp = time("proofs: tp view", || {
        tp_block(
            &inp,
            minutes,
            roles,
            &hypo.registry.hyps,
            parent,
            (
                fragments.as_deref(),
                Some(&infusion[..]).filter(|i| !i.is_empty()),
            ),
            rules.as_ref(),
            is_unit(&meta),
            &signs,
        )
    });
    add_candidates(&mut tp, hypo, is_unit(&meta), &aligned, &spectra);
    let held = aligned.heap_bytes() + tp.heap_bytes();
    if !budget.in_memory(held + aligned.len() * IDENTITY_BYTES) {
        ids = None;
    }
    let out = emit(sink, budget, ids, &inp, (&points, &tp), (r, &origin), held);
    report("evidence");
    out
}

/// The table of the series with the rows of the hypotheses added, in order, and its points table.
fn read_aligned<R: ReadAt>(
    table: &mut R,
    size: u64,
    hypo: &HypoRun,
) -> Result<(Aligned, Value, PointsTable)> {
    let (mut aligned, meta) = time("proofs: read table", || read_table_from(table, size))?;
    time("proofs: add missing", || {
        hypo.result
            .add_missing_to(&mut aligned, &JoinParams::default());
        aligned.sort_rows();
    });
    let points = time("proofs: points", || {
        points_and_identity(&mut aligned, hypo, &meta)
    })?;
    Ok((aligned, meta, points))
}

/// The identity of every row when they fit in memory (one identity per row serves both the signs of the view of the products and the graph: the same
/// pure function of the row), and the signs of the proofs of the rows (none without a parent in the table).
fn identities_and_signs(
    inp: &Inputs,
    budget: &MemBudget,
    has_parent: bool,
) -> (Option<Vec<Identity>>, Vec<Signs>) {
    let ids = budget
        .in_memory(inp.aligned.heap_bytes() + inp.aligned.len() * IDENTITY_BYTES)
        .then(|| time("proofs: identities", || identities(inp)));
    let signs = match (&ids, has_parent) {
        (_, false) => Vec::new(),
        (Some(v), _) => v.iter().map(signs_of).collect(),
        (None, _) => time("proofs: row signs", || row_signs(inp)),
    };
    (ids, signs)
}

/// The table with the view of the products and the graph of the proofs: from `ids` (the identities already made, in the order of the rows) when
/// they fit in memory, in pieces otherwise.
#[allow(clippy::too_many_arguments)]
fn emit<W: WriteAt>(
    sink: &mut W,
    budget: &MemBudget,
    ids: Option<Vec<Identity>>,
    inp: &Inputs,
    (points, tp): (&PointsTable, &TpBlock),
    (r, origin): (&Recipe, &Origin),
    held: usize,
) -> Result<()> {
    let aligned = inp.aligned;
    match ids {
        Some(mut ids) => {
            sort_graph(&mut ids);
            let json = time("proofs: graph", || graph_json(&ids, points));
            time("proofs: write", || {
                write_table_tp(sink, aligned, tp, r, origin, &mut json.as_bytes())
            })
        }
        None => {
            let need = held + aligned.len() * GraphStream::bytes_per_row();
            let _g = budget.reserve("proofs", need)?;
            let mut graph = GraphStream::new(inp);
            time("proofs: write", || {
                write_table_tp(sink, aligned, tp, r, origin, &mut graph)
            })
        }
    }
}

/// The view of the products of the table (`tp/1`): empty without a parent in the table or without controls to read the gates against.
#[allow(clippy::too_many_arguments)]
fn tp_block(
    inp: &Inputs,
    minutes: &[f64],
    roles: Option<&TpRoles>,
    hyps: &[Hypothesis],
    parent: Option<(f64, f64)>,
    (fragments, infusion): (Option<&[f64]>, Option<&[f64]>),
    rules: Option<&Rules>,
    unit: bool,
    signs: &[Signs],
) -> TpBlock {
    let Some(parent) = parent else {
        return TpBlock::default();
    };
    let params = TpParams {
        ppm: if unit {
            -mzlab_feat::axis::LR_TOL
        } else {
            TpParams::default().ppm
        },
        ..TpParams::default()
    };
    let tp = TpInputs {
        aligned: inp.aligned,
        minutes,
        hyps,
        signs,
        parent,
        fragments,
        infusion,
        params: &params,
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
