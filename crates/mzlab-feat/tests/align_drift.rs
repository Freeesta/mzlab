//! MEASURE (no default is changed): alignment of 20 files with a linear RT drift (+0.008 min per injection) where two isomers of the same m/z sit 0.14 min
//! apart (FWHM 0.06) and a third, minor one is in 5 files only. Rows, merged isomers and lost cells are pinned for `rt_warp` off/piecewise/loess,
//! `max_rt_span` and both modes (ion, family); see `docs/agenti/mzfeat-rt.md` (MISURA lines).

use mzlab_feat::align::AlignParams;
use mzlab_feat::famalign::{align_families, align_ions, FamAlignParams};
use mzlab_feat::rtwarp::{Method, WarpParams};
use mzlab_feat::table::{Aligned, FileFeatures, FLAG_GOOD, ROLE_M};

const FILES: usize = 20;
const DRIFT: f64 = 0.008;
const ISO_MZ: f64 = 300.1;
/// RT at injection 0 of the three isomers (the third is the minor one).
const ISO_RT: [f64; 3] = [5.0, 5.14, 5.28];
/// Anchors: strong isolated compounds that make the RT map possible.
const ANCHORS: usize = 40;

/// Ions of file `k` as (m/z, RT, area): the three isomers and the anchors, sorted by m/z.
fn ions(k: usize) -> Vec<(f64, f64, f64)> {
    let d = DRIFT * k as f64;
    let mut v: Vec<(f64, f64, f64)> = (0..ANCHORS)
        .map(|a| (200.0 + 7.3 * a as f64, 1.0 + 0.33 * a as f64 + d, 1e7))
        .collect();
    v.push((ISO_MZ, ISO_RT[0] + d, 1e7));
    v.push((ISO_MZ, ISO_RT[1] + d, 5e6));
    if k < 5 {
        v.push((ISO_MZ, ISO_RT[2] + d, 2e6));
    }
    v.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    v
}

fn file(k: usize) -> FileFeatures {
    let mut f = FileFeatures::default();
    for (i, (mz, rt, area)) in ions(k).into_iter().enumerate() {
        f.mz.push(mz);
        f.rt.push(rt as f32);
        f.left.push(rt as f32 - 0.05);
        f.right.push(rt as f32 + 0.05);
        f.area.push(area);
        f.height.push((area / 4.0) as f32);
        f.flags.push(FLAG_GOOD);
        f.role.push(ROLE_M);
        f.charge.push(1);
        f.family_id.push(i as u32 + 1);
    }
    f
}

/// (rows at the isomer m/z, detected cells in them, rows that hold only one isomer by the RT of their cells).
fn measure(al: &Aligned) -> (usize, usize, usize) {
    let rows = al.rows_near(ISO_MZ, 5.0);
    let mut cells = 0;
    let mut pure = 0;
    for &r in &rows {
        let mut kinds = [false; 3];
        for k in 0..FILES {
            let c = al.cell(r, k);
            if al.state[c] != mzlab_feat::table::CELL_DETECTED {
                continue;
            }
            cells += 1;
            let rt = f64::from(al.apex_rt[c]) - DRIFT * k as f64;
            let near = (0..3)
                .min_by(|&a, &b| (ISO_RT[a] - rt).abs().total_cmp(&(ISO_RT[b] - rt).abs()))
                .unwrap();
            kinds[near] = true;
        }
        pure += usize::from(kinds.iter().filter(|&&x| x).count() == 1);
    }
    (rows.len(), cells, pure)
}

fn run(
    family: bool,
    method: Method,
    span: Option<f64>,
    split: Option<f64>,
) -> (usize, usize, usize) {
    let files: Vec<FileFeatures> = (0..FILES).map(file).collect();
    let refs: Vec<&FileFeatures> = files.iter().collect();
    let p = FamAlignParams {
        warp: WarpParams {
            method,
            ..WarpParams::default()
        },
        ion: AlignParams {
            max_rt_span: span,
            split_gap: split,
            ..AlignParams::default()
        },
        ..FamAlignParams::default()
    };
    let al = if family {
        align_families(&refs, &p).unwrap()
    } else {
        align_ions(&refs, &p, &p.rt).unwrap()
    };
    measure(&al)
}

/// Pinned with the current defaults: (family mode, warp, max_rt_span, split_gap) -> (rows at the isomer m/z, detected cells, rows with one isomer).
/// 45 features exist (20 + 20 + 5); a merged row keeps only the largest per file, so 20 cells in one row means all three isomers fused.
#[test]
fn drift_measure() {
    let off = Method::Off;
    let cases = [
        (false, off, None, None, (1, 20, 1)),
        (false, Method::Piecewise, None, None, (3, 45, 3)),
        (false, Method::Loess, None, None, (3, 45, 3)),
        (false, off, Some(0.25), None, (8, 45, 6)),
        (false, off, None, Some(0.1), (1, 20, 1)),
        (true, off, None, None, (3, 45, 3)),
        (true, Method::Piecewise, None, None, (3, 45, 3)),
        (true, Method::Loess, None, None, (3, 45, 3)),
        (true, off, Some(0.25), None, (3, 45, 3)),
    ];
    for (family, method, span, split, want) in cases {
        let got = run(family, method, span, split);
        assert_eq!(
            got, want,
            "family={family} {method:?} span={span:?} split={split:?}"
        );
    }
}
