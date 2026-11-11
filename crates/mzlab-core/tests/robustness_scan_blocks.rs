//! A `.wiff.scan` block whose head claims more points, or an MRM block more transitions, than the memory holds: the reader answers `error.memoryBudget`
//! instead of a trap (the site) or an abort (the app). The scan file is virtual: 8 GB of zeros after one synthetic head, nothing is written to disk.

#![allow(clippy::disallowed_methods)] // the shared bench compares against the standard sorts

mod robustness;

use mzlab_core::error::Result;
use mzlab_core::idxread::ReadAt;
use mzlab_core::wiff::scan;
use robustness::Counter;

#[global_allocator]
static ALLOCATORE: Counter = Counter;

const LEN: u64 = 8 << 30;

/// A version-1 scan head (marker, version, one range, the point count) at offset 0 and zeros up to `LEN`.
struct Virtual(Vec<u8>);

impl ReadAt for Virtual {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        buf.fill(0);
        let head = &self.0;
        if let Ok(o) = usize::try_from(off) {
            for (i, b) in buf.iter_mut().enumerate() {
                if let Some(&h) = o.checked_add(i).and_then(|k| head.get(k)) {
                    *b = h;
                }
            }
        }
        Ok(())
    }
}

fn head(points: u32) -> Virtual {
    let mut h = scan::MARKER.to_vec();
    h.extend(1u32.to_le_bytes());
    [100.0f64, 1000.0, 0.1, 1.0]
        .iter()
        .for_each(|v| h.extend(v.to_le_bytes()));
    h.extend(points.to_le_bytes());
    Virtual(h)
}

#[test]
fn a_scan_block_claiming_4_billion_points_is_the_memory_error() {
    // 20 bytes per point of 2^32 points over a block of 8 GB: a request the allocator refuses
    let e = scan::read_scan(&mut head(u32::MAX), LEN, 0, LEN)
        .err()
        .unwrap();
    assert_eq!(e.key, "error.memoryBudget");
    // a small claim in the same block reads
    let p = scan::read_scan(&mut head(3), LEN, 0, LEN).unwrap().unwrap();
    assert!(p.mz.len() <= 3);
}

#[test]
fn an_mrm_block_of_2_gb_is_the_memory_error() {
    let e = scan::read_mrm(&mut head(0), LEN, 0, 1 << 29).err().unwrap();
    assert_eq!(e.key, "error.memoryBudget");
    assert_eq!(
        scan::read_mrm(&mut head(0), LEN, 0, 2)
            .unwrap()
            .map(|v| v.len()),
        Some(2)
    );
}
