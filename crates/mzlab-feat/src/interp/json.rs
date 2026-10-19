//! The JSON of the block: `interp/1` of one row and the meta of the `.mzfeat` (`interp`: parameters, parent, calibration of the files, library of the
//! tree, registry `filtri/1`). Numbers are rounded as the page shows them.

use mzlab_core::json::{round, Out};

use super::build::{Output, RowInterp};
use super::filtri::{Registry, STAGES};
use super::{ms2link, SCHEMA, VERSION};
use crate::legacy::series::Parent;

fn pair(o: &mut Out, obs: f64, exp: f64) {
    o.raw("{\"obs\":")
        .num(round(obs, 5))
        .raw(",\"exp\":")
        .num(round(exp, 5))
        .raw("}");
}

fn spectrum(o: &mut Out, c: &crate::legacy::ms2::Consensus) {
    o.raw("{\"mz\":")
        .rounded(&c.mz, 4)
        .raw(",\"rel\":")
        .rounded(&c.rel, 1)
        .raw("}");
}

fn list(o: &mut Out, v: &[f64]) {
    o.rounded(v, 4);
}

/// The block `interp/1` of a row.
pub fn row_json(r: &RowInterp, parent: &Parent) -> String {
    let mut o = Out::default();
    o.raw("{\"schema\":")
        .str(SCHEMA)
        .raw(",\"id\":")
        .int(i64::from(r.id));
    o.raw(",\"parent\":{\"smiles\":");
    match &parent.smiles {
        Some(s) => o.str(s),
        None => o.raw("null"),
    };
    o.raw(",\"ion_formula\":")
        .str(&mzlab_core::elements::fmt(&parent.ion))
        .raw(",\"mz\":")
        .num(round(parent.mz, 5))
        .raw("}");
    change(&mut o, r);
    ms1(&mut o, r);
    ms2(&mut o, r);
    vs_parent(&mut o, r);
    tree(&mut o, r);
    super::sito_json::write(&mut o, r);
    o.raw(",\"hidden\":");
    match r.hidden {
        Some(h) => o.str(h),
        None => o.raw("null"),
    };
    o.raw("}");
    o.0
}

fn change(o: &mut Out, r: &RowInterp) {
    o.raw(",\"change\":");
    let Some(c) = r.change.first() else {
        o.raw("null");
        return;
    };
    o.raw("{\"delta\":")
        .str(&c.delta)
        .raw(",\"neutral_formula\":")
        .str(&c.neutral_formula);
    o.raw(",\"ion_formula\":")
        .str(&c.ion_formula)
        .raw(",\"ppm\":")
        .num(round(c.ppm, 2));
    o.raw(",\"source\":")
        .str(c.source)
        .raw(",\"alternatives\":[");
    for (i, a) in r.change[1..].iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"delta\":")
            .str(&a.delta);
        o.raw(",\"ion_formula\":")
            .str(&a.ion_formula)
            .raw(",\"ppm\":")
            .num(round(a.ppm, 2))
            .raw("}");
    }
    o.raw("]}");
}

fn ms1(o: &mut Out, r: &RowInterp) {
    o.raw(",\"ms1\":");
    let Some(i) = &r.iso else {
        o.raw("null");
        return;
    };
    o.raw("{\"m1\":");
    pair(o, i.m1_obs, i.m1_exp);
    o.raw(",\"m2\":");
    pair(o, i.m2_obs, i.m2_exp);
    o.raw(",\"s_count\":").str(i.s_count);
    if let Some((mz, k)) = r.mz_obs {
        o.raw(",\"mz_obs\":")
            .num(round(mz, 5))
            .raw(",\"file\":")
            .int(k as i64);
    }
    o.raw("}");
}

fn ms2(o: &mut Out, r: &RowInterp) {
    o.raw(",\"ms2\":{\"scans\":[");
    for (i, s) in r.scans.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"file\":")
            .int(s.file as i64)
            .raw(",\"scan\":")
            .int(s.scan as i64);
        o.raw(",\"rt\":")
            .num(round(s.rt, 4))
            .raw(",\"precursor\":")
            .num(round(s.precursor, 5));
        o.raw(",\"iso\":")
            .floats(&[round(s.iso.0, 3), round(s.iso.1, 3)])
            .raw(",\"ce\":")
            .opt(s.ce);
        o.raw(",\"scan_range\":");
        match s.scan_range {
            Some((a, b)) => o.floats(&[a, b]),
            None => o.raw("null"),
        };
        o.raw(",\"purity\":")
            .num(round(s.purity, 3))
            .raw(",\"chimeric\":")
            .raw(if s.chimeric { "true" } else { "false" })
            .raw("}");
    }
    o.raw("],\"files_with_ms2\":").int(r.files_with_ms2 as i64);
    o.raw(",\"consensus\":");
    spectrum(o, &r.pooled.consensus);
    o.raw(",\"isomers\":").str(r.isomers());
    if let ms2link::Isomers::Separated(_) = r.pooled.isomers {
        o.raw(",\"isomer_consensus\":[");
        for (i, g) in r.pooled.groups.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" });
            spectrum(o, g);
        }
        o.raw("]");
    }
    o.raw("}");
}

