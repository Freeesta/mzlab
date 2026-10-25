use super::*;

fn row(grid: u8, lo: u32, hi: u32, apex: u32) -> Row {
    let rt = |s: u32| s as f64 * 0.02;
    Row {
        mz: 400.0,
        rt: rt(apex),
        n: hi - lo + 1,
        height: 1e6,
        area: 1.0,
        apex,
        lo,
        hi,
        left: rt(lo),
        right: rt(hi),
        long: false,
        broad: false,
        grid,
    }
}

// Same ion on both grids, apexes 0.4 min apart and edges that do not contain each other: the old test keeps both.
fn pair() -> Vec<Row> {
    vec![row(0, 100, 160, 110), row(1, 130, 190, 150)]
}

#[test]
fn overlapping_scans_merge_only_when_asked() {
    let kept = |overlap| {
        let keep = merge_grids(&pair(), MassAxis::Ppm, overlap).unwrap();
        keep.iter().filter(|k| **k).count()
    };
    assert_eq!(kept(0.0), 2);
    // 31 of the 61 scans of the shorter trace are shared
    assert_eq!(kept(0.5), 1);
    assert_eq!(kept(0.9), 2);
}
