use super::*;
use std::cell::RefCell;
use std::rc::Rc;

fn file(n: usize) -> Vec<u8> {
    (0..n)
        .map(|i| (i.wrapping_mul(2654435761) >> 7) as u8)
        .collect()
}

/// The fetches `(offset, length)` a cache made.
type FetchLog = Rc<RefCell<Vec<(u64, usize)>>>;

/// A cache over `data` with the list of the fetches it made.
fn cache(data: Vec<u8>, size: usize, slots: usize) -> (impl ReadAt, FetchLog) {
    let log: FetchLog = Rc::default();
    let l = log.clone();
    let dim = data.len() as u64;
    let fetch = move |off: u64, buf: &mut [u8]| -> Result<()> {
        l.borrow_mut().push((off, buf.len()));
        let end = off as usize + buf.len();
        if end > data.len() {
            return Err(crate::Error::new("error.badIndex"));
        }
        buf.copy_from_slice(&data[off as usize..end]);
        Ok(())
    };
    (BlockCache::new(fetch, dim, size, slots), log)
}

#[test]
fn every_read_gives_the_bytes_of_the_file() {
    let data = file(100_000);
    for slots in [1, 2, 3] {
        let (mut c, _) = cache(data.clone(), 4096, slots);
        let mut state = 0x2545_f491_4f6c_dd1du64;
        for _ in 0..3000 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let len = (state % 9000) as usize + 1;
            let off = (state >> 20) % (data.len() as u64 - len as u64);
            let mut b = vec![0u8; len];
            c.read_at(off, &mut b).unwrap();
            assert_eq!(b, data[off as usize..off as usize + len]);
        }
    }
}

#[test]
fn a_read_past_the_end_is_refused_and_leaves_the_cache_sound() {
    let data = file(10_000);
    let (mut c, _) = cache(data.clone(), 4096, 2);
    let mut b = [0u8; 16];
    assert!(c.read_at(9_990, &mut b).is_err());
    c.read_at(100, &mut b).unwrap();
    assert_eq!(b, data[100..116]);
}

#[test]
fn two_streams_in_turn_cost_one_block_each_with_two_slots_and_a_block_per_read_with_one() {
    let data = file(1_000_000);
    let run = |slots: usize| {
        let (mut c, log) = cache(data.clone(), 65_536, slots);
        let mut b = [0u8; 8];
        for k in 0..200u64 {
            c.read_at(k * 100, &mut b).unwrap(); // the packets, forward
            c.read_at(900_000 + k * 50, &mut b).unwrap(); // the trailer, far away, forward
        }

        log.borrow().len()
    };
    assert_eq!(run(1), 400);
    assert_eq!(run(2), 2);
}

#[test]
fn a_range_that_wraps_past_u64_max_goes_to_the_fetch_and_is_refused() {
    let data = file(10_000);
    let dim = data.len() as u64;
    let fetch = move |off: u64, buf: &mut [u8]| -> Result<()> {
        match off.checked_add(buf.len() as u64) {
            Some(end) if end <= dim => {
                buf.copy_from_slice(&data[off as usize..end as usize]);
                Ok(())
            }
            _ => Err(crate::Error::new("error.badIndex")),
        }
    };
    let mut c = BlockCache::new(fetch, dim, 4096, 2);
    assert!(c.read_at(u64::MAX - 2, &mut [0u8; 16]).is_err());
    let mut b = [0u8; 16];
    c.read_at(100, &mut b).unwrap();
}

#[test]
fn a_slot_reused_for_a_shorter_or_longer_block_holds_only_the_new_bytes() {
    let data = file(10_000);
    let (mut c, log) = cache(data.clone(), 4096, 1);
    let mut b = [0u8; 8];
    for off in [9_995u64, 0, 9_000, 4_000, 9_995, 100] {
        c.read_at(off, &mut b[..5]).unwrap();
        assert_eq!(b[..5], data[off as usize..off as usize + 5]);
        // the next bytes of the same block come from the cache, not from the last block of the slot
        if off + 8 <= 10_000 {
            c.read_at(off, &mut b).unwrap();
            assert_eq!(b, data[off as usize..off as usize + 8]);
        }
    }
    assert_eq!(log.borrow().len(), 6);
}

/// A cache over `data` that grows its blocks up to `max` along reads in order, with the list of its fetches.
fn cache_ahead(data: Vec<u8>, size: usize, slots: usize, max: usize) -> (impl ReadAt, FetchLog) {
    let log: FetchLog = Rc::default();
    let l = log.clone();
    let dim = data.len() as u64;
    let fetch = move |off: u64, buf: &mut [u8]| -> Result<()> {
        l.borrow_mut().push((off, buf.len()));
        let end = off as usize + buf.len();
        if end > data.len() {
            return Err(crate::Error::new("error.badIndex"));
        }
        buf.copy_from_slice(&data[off as usize..end]);
        Ok(())
    };
    (
        BlockCache::new(fetch, dim, size, slots).read_ahead(max),
        log,
    )
}