fn vs_parent(o: &mut Out, r: &RowInterp) {
    o.raw(",\"vs_parent\":");
    let Some(v) = &r.vs_parent else {
        o.raw("null");
        return;
    };
    o.raw("{\"ref\":")
        .str(v.reference)
        .raw(",\"cos_mod\":")
        .num(round(v.cos_mod, 4))
        .raw(",\"entropy\":")
        .num(round(v.entropy, 4));
    for (k, a) in [
        ("equal", &v.equal),
        ("shifted", &v.shifted),
        ("losses", &v.losses),
        ("formula_equal", &v.formula_equal),
        ("formula_shifted", &v.formula_shifted),
    ] {
        o.raw(",").str(k).raw(":");
        list(o, a);
    }
    o.raw("}");
}

fn tree(o: &mut Out, r: &RowInterp) {
    o.raw(",\"tree\":");
    if r.tree.is_empty() {
        o.raw("null");
        return;
    }
    o.raw("{\"nodes\":[");
    for (i, n) in r.tree.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"id\":")
            .str(&n.id)
            .raw(",\"formula\":")
            .str(&n.formula);
        o.raw(",\"mz\":")
            .num(round(n.mz, 5))
            .raw(",\"state\":")
            .str(n.state)
            .raw("}");
    }
    let count = |s: &str| r.tree.iter().filter(|n| n.state == s).count();
    o.raw("],\"n_same\":")
        .int(count("same") as i64)
        .raw(",\"n_shifted\":")
        .int(count("shifted") as i64)
        .raw("}");
}

/// The registry `filtri/1`.
pub fn filtri_json(reg: &Registry) -> String {
    let mut o = Out::default();
    o.raw("{\"schema\":")
        .str(super::filtri::SCHEMA)
        .raw(",\"stages\":[");
    for (i, (id, name, why)) in STAGES.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"id\":")
            .str(id)
            .raw(",\"name\":")
            .str(name);
        o.raw(",\"hidden\":")
            .int(reg.hidden[i].len() as i64)
            .raw(",\"ids\":[");
        for (j, id) in reg.hidden[i].iter().enumerate() {
            o.raw(if j > 0 { "," } else { "" }).int(i64::from(*id));
        }
        o.raw("],\"why\":").str(why).raw("}");
    }
    o.raw("],\"decoys\":");
    match reg.decoys {
        Some((rules, survived)) => o
            .raw("{\"rules\":")
            .int(rules as i64)
            .raw(",\"survived\":")
            .int(survived as i64)
            .raw("}"),
        None => o.raw("null"),
    };
    o.raw("}");
    o.0
}

/// The object `interp` of the meta of the `.mzfeat`.
pub fn meta_json(out: &Output, parent: &Parent) -> String {
    let mut o = Out::default();
    o.raw("{\"version\":")
        .int(VERSION)
        .raw(",\"parent\":{\"ion_formula\":")
        .str(&mzlab_core::elements::fmt(&parent.ion));
    o.raw(",\"mz\":")
        .num(round(parent.mz, 5))
        .raw("},\"params\":{\"select_ppm\":")
        .num(ms2link::SELECT_PPM);
    o.raw(",\"purity_min\":")
        .num(ms2link::PURITY_MIN)
        .raw(",\"iso_corr\":")
        .num(ms2link::ISO_CORR)
        .raw("},\"calibration\":[");
    for (i, c) in out.cals.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"median_ppm\":")
            .num(round(c.median_ppm, 2))
            .raw(",\"mad_ppm\":")
            .num(round(c.mad_ppm, 2));
        o.raw(",\"n\":")
            .int(c.n as i64)
            .raw(",\"tolerance\":")
            .num(round(c.tolerance, 2))
            .raw("}");
    }
    o.raw("],\"msnlib\":{");
    for (k, v) in [("nodes", &out.nodes), ("entries", &out.entries)] {
        o.raw(if k == "nodes" { "" } else { "," }).str(k).raw(":[");
        for (i, n) in v.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" })
                .raw("{\"id\":")
                .str(&n.id)
                .raw(",\"formula\":")
                .str(&n.formula);
            o.raw(",\"mz\":").num(round(n.mz, 5)).raw("}");
        }
        o.raw("]");
    }
    o.raw("},\"filtri\":")
        .raw(&filtri_json(&out.registry))
        .raw("}");
    o.0
}
