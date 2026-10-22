//! The proof "MSn tree of a reference": the MS2 of a row against the entries of an `msnlib/1` library (`docs/agenti/msnlib.md`). It becomes an
//! `ExternalHit` whose score is the entropy similarity of the best MS2 of the entry; it moves the order of the candidates and never gives a level.

use super::ExternalHit;
use mzlab_core::error::Result;
use mzlab_core::msngrafo::cerca_ms2;

pub const SOURCE: &str = "MSn tree of a reference";
const PPM: f64 = 10.0;
const TOL: f64 = 0.01;
const MIN_SHARED: usize = 3;

/// The MS2 of a row of the table: precursor m/z, polarity and the peaks `(m/z, intensity)`.
pub struct RowMs2<'a> {
    pub row: usize,
    pub mz: f64,
    pub positive: bool,
    pub peaks: &'a [(f64, f64)],
}

/// One hit per library entry that resembles the MS2 of a row; empty when the library has no entry at the precursor.
pub fn msn_tree_hits(library: &str, rows: &[RowMs2]) -> Result<Vec<ExternalHit>> {
    let mut out = Vec::new();
    for r in rows {
        for h in cerca_ms2(library, r.mz, r.positive, r.peaks, PPM, TOL, MIN_SHARED)? {
            out.push(ExternalHit {
                row: r.row,
                name: format!("{} {}", h.id, h.ion),
                score: h.entropy,
                source: SOURCE,
            });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIB: &str = r#"{"formato":"msnlib/1","voci":[{"id":"STD","ione":{"formula":"C1"},"grafo":{"polarita":1,"radici":["n0"],
        "nodi":[{"id":"n0","mz":305.1}],"archi":[],"spettri":[{"albero":"305@cid35","livello":2,"att":"cid","ce":35,"precursore":"n0",
        "picchi":[{"mz":261.1,"rel":100},{"mz":244.1,"rel":50},{"mz":217.1,"rel":20}]}]}}]}"#;

    #[test]
    fn a_row_with_the_ms2_of_a_reference_gets_a_hit_and_the_others_none() {
        let peaks = [(261.1, 100.0), (244.1, 50.0), (217.1, 20.0)];
        let rows = [
            RowMs2 {
                row: 4,
                mz: 305.1,
                positive: true,
                peaks: &peaks,
            },
            RowMs2 {
                row: 7,
                mz: 400.0,
                positive: true,
                peaks: &peaks,
            },
        ];
        let hits = msn_tree_hits(LIB, &rows).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!((hits[0].row, hits[0].source), (4, SOURCE));
        assert!(hits[0].score > 0.99 && hits[0].name.starts_with("STD"));
    }
}
