//! A row whose features all have area 0 (a file with a zero scan interval gives area 0 everywhere) has the plain mean of their m/z and RT, never NaN;
//! a row with weight keeps the area-weighted mean bit for bit.

use mzlab_feat::align::{align, AlignParams};
use mzlab_feat::table::{FileFeatures, FLAG_GOOD};

fn file(mz: f64, rt: f32, area: f64) -> FileFeatures {
    let mut f = FileFeatures::default();
    f.mz.push(mz);
    f.rt.push(rt);
    f.left.push(rt - 0.03);
    f.right.push(rt + 0.03);
    f.area.push(area);
    f.height.push(2e5);
    f.flags.push(FLAG_GOOD);
    f.role.push(0);
    f.charge.push(1);
    f.family_id.push(0);
    f
}

fn row(files: &[FileFeatures]) -> (f64, f64) {
    let refs: Vec<&FileFeatures> = files.iter().collect();
    let t = align(&refs, &AlignParams::default()).unwrap();
    assert_eq!(t.len(), 1);
    (t.mz[0], t.rt[0])
}

#[test]
fn a_row_of_zero_areas_has_the_plain_mean_and_not_nan() {
    let (mz, rt) = row(&[file(300.1, 5.0, 0.0), file(300.1002, 5.02, 0.0)]);
    assert!(mz.is_finite() && rt.is_finite(), "mz {mz} rt {rt}");
    assert_eq!(mz, (300.1 + 300.1002) / 2.0);
    assert!((rt - 5.01).abs() < 1e-6, "rt {rt}");
}

#[test]
fn a_row_with_weight_keeps_the_weighted_mean_bit_for_bit() {
    let (mz, rt) = row(&[file(300.1, 5.0, 1e6), file(300.1002, 5.02, 3e6)]);
    let sum: f64 = 300.1 * 1e6 + 300.1002 * 3e6;
    assert_eq!(mz.to_bits(), (sum / (1e6 + 3e6)).to_bits());
    assert!(rt > 5.0 && rt < 5.02, "rt {rt}");
}

#[test]
fn one_file_with_area_decides_the_row_as_before() {
    let (mz, _) = row(&[file(300.1, 5.0, 0.0), file(300.1002, 5.02, 3e6)]);
    assert_eq!(mz.to_bits(), (300.1002f64 * 3e6 / 3e6).to_bits());
}
