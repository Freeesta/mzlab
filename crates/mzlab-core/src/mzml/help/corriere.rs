//! Decoding of the arrays of an mzML in other instances (the browser's helper workers, which share no memory with the parser): the parser hands
//! the arrays of each spectrum to a `Corriere` as bytes, a `Decodificatore` decodes them where the courier takes them, and the parser puts the
//! answers back in file order, so `parse_corriere` gives the same items as `parse`. A job that comes back empty is decoded on the parser.

use super::Decode;
use crate::error::Result;
use crate::mzml::{assemble, decode_peaks, finish_chromatogram, finish_spectrum, Arr, ArrayKind, Chrom, Item, Mobility, Peaks, Spec, Spectrum};
use crate::mzmlarray::Inflate;
use crate::numarray::Num;
use std::collections::VecDeque;

/// Carries jobs (`send`) to decoders elsewhere and brings back their answers (`take`), by sequence number.
pub trait Corriere {
    /// Jobs that may be out at once; 0 = no decoder, everything is decoded on the parser.
    fn window(&self) -> usize;
    /// Hands job `seq`, whose answer takes at most about `bound` bytes; false = nobody took it.
    fn send(&mut self, seq: u64, job: &[u8], bound: usize) -> bool;
    /// The answer of `seq`, waiting for it; `None` = lost or failed (the parser decodes the spectrum itself).
    fn take(&mut self, seq: u64) -> Option<Vec<u8>>;
}

/// The decoding side: one per helper instance, with its inflater reused by every job.
pub struct Decodificatore(Inflate);

impl Default for Decodificatore {
    fn default() -> Self {
        Decodificatore(Inflate::new())
    }
}

impl Decodificatore {
    /// The answer to `job`; `None` when it is not a job or does not decode (the parser then decodes it and tells the real error).
    pub fn decode(&mut self, job: &[u8]) -> Option<Vec<u8>> {
        let (len, arrays) = read_job(job)?;
        let p = decode_peaks(arrays.into_iter(), len, &mut self.0).ok()?;
        Some(write_peaks(&p))
    }
}

const KINDS: [ArrayKind; 3] = [ArrayKind::Mz, ArrayKind::Intensity, ArrayKind::Mobility];
const NUMS: [Num; 4] = [Num::F32, Num::F64, Num::I32, Num::I64];

/// `len` u64, then per array: kind, number type, zlib (one byte each), the length of the text (u32) and the text. Little endian.
fn write_job(s: &Spec) -> (Vec<u8>, usize) {
    let mut out = Vec::new();
    out.extend_from_slice(&(s.len as u64).to_le_bytes());
    let mut bound = 16;
    for a in s.arrays.iter().filter(|a| KINDS.contains(&a.kind)) {
        let k = KINDS.iter().position(|&k| k == a.kind).unwrap_or(0);
        let n = NUMS.iter().position(|&n| n == a.num).unwrap_or(0);
        out.extend_from_slice(&[k as u8, n as u8, u8::from(a.zlib)]);
        out.extend_from_slice(&(a.text.len() as u32).to_le_bytes());
        out.extend_from_slice(a.text.as_bytes());
        bound += 8 + s.len * if a.kind == ArrayKind::Intensity { 12 } else { 8 };
    }
    (out, bound)
}

fn read_job(job: &[u8]) -> Option<(usize, Vec<Arr<'_>>)> {
    let len = usize::try_from(u64::from_le_bytes(job.get(..8)?.try_into().ok()?)).ok()?;
    let (mut at, mut arrays) = (8, Vec::new());
    while at < job.len() {
        let h = job.get(at..at + 7)?;
        let n = u32::from_le_bytes(h.get(3..7)?.try_into().ok()?) as usize;
        let text = std::str::from_utf8(job.get(at + 7..at + 7 + n)?).ok()?;
        arrays.push(Arr {
            kind: *KINDS.get(usize::from(*h.first()?))?,
            num: *NUMS.get(usize::from(*h.get(1)?))?,
            zlib: *h.get(2)? == 1,
            text,
        });
        at += 7 + n;
    }
    Some((len, arrays))
}

/// The counts of mz, intensity, wide (or `u32::MAX` for none) and mobility (the same), then the values: m/z `f64`, intensity `f32`, wide and mobility `f64`.
fn write_peaks(p: &Peaks) -> Vec<u8> {
    let none = u32::MAX;
    let count = |v: Option<&Vec<f64>>| v.map_or(none, |v| v.len() as u32);
    let mut out = Vec::with_capacity(16 + 8 * p.mz.len() + 4 * p.it.len() + 8 * (p.wide.as_ref().map_or(0, Vec::len) + p.mob.as_ref().map_or(0, Vec::len)));
    for c in [p.mz.len() as u32, p.it.len() as u32, count(p.wide.as_ref()), count(p.mob.as_ref())] {
        out.extend_from_slice(&c.to_le_bytes());
    }
    p.mz.iter().for_each(|x| out.extend_from_slice(&x.to_le_bytes()));
    p.it.iter().for_each(|x| out.extend_from_slice(&x.to_le_bytes()));
    for v in [&p.wide, &p.mob].into_iter().flatten() {
        v.iter().for_each(|x| out.extend_from_slice(&x.to_le_bytes()));
    }
    out
}

