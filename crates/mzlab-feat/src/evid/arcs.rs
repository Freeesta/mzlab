//! The arcs of the families (made inside each file, with the proofs of `edge_evidence`) on the rows of the aligned table: an arc belongs to the row that
//! holds both of its ions as members. The same pair seen in several files keeps the arc with the best correlation of the traces.

use super::{Evidence, Kind, Sign};
use crate::aligned::Aligned;
use crate::family::evidence::{EdgeKind, Proof, ProofKind, PROOF_VERSION};
use crate::family::Families;
use crate::table::FileFeatures;

/// m/z window (ppm) and RT window (min) to find the ion of a file among the members of a row.
const LOCATE_PPM: f64 = 5.0;
const LOCATE_RT: f64 = 0.3;

/// A file with its families.
pub struct FileFamilies<'a> {
    pub feats: &'a FileFeatures,
    pub fam: &'a Families,
}

/// An arc on a row of the aligned table: `lo` and `up` are positions among the members of the row (0 = the reference ion).
#[derive(Clone, Debug)]
pub struct RowArc {
    pub row: usize,
    pub lo: usize,
    pub up: usize,
    pub r: f32,
    pub kind: EdgeKind,
    pub readings: Vec<String>,
    pub proofs: Vec<Proof>,
}

/// Every ion of the table with its row and its place among the members, ascending by m/z.
fn ions(t: &Aligned) -> Vec<(f64, f64, usize, usize)> {
    let mut v = Vec::new();
    for r in 0..t.len() {
        let m = t.members(r);
        if m.is_empty() {
            v.push((t.mz[r], t.rt[r], r, 0));
        }
        for (pos, k) in m.clone().enumerate() {
            v.push((t.member_mz[k], f64::from(t.member_rt[k]), r, pos));
        }
    }
    v.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.2.cmp(&b.2)).then(a.3.cmp(&b.3)));
    v
}

fn locate(ions: &[(f64, f64, usize, usize)], mz: f64, rt: f64) -> Vec<(usize, usize)> {
    let tol = mz * LOCATE_PPM * 1e-6;
    let from = ions.partition_point(|i| i.0 < mz - tol);
    ions[from..]
        .iter()
        .take_while(|i| i.0 <= mz + tol)
        .filter(|i| (i.1 - rt).abs() <= LOCATE_RT)
        .map(|i| (i.2, i.3))
        .collect()
}

/// The arcs of all the files on the rows of the table, ordered by row and pair.
pub fn arcs_from_families(t: &Aligned, files: &[FileFamilies]) -> Vec<RowArc> {
    let ions = ions(t);
    let mut out: Vec<RowArc> = Vec::new();
    for f in files {
        for a in &f.fam.arcs {
            let (lo, up) = (a.lo as usize, a.up as usize);
            let at = |i: usize| locate(&ions, f.feats.mz[i], f64::from(f.feats.rt[i]));
            let (l, u) = (at(lo), at(up));
            let Some(&(row, pl)) = l.iter().find(|x| u.iter().any(|y| y.0 == x.0)) else {
                continue;
            };
            let pu = u.iter().find(|y| y.0 == row).map_or(0, |y| y.1);
            let arc = RowArc {
                row,
                lo: pl,
                up: pu,
                r: a.r,
                kind: a.kind,
                readings: a.readings.iter().map(|x| x.name.clone()).collect(),
                proofs: a.proofs.clone(),
            };
            match out
                .iter_mut()
                .find(|x| (x.row, x.lo, x.up) == (row, pl, pu))
            {
                Some(old) if old.r < arc.r => *old = arc,
                Some(_) => {}
                None => out.push(arc),
            }
        }
    }
    out.sort_by_key(|a| (a.row, a.lo, a.up));
    out
}

/// A proof of an arc as a record of the graph: co-elution is a proof of the RT, the rest belongs to the network. `points` are those of the proof.
pub fn proof_evidence(p: &Proof) -> Evidence {
    let kind = if p.kind == ProofKind::Coelution {
        Kind::Rt
    } else {
        Kind::Network
    };
    let sign = match p.sign {
        s if s > 0 => Sign::For,
        s if s < 0 => Sign::Against,
        _ => Sign::Neutral,
    };
    Evidence {
        kind,
        value: f64::from(p.value),
        threshold: Some(f64::from(p.threshold)),
        points: if sign == Sign::For {
            f64::from(p.points).abs()
        } else {
            0.0
        },
        sign,
        source: p.source,
        version: PROOF_VERSION,
        params: vec![("proof", f64::from(p.kind as u8))],
    }
}
