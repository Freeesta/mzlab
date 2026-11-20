//! The centroids of an index in memory against the route of the Flow (`spectra.centroid`, then the index of the centroid mzML): the same peaks.

use super::tests::profile_file;
use super::{centroid_index, mzml_to_centroids, Over};
use crate::idxread::Index;
use crate::mzidx;
use crate::upload;

fn index_of(mzml: &[u8]) -> Index<Vec<u8>> {
    let p = upload::prepare(&mut mzml.to_vec(), mzml.len() as u64, "demo.mzML").unwrap();
    let (bytes, _) = mzidx::build_mzml_to_vec(&p.table, mzml, mzidx::DEFAULT_BUDGET).unwrap();
    let n = bytes.len() as u64;
    Index::open(bytes, n).unwrap()
}

fn peaks(ix: &mut Index<Vec<u8>>) -> Vec<(usize, f64, f64)> {
    let si = ix.section_of(1, Some(1)).unwrap();
    let n = ix.sections[si].n_peaks;
    let mut out = Vec::new();
    ix.each_peak(si, 0, n, |p, mz, y| out.push((p, mz, y))).unwrap();
    out
}

#[test]
fn the_centroids_of_an_index_are_those_of_the_centroid_mzml() {
    let src = profile_file(40, true);
    let mut ix = index_of(&src);
    assert!(ix.scans.iter().all(|s| s.profile));
    let made = centroid_index(&mut ix, Over::default()).unwrap().unwrap();
    let n = made.bytes.len() as u64;
    let mut mine = Index::open(made.bytes, n).unwrap();
    assert!(mine.scans.iter().all(|s| !s.profile));
    let mut out = Vec::new();
    assert!(mzml_to_centroids(|| Ok(src.as_slice()), &mut out, "demo.mzML", &mut |_, _| {}).unwrap());
    let mut theirs = index_of(&out);
    let (a, b) = (peaks(&mut mine), peaks(&mut theirs));
    assert!(a.len() >= 40 * 3, "{}", a.len());
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert!(x.0 == y.0 && (x.1 - y.1).abs() < 1e-9 && (x.2 - y.2).abs() < 1e-6, "{x:?} {y:?}");
    }
}

#[test]
fn an_index_of_centroids_is_left_alone() {
    let src = profile_file(10, false);
    let mut ix = index_of(&src);
    assert!(centroid_index(&mut ix, Over::default()).unwrap().is_none());
}