fn read_peaks(b: &[u8]) -> Option<Peaks> {
    let c = |i: usize| b.get(4 * i..4 * i + 4).and_then(|s| s.try_into().ok()).map(u32::from_le_bytes);
    let (nm, ni, nw, nb) = (c(0)? as usize, c(1)? as usize, c(2)?, c(3)?);
    let mut at = 16usize;
    // the next `n` values of `w` bytes
    let mut take = |n: usize, w: usize| -> Option<&[u8]> {
        let s = b.get(at..at.checked_add(n.checked_mul(w)?)?)?;
        at += s.len();
        Some(s)
    };
    let f64s = |s: &[u8]| s.chunks_exact(8).map(|x| f64::from_le_bytes(x.try_into().unwrap_or_default())).collect::<Vec<f64>>();
    let mz = f64s(take(nm, 8)?);
    let it = take(ni, 4)?.chunks_exact(4).map(|x| f32::from_le_bytes(x.try_into().unwrap_or_default())).collect();
    let mut opt = |n: u32| if n == u32::MAX { Some(None) } else { take(n as usize, 8).map(|s| Some(f64s(s))) };
    let wide = opt(nw)?;
    let mob = opt(nb)?;
    (16 + 8 * nm + 4 * ni + 8 * (wide.as_ref().map_or(0, Vec::len) + mob.as_ref().map_or(0, Vec::len)) == b.len()).then_some(Peaks { mz, it, wide, mob })
}

enum Slot {
    /// Out with a decoder: the spectrum is kept to be assembled (or decoded here if the answer does not come).
    Out(u64, Spec),
    Ready(Result<Vec<Item>>),
}

/// The parser side: the jobs out and the queue that keeps the file order.
pub(crate) struct Sent<'a> {
    c: &'a mut dyn Corriere,
    inf: Inflate,
    queue: VecDeque<Slot>,
    next: u64,
    out: usize,
    /// An error has gone out: nothing after it is handed to `f`, as when one thread stops at the error.
    dead: bool,
}

fn items(r: Result<(Spectrum, Option<Mobility>)>) -> Result<Vec<Item>> {
    r.map(|(sp, m)| m.map(Item::Mobility).into_iter().chain([Item::Spectrum(sp)]).collect())
}

impl<'a> Sent<'a> {
    pub(crate) fn new(c: &'a mut dyn Corriere) -> Self {
        Sent { c, inf: Inflate::new(), queue: VecDeque::new(), next: 0, out: 0, dead: false }
    }

    /// The front job's answer (waiting for it), put in its place.
    fn resolve_front(&mut self) {
        if let Some(Slot::Out(seq, _)) = self.queue.front() {
            let got = self.c.take(*seq).and_then(|b| read_peaks(&b));
            if let Some(Slot::Out(_, s)) = self.queue.pop_front() {
                self.out -= 1;
                let r = match got {
                    Some(p) => Ok(assemble(s, p)),
                    None => finish_spectrum(s, &mut self.inf),
                };
                self.queue.push_front(Slot::Ready(items(r)));
            }
        }
    }

    /// Hands to `f` the items at the front of the queue that are ready.
    fn drain(&mut self, f: &mut dyn FnMut(Item)) -> Result<()> {
        while let Some(Slot::Ready(_)) = self.queue.front() {
            if let Some(Slot::Ready(r)) = self.queue.pop_front() {
                match r {
                    Ok(items) => items.into_iter().for_each(&mut *f),
                    Err(e) => {
                        self.dead = true;
                        self.queue.clear();
                        self.out = 0;
                        return Err(e);
                    }
                }
            }
        }
        Ok(())
    }

    fn push_ready(&mut self, r: Result<Vec<Item>>, f: &mut dyn FnMut(Item)) -> Result<()> {
        self.queue.push_back(Slot::Ready(r));
        self.drain(f)
    }
}

impl Decode for Sent<'_> {
    fn spectrum(&mut self, s: Spec, f: &mut dyn FnMut(Item)) -> Result<()> {
        let window = self.c.window();
        // a free place: the oldest job out is taken first (its slot is the one the new job reuses)
        while window > 0 && self.out >= window {
            self.resolve_front();
            self.drain(f)?;
        }
        // only the jobs that go out take a number: those out are always the last `out` numbers
        let seq = self.next;
        if window > 0 && {
            let (job, bound) = write_job(&s);
            self.c.send(seq, &job, bound)
        } {
            self.next += 1;
            self.queue.push_back(Slot::Out(seq, s));
            self.out += 1;
            return self.drain(f);
        }
        let r = items(finish_spectrum(s, &mut self.inf));
        self.push_ready(r, f)
    }

    fn chromatogram(&mut self, c: Chrom, f: &mut dyn FnMut(Item)) -> Result<()> {
        let r = finish_chromatogram(c, &mut self.inf).map(|c| vec![Item::Chromatogram(c)]);
        self.push_ready(r, f)
    }

    fn item(&mut self, it: Item, f: &mut dyn FnMut(Item)) -> Result<()> {
        self.push_ready(Ok(vec![it]), f)
    }

    fn finish(&mut self, f: &mut dyn FnMut(Item)) -> Result<()> {
        while !self.dead && !self.queue.is_empty() {
            self.resolve_front();
            self.drain(f)?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "corriere_tests.rs"]
mod tests;
