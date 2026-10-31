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
