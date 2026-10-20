//! The family of every peak as a record, `famiglia/1`: the members of each family (m/z, RT, role, parent row) and, for every arc, the readings of
//! its step, the shape correlation and the proofs of its label. Numbers and words only, no verdict. Read by `examples/famiglie.rs`; the size of a
//! family is the length of `members`. Same input gives the same bytes.

use mzlab_core::json::{round, Out};

use crate::family::evidence::{EdgeKind, ProofKind};
use crate::family::rel::RKind;
use crate::family::{Arc, Families};
use crate::table::*;

pub const SCHEMA: &str = "famiglia/1";

pub fn role_name(r: u8) -> &'static str {
    match r {
        ROLE_M => "M",
        ROLE_ISOTOPE => "isotopologue",
        ROLE_ADDUCT => "adduct",
        ROLE_ISF => "in_source_fragment",
        ROLE_DIMER => "dimer",
        ROLE_UNDETERMINED => "undetermined",
        _ => "none",
    }
}

fn kind_name(k: EdgeKind) -> &'static str {
    match k {
        EdgeKind::Adduct => "adduct",
        EdgeKind::Isf => "isf",
        EdgeKind::Undetermined => "undetermined",
        EdgeKind::Other => "other",
    }
}

fn reading_name(k: RKind) -> &'static str {
    match k {
        RKind::Adduct => "adduct",
        RKind::Loss => "loss",
        RKind::Dimer => "dimer",
        RKind::Isotope => "isotope",
    }
}

fn proof_name(k: ProofKind) -> &'static str {
    match k {
        ProofKind::Step => "step",
        ProofKind::Coelution => "coelution",
        ProofKind::FragmentInMs2 => "fragment_in_ms2",
        ProofKind::Ratio => "ratio",
        ProofKind::Skeleton => "skeleton",
        ProofKind::TypicalLoss => "typical_loss",
        ProofKind::NoFragmentation => "no_fragmentation",
    }
}

fn member(o: &mut Out, f: &FileFeatures, fam: &Families, i: usize) {
    let par = fam.parent[i];
    o.raw("{\"row\":")
        .int(i as i64)
        .raw(",\"mz\":")
        .num(round(f.mz[i], 5))
        .raw(",\"rt\":")
        .num(round(f64::from(f.rt[i]), 3))
        .raw(",\"role\":")
        .str(role_name(fam.role[i]))
        .raw(",\"parent\":");
    if par == u32::MAX {
        o.raw("null");
    } else {
        o.int(i64::from(par));
    }
    o.raw("}");
}

fn arc(o: &mut Out, f: &FileFeatures, a: &Arc) {
    let (lo, up) = (a.lo as usize, a.up as usize);
    o.raw("{\"lo\":")
        .int(i64::from(a.lo))
        .raw(",\"up\":")
        .int(i64::from(a.up))
        .raw(",\"delta_mz\":")
        .num(round(f.mz[up] - f.mz[lo], 5))
        .raw(",\"delta_rt\":")
        .num(round(f64::from(f.rt[up] - f.rt[lo]), 3))
        .raw(",\"r\":")
        .num(round(f64::from(a.r), 3))
        .raw(",\"kind\":")
        .str(kind_name(a.kind))
        .raw(",\"readings\":[");
    for (k, r) in a.readings.iter().enumerate() {
        if k > 0 {
            o.raw(",");
        }
        o.raw("{\"kind\":")
            .str(reading_name(r.kind))
            .raw(",\"name\":")
            .str(&r.name)
            .raw(",\"step\":")
            .num(round(r.step, 5))
            .raw(",\"ppm\":")
            .num(round(r.ppm, 2))
            .raw("}");
    }
    o.raw("],\"proofs\":[");
    for (k, p) in a.proofs.iter().enumerate() {
        if k > 0 {
            o.raw(",");
        }
        o.raw("{\"kind\":")
            .str(proof_name(p.kind))
            .raw(",\"value\":")
            .num(round(f64::from(p.value), 4))
            .raw(",\"threshold\":")
            .num(round(f64::from(p.threshold), 4))
            .raw(",\"points\":")
            .num(round(f64::from(p.points), 3))
            .raw(",\"sign\":")
            .int(i64::from(p.sign))
            .raw("}");
    }
    o.raw("]}");
}

/// `famiglia/1` of one file: `{schema, file, families: [{family, size, members, arcs}]}`, families in the order of their M row.
pub fn record(file: &str, f: &FileFeatures, fam: &Families) -> String {
    let mut o = Out::default();
    o.raw("{\"schema\":")
        .str(SCHEMA)
        .raw(",\"file\":")
        .str(file)
        .raw(",\"families\":[");
    let mut first = true;
    for m in (0..f.len()).filter(|&i| fam.role[i] == ROLE_M) {
        let id = fam.family_id[m];
        if !first {
            o.raw(",");
        }
        first = false;
        // M first, then the others by row
        let members: Vec<usize> = std::iter::once(m)
            .chain((0..f.len()).filter(|&i| i != m && fam.family_id[i] == id))
            .collect();
        o.raw("{\"family\":")
            .int(i64::from(id))
            .raw(",\"size\":")
            .int(members.len() as i64)
            .raw(",\"members\":[");
        for (k, &i) in members.iter().enumerate() {
            if k > 0 {
                o.raw(",");
            }
            member(&mut o, f, fam, i);
        }
        o.raw("],\"arcs\":[");
        let mut sep = false;
        for a in fam
            .arcs
            .iter()
            .filter(|a| fam.family_id[a.lo as usize] == id)
        {
            if sep {
                o.raw(",");
            }
            sep = true;
            arc(&mut o, f, a);
        }
        o.raw("]}");
    }
    o.raw("]}");
    o.0
}
