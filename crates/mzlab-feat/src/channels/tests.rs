//! Tests of `channels`: the bounded reach of `link` against the full one.

use super::*;

/// `link` as it was before the reach of a segment was bounded: every segment of the next channel is tried.
fn link_all(segs: &[Seg], p: &Params, parent: &mut [u32]) {
    let max_gap = p.max_gap as i64;
    for i in 0..segs.len() {
        let c = segs[i].chan;
        let lo = segs.partition_point(|s| s.chan < c + 1);
        let hi = segs.partition_point(|s| s.chan <= c + 1);
        for j in lo..hi {
            let (si, sj) = (&segs[i], &segs[j]);
            let ov = sj.z.min(si.z) as i64 - sj.a.max(si.a) as i64 + 1;
            let short = (si.z as i64 - si.a as i64 + 1).min(sj.z as i64 - sj.a as i64 + 1);
            let (hi_mx, lo_mx) = (si.mx.max(sj.mx), si.mx.min(sj.mx));
            let like = (si.mzc - sj.mzc).abs() <= 4e-6 * si.mzc
                && hi_mx <= 10.0 * lo_mx
                && lo_mx >= TOUCH_HEIGHT;
            let overlap = ov as f64 >= 0.5 * short as f64;
            if overlap || (ov >= 1 - max_gap && ov <= max_gap + 1 && like) {
                let (ra, rb) = (find(parent, i as u32), find(parent, j as u32));
                if ra != rb {
                    parent[ra.max(rb) as usize] = ra.min(rb);
                }
            }
        }
    }
}

#[test]
fn the_bounded_reach_links_the_segments_the_full_one_links() {
    let p = Params::default();
    let mut seed = 5u64;
    let mut r = |n: u64| {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) % n
    };
    for _ in 0..200 {
        let mut segs = Vec::new();
        for chan in 0..6i32 {
            // the runs of a channel come in scan order and apart, as `segments` makes them
            let mut at = r(30) as u32;
            for _ in 0..r(12) {
                let (a, z) = (at, at + 1 + r(25) as u32);
                let mx = 1e5 * (1 + r(40)) as f64;
                segs.push(Seg {
                    chan,
                    mzc: 300.0 + 1e-6 * r(8) as f64 * 300.0,
                    apex: a,
                    n: z - a + 1,
                    mx,
                    tot: 3.0 * mx,
                    a,
                    z,
                    broad: false,
                });
                at = z + p.max_gap + 1 + r(20) as u32;
            }
        }
        let mut want: Vec<u32> = (0..segs.len() as u32).collect();
        link_all(&segs, &p, &mut want);
        let mut got: Vec<u32> = (0..segs.len() as u32).collect();
        link(&segs, &p, &mut got);
        for i in 0..segs.len() as u32 {
            assert_eq!(find(&mut got, i), find(&mut want, i));
        }
    }
}

/// A segment stays at 48 bytes: the peak memory of detection on the wasm is guarded in `tests/test_guardie_memoria.py`.
#[test]
fn a_segment_is_48_bytes() {
    assert_eq!(std::mem::size_of::<Seg>(), 48);
}
