//! The proofs step with the files of the series: the graph of the proofs and the view of the products, with the facts of the rows measured in the `.mzidx`
//! (`evid::rowfacts`) when the caller hands the files in. Called from `run.rs` (re-exported there) and from the bridge.

use mzlab_core::budget::{MemBudget, WriteAt};
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_feat::evid::producers::{FeatIndex, Peaks};
use mzlab_feat::evid::rowfacts::{row_facts, row_near};
use mzlab_feat::evid::rowproofs::row_signs;
use mzlab_feat::evid::{build_graph, graph_json, Facts, GraphStream, Inputs, Kinetics};
use mzlab_feat::hypo::{Hypothesis, Rules};
use mzlab_feat::kinetic::{parent_kinetics, ChainBank, FormBank, Setup, K_PARENT_MIN};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::slabs::Stored;
use mzlab_feat::store::origin_from_meta;
use mzlab_feat::store::{read_file, read_table_from, write_table_tp, Slice};
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

/// The facts of every row measured in the files, and the fragments of the MS2 of the parent. The `.mzfeatf` read take their bytes from `budget` while the
/// facts are made (`error.memoryBudget` past it).
fn measured_facts<O: ReadAt>(
    aligned: &mzlab_feat::table::Aligned,
    hypo: &HypoRun,
    parent: Option<(f64, f64)>,
    unit: bool,
    spec: (&mzlab_feat::ms2spec::SpecOpts, Option<&TpRoles>),
    src: RowSources<O>,
    budget: &MemBudget,
) -> Result<(Vec<Facts>, Option<Vec<f64>>)> {
    let RowSources { stored, mut open } = src;
    let mut feats = Vec::new();
    let mut held = 0;
    for k in 0..stored.count() {
        let bytes = stored.bytes(k)?;
        held += bytes.len();
        feats.push(read_file(&bytes)?.0);
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
        ..spec.0.clone()
    };
    let blanks = spec.1.map_or(&[][..], |x| &x.blank[..]);
    let spectra: Vec<Option<(Peaks, f32)>> =
        mzlab_feat::ms2spec::spectra_with(&ctx, &mut open, &ms2, blanks)?
            .0
            .into_iter()
            .enumerate()
            .map(|(row, m)| m.map(|m| (Peaks { mz: m.mz, it: m.it }, purity(row))))
            .collect();
    let idx: Vec<FeatIndex> = feats.iter().map(FeatIndex::new).collect();
    let (facts, parent_spec) = row_facts(aligned, &hypo.registry, parent, &spectra, &idx, unit);
    Ok((facts, parent_spec.map(|p| p.mz)))
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
        .and_then(|(mz, rt)| row_near(&aligned, mz, rt, 0.3))
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
    let (facts, fragments) = match src {
        Some(src) => measured_facts(
            &aligned,
            hypo,
            parent,
            is_unit(&meta),
            (&r.ms2, roles),
            src,
            budget,
        )?,
        None => (Vec::new(), None),
    };
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
    let tp = tp_block(
        &inp,
        minutes,
        roles,
        &hypo.registry.hyps,
        parent,
        fragments.as_deref(),
        rules.as_ref(),
    );
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
    fragments: Option<&[f64]>,
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
        fragments,
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