#[test]
fn reads_in_order_double_the_block_up_to_the_cap() {
    let data = file(3_000_000);
    let (mut c, log) = cache_ahead(data.clone(), 65_536, 3, 1 << 20);
    let mut b = [0u8; 1000];
    for k in 0..3000u64 {
        c.read_at(k * 1000, &mut b).unwrap();
        assert_eq!(b, data[(k * 1000) as usize..(k * 1000 + 1000) as usize]);
    }
    let lens: Vec<usize> = log.borrow().iter().map(|&(_, n)| n).collect();
    assert_eq!(lens[..5], [65_536, 131_072, 262_144, 524_288, 1 << 20]);
    // the rest in blocks of the cap, the last one cut at the end of the file
    assert!(lens[5..lens.len() - 1].iter().all(|&n| n == 1 << 20));
    assert_eq!(lens.len(), 6);
}

#[test]
fn a_far_read_starts_again_from_the_small_block() {
    let data = file(4_000_000);
    let (mut c, log) = cache_ahead(data.clone(), 65_536, 1, 1 << 20);
    let mut b = [0u8; 8];
    for k in 0..40_000u64 {
        c.read_at(k * 8, &mut b).unwrap(); // 320 000 bytes in order
    }
    c.read_at(3_000_000, &mut b).unwrap(); // far away: a header, an index
    assert_eq!(b, data[3_000_000..3_000_008]);
    let lens: Vec<usize> = log.borrow().iter().map(|&(_, n)| n).collect();
    assert_eq!(lens, [65_536, 131_072, 262_144, 65_536]);
}

#[test]
fn two_runs_in_turn_each_grow_their_own_slot() {
    let data = file(4_000_000);
    let (mut c, log) = cache_ahead(data.clone(), 65_536, 2, 1 << 20);
    let mut b = [0u8; 8];
    for k in 0..100_000u64 {
        c.read_at(k * 8, &mut b).unwrap(); // the packets, forward
        assert_eq!(b, data[(k * 8) as usize..(k * 8 + 8) as usize]);
        c.read_at(2_000_000 + k * 8, &mut b).unwrap(); // the trailer, far away, forward
        assert_eq!(
            b,
            data[(2_000_000 + k * 8) as usize..(2_000_008 + k * 8) as usize]
        );
    }
    // 800 000 bytes of each run: blocks of 64, 128, 256 and 512 KiB, not twelve of 64 KiB
    let lens: Vec<usize> = log.borrow().iter().map(|&(_, n)| n).collect();
    assert_eq!(lens.len(), 8);
}

#[test]
fn every_read_gives_the_bytes_of_the_file_with_read_ahead() {
    let data = file(400_000);
    for slots in [1, 2, 3] {
        let (mut c, _) = cache_ahead(data.clone(), 4096, slots, 65_536);
        let mut state = 0x2545_f491_4f6c_dd1du64;
        let mut off = 0u64;
        for _ in 0..5000 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let len = (state % 9000) as usize + 1;
            // mostly in order with some jumps, as the reader of a `.raw`
            off = if state % 7 == 0 {
                (state >> 20) % (data.len() as u64 - len as u64)
            } else {
                (off + (state % 3000)).min(data.len() as u64 - len as u64)
            };
            let mut b = vec![0u8; len];
            c.read_at(off, &mut b).unwrap();
            assert_eq!(b, data[off as usize..off as usize + len]);
        }
    }
}

#[test]
fn reads_forward_with_gaps_shorter_than_the_block_grow_it_too() {
    let data = file(8_000_000);
    let (mut c, log) = cache_ahead(data.clone(), 65_536, 3, 1 << 20);
    let mut b = [0u8; 2000];
    // each scan: its header, then its centroids 50 KB further on (the profile between them is skipped)
    for k in 0..100u64 {
        for off in [k * 70_000, k * 70_000 + 50_000] {
            c.read_at(off, &mut b).unwrap();
            assert_eq!(b, data[off as usize..off as usize + 2000]);
        }
    }
    let lens: Vec<usize> = log.borrow().iter().map(|&(_, n)| n).collect();
    assert_eq!(lens[..5], [65_536, 131_072, 262_144, 524_288, 1 << 20]);
    assert!(lens.len() <= 11, "{lens:?}");
}
