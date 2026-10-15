//! The reference ion of a family at unit resolution: an ion split in two centroids in one file votes once for that file, so a weak neighbour
//! (+0.95 Da, an isotope-like piece of the strong ion) does not take the reference from the strong ion (invented ions).

use mzlab_feat::famalign::align_families;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::table::{FileFeatures, FLAG_GOOD, ROLE_ISOTOPE, ROLE_M};

const FILES: usize = 6;
const X: f64 = 300.15;
const RT: f32 = 6.0;

/// File `k`: the strong ion X (root of the family) and its +0.95 Da neighbour, as one ISOTOPE piece and, in the first four files, two M pieces.
fn file(k: usize) -> FileFeatures {
    let rt = RT + 0.02 * k as f32;
    let mut ions: Vec<(f64, f64, u8)> = vec![(X, 1e8, ROLE_M), (X + 0.95, 1e7, ROLE_ISOTOPE)];
    if k < 4 {
        ions.push((X + 0.90, 4e6, ROLE_M));
        ions.push((X + 1.05, 4e6, ROLE_M));
    }
    ions.sort_by(|a, b| a.0.total_cmp(&b.0));
    let root = ions.iter().position(|i| i.0 == X).unwrap();
    let mut f = FileFeatures::default();
    for ion in &ions {
        f.mz.push(ion.0);
        f.rt.push(rt);
        f.left.push(rt - 0.05);
        f.right.push(rt + 0.05);
        f.area.push(ion.1);
        f.height.push((ion.1 / 40.0) as f32);
        f.flags.push(FLAG_GOOD);
        f.role.push(ion.2);
        f.charge.push(1);
        f.family_id.push(root as u32 + 1);
    }
    f
}

#[test]
fn a_split_neighbour_does_not_take_the_reference_from_the_strong_ion() {
    let files: Vec<FileFeatures> = (0..FILES).map(file).collect();
    let refs: Vec<&FileFeatures> = files.iter().collect();
    let p = Recipe::lr(0.0, 2e3, 2e4).align;
    let al = align_families(&refs, &p).unwrap();
    let rows = al.rows_near(X, 5.0);
    assert_eq!(rows.len(), 1, "{rows:?}");
    let r = rows[0];
    assert!((al.mz[r] - X).abs() < 0.01, "reference {}", al.mz[r]);
    let members: Vec<f64> = al.members(r).map(|m| al.member_mz[m]).collect();
    assert!(
        members.iter().any(|m| (m - (X + 0.95)).abs() < 0.2),
        "{members:?}"
    );
}
