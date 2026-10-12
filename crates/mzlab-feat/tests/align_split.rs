//! Close isomers: two peaks 0.06 min apart in every file are one row with the RT gap of the alignment and two rows with `split_gap`.

use mzlab_feat::align::{align, AlignParams};
use mzlab_feat::table::{FileFeatures, FLAG_GOOD};

/// A file with `peaks` (RT) at the same m/z.
fn file(peaks: &[f32]) -> FileFeatures {
    let mut f = FileFeatures::default();
    for &rt in peaks {
        f.mz.push(300.1);
        f.rt.push(rt);
        f.left.push(rt - 0.03);
        f.right.push(rt + 0.03);
        f.area.push(1e6);
        f.height.push(2e5);
        f.flags.push(FLAG_GOOD);
        f.role.push(0);
        f.charge.push(1);
        f.family_id.push(0);
    }
    f
}

fn series(files: usize, peaks: &[f32]) -> Vec<FileFeatures> {
    (0..files).map(|_| file(peaks)).collect()
}

#[test]
fn two_isomers_seen_in_many_files_are_two_rows() {
    let files = series(10, &[5.0, 5.06]);
    let refs: Vec<&FileFeatures> = files.iter().collect();
    assert_eq!(align(&refs, &AlignParams::default()).len(), 1);
    let split = AlignParams {
        split_gap: Some(0.03),
        ..AlignParams::default()
    };
    assert_eq!(align(&refs, &split).len(), 2);
}

#[test]
fn a_double_peak_in_few_files_stays_one_row() {
    let mut files = series(10, &[5.0]);
    files[0] = file(&[5.0, 5.06]);
    files[1] = file(&[5.0, 5.06]);
    let refs: Vec<&FileFeatures> = files.iter().collect();
    let split = AlignParams {
        split_gap: Some(0.03),
        ..AlignParams::default()
    };
    assert_eq!(align(&refs, &split).len(), 1);
}
