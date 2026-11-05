//! `Accorpa`: the destination ends with the same bytes as with direct writes, whatever the order (gaps, rewrites, reads of the scratch area).
use super::*;
use std::io::Cursor;

/// A destination that counts its calls and keeps the bytes.
#[derive(Default)]
struct Conta {
    c: Cursor<Vec<u8>>,
    calls: usize,
}
impl Output for Conta {
    fn write_at(&mut self, o: u64, d: &[u8]) -> Result<()> {
        self.calls += 1;
        self.c.write_at(o, d)
    }
}

fn writes() -> Vec<(u64, Vec<u8>)> {
    let mut v = Vec::new();
    let mut at = 0u64;
    let mut x = 7u32;
    for i in 0..400u32 {
        x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let n = (x >> 8) as usize % 3000 + 1;
        let d: Vec<u8> = (0..n).map(|k| (k as u32 ^ i ^ x) as u8).collect();
        // mostly appends; every so often a gap ahead or a rewrite of an earlier place (the header at the end of a build)
        let o = match i % 37 {
            5 => at + 999,
            11 => at / 3,
            _ => at,
        };
        at = at.max(o + n as u64);
        v.push((o, d));
    }
    v
}

#[test]
fn same_bytes_fewer_calls() {
    let mut direct = Conta::default();
    let mut a = Accorpa::with_block(Conta::default(), 64 << 10);
    for (o, d) in writes() {
        direct.write_at(o, &d).unwrap();
        a.write_at(o, &d).unwrap();
    }
    a.fine().unwrap();
    let a = a.into_inner();
    assert_eq!(a.c.get_ref(), direct.c.get_ref());
    assert!(
        a.calls * 5 < direct.calls,
        "{} calls against {}",
        a.calls,
        direct.calls
    );
}

#[test]
fn a_big_write_goes_through_and_fine_is_needed() {
    let mut a = Accorpa::with_block(Conta::default(), 16);
    a.write_at(0, &[1; 40]).unwrap();
    a.write_at(40, &[2; 3]).unwrap();
    assert_eq!(a.dest.calls, 1);
    a.fine().unwrap();
    a.fine().unwrap();
    let a = a.into_inner();
    assert_eq!(a.calls, 2);
    assert_eq!(a.c.get_ref().len(), 43);
}

#[test]
fn a_get_sees_the_pending_puts() {
    let mut s = Accorpa::new(Cursor::new(Vec::new()));
    s.put(0, &[1, 2, 3]).unwrap();
    s.put(3, &[4, 5]).unwrap();
    let mut b = [0u8; 4];
    s.get(1, &mut b).unwrap();
    assert_eq!(b, [2, 3, 4, 5]);
    s.put(2, &[9]).unwrap();
    s.get(0, &mut b).unwrap();
    assert_eq!(b, [1, 2, 9, 4]);
}

#[test]
fn scratch_puts_join_across_the_slack_of_a_slot() {
    let mut s = Accorpa::with_block(Cursor::new(Vec::new()), 1 << 20);
    s.put(0, &[7; 100]).unwrap();
    s.put(110, &[8; 10]).unwrap();
    assert!(
        s.dest.get_ref().is_empty(),
        "a gap of 10 bytes ends the block"
    );
    s.put(120 + BUCO + 1, &[9; 4]).unwrap();
    assert_eq!(
        s.dest.get_ref().len(),
        120,
        "a gap over a slot sends the block"
    );
    let mut b = [1u8; 12];
    s.get(98, &mut b).unwrap();
    assert_eq!(b, [7, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let mut o = Accorpa::with_block(Conta::default(), 1 << 20);
    o.write_at(0, &[1]).unwrap();
    o.write_at(2, &[1]).unwrap();
    assert_eq!(o.dest.calls, 1, "an output joins only what is contiguous");
}
