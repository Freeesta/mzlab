//! The bench of robustness shared by the tests `robustezza_*.rs`: a damaged, truncated or hostile file gives an `err.*` key, never a panic, a hang or an allocation
//! that the size of the file does not explain. The inputs are a valid seed and deterministic mutations of it (fixed seed of the generator, no dependency,
//! no data of the laboratory); every case runs in a thread of its own, so that a panic, a hang and the peak of memory are those of that case alone.

#![allow(dead_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::mpsc;
use std::time::Duration;

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::readers::{FileSet, Member};

/// Counts, per thread, the bytes alive, the peak and the largest single request. A request over `REAL_CAP` is refused (null), so that a hostile count cannot
/// bring the machine to its knees: the reader that handles the refusal is flagged all the same, because its count was never compared with the file.
pub struct Counter;

const REAL_CAP: usize = 256 << 20;

thread_local! {
    static VIVI: Cell<isize> = const { Cell::new(0) };
    static PEAK: Cell<isize> = const { Cell::new(0) };
    static LARGE: Cell<usize> = const { Cell::new(0) };
    static REQUEST: Cell<u64> = const { Cell::new(0) };
}

fn count(delta: isize, size: usize) {
    VIVI.with(|v| {
        v.set(v.get().saturating_add(delta));
        PEAK.with(|p| p.set(p.get().max(v.get())));
    });
    LARGE.with(|g| g.set(g.get().max(size)));
}

unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if l.size() > REAL_CAP {
            count(0, l.size());
            return std::ptr::null_mut();
        }
        let p = unsafe { System.alloc(l) };
        if !p.is_null() {
            count(l.size() as isize, l.size());
        }
        p
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        if l.size() > REAL_CAP {
            count(0, l.size());
            return std::ptr::null_mut();
        }
        let p = unsafe { System.alloc_zeroed(l) };
        if !p.is_null() {
            count(l.size() as isize, l.size());
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) };
        count(-(l.size() as isize), 0);
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        if new > REAL_CAP {
            count(0, new);
            return std::ptr::null_mut();
        }
        let q = unsafe { System.realloc(p, l, new) };
        if !q.is_null() {
            count(new as isize - l.size() as isize, new);
        }
        q
    }
}

/// A member of the set: the bytes of the case, with the largest request of the reader noted.
pub struct Letture(pub Vec<u8>);

impl ReadAt for Letture {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        REQUEST.with(|r| r.set(r.get().max(buf.len() as u64)));
        self.0.read_at(off, buf)
    }
}

/// The members of a case as the `FileSet` a reader receives.
pub fn fileset(members: &[(String, Vec<u8>)]) -> FileSet {
    FileSet {
        membri: members
            .iter()
            .map(|(name, b)| Member {
                nome: name.clone(),
                dim: b.len() as u64,
                dati: Box::new(Letture(b.clone())),
                sha1: None,
            })
            .collect(),
    }
}

/// Deterministic generator (xorshift64*), fixed seed.
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

#[derive(Debug, Clone)]
pub enum Mutazione {
    Empty,
    UnByte(u8),
    Tronca(usize),
    Stampa {
        at: usize,
        val: Vec<u8>,
    },
    Zeri {
        at: usize,
        n: usize,
    },
    Cambia(Vec<(usize, u8)>),
    Repeat,
    TestaPoiCaso {
        testa: usize,
        length: usize,
        seme: u64,
    },
    NumeroGrande {
        da: usize,
        a: usize,
        val: &'static str,
    },
}

const LARGE_ONES: [&str; 2] = ["4294967295", "18446744073709551615"];

pub fn apply(m: &Mutazione, s: &[u8]) -> Vec<u8> {
    match m {
        Mutazione::Empty => Vec::new(),
        Mutazione::UnByte(b) => vec![*b],
        Mutazione::Tronca(n) => s[..(*n).min(s.len())].to_vec(),
        Mutazione::Stampa { at, val } => {
            let mut v = s.to_vec();
            let fine = (*at + val.len()).min(v.len());
            v[*at..fine].copy_from_slice(&val[..fine - *at]);
            v
        }
        Mutazione::Zeri { at, n } => {
            let mut v = s.to_vec();
            let fine = (*at + *n).min(v.len());
            v[*at..fine].fill(0);
            v
        }
        Mutazione::Cambia(c) => {
            let mut v = s.to_vec();
            c.iter().for_each(|&(i, b)| v[i] = b);
            v
        }
        Mutazione::Repeat => s.repeat(3),
        Mutazione::TestaPoiCaso {
            testa,
            length: lunghezza,
            seme,
        } => {
            let mut v = s[..(*testa).min(s.len())].to_vec();
            let mut r = Rng(*seme);
            while v.len() < *lunghezza {
                v.push(r.next() as u8);
            }
            v
        }
        Mutazione::NumeroGrande { da, a, val } => {
            let mut v = s[..*da].to_vec();
            v.extend_from_slice(val.as_bytes());
            v.extend_from_slice(&s[*a..]);
            v
        }
    }
}

/// The mutations of a seed. `testo`: an XML or JSON text, where the counts are numbers written in decimal; otherwise a binary format, where they are
/// little-endian integers.
pub fn mutazioni(s: &[u8], text: bool) -> Vec<Mutazione> {
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    let n = s.len();
    let mut out = vec![
        Mutazione::Empty,
        Mutazione::UnByte(0),
        Mutazione::UnByte(0xFF),
        Mutazione::Repeat,
    ];
    // truncation at the small offsets (every magic, every header) and at samples of the rest
    let mut cuts: Vec<usize> = (0..n.min(40))
        .chain([
            63, 64, 65, 127, 128, 255, 256, 511, 512, 513, 1023, 1024, 4095, 4096, 4097,
        ])
        .collect();
    cuts.extend([n.saturating_sub(1), n.saturating_sub(2), n / 2, n * 3 / 4]);
    cuts.extend((0..24).map(|_| r.below(n)));
    cuts.retain(|&t| t < n);
    cuts.sort_unstable();
    cuts.dedup();
    out.extend(cuts.into_iter().map(Mutazione::Tronca));
    // counts and lengths at their maximum
    if text {
        let digits = cifre(s);
        let step = (digits.len() / 60).max(1);
        for &(da, a) in digits.iter().step_by(step) {
            out.extend(
                LARGE_ONES
                    .iter()
                    .map(|&val| Mutazione::NumeroGrande { da, a, val }),
            );
        }
    } else {
        let fitti = (0..n.min(1024)).step_by(4);
        let sparsi: Vec<usize> = (0..128).map(|_| r.below(n) & !3).collect();
        for at in fitti.chain(sparsi) {
            out.push(Mutazione::Stampa {
                at,
                val: u32::MAX.to_le_bytes().to_vec(),
            });
            out.push(Mutazione::Stampa {
                at,
                val: u64::MAX.to_le_bytes().to_vec(),
            });
            out.push(Mutazione::Stampa {
                at,
                val: i32::MIN.to_le_bytes().to_vec(),
            });
        }
    }
    // zeros
    for at in (0..n.min(128))
        .step_by(4)
        .chain((0..40).map(|_| r.below(n)))
    {
        out.push(Mutazione::Zeri { at, n: 16 });
    }
    // flipped bytes: single and in groups
    for k in 0..160 {
        let changes = (0..if k < 100 { 1 } else { 8 })
            .map(|_| (r.below(n), r.next() as u8))
            .collect();
        out.push(Mutazione::Cambia(changes));
    }
    // a valid head and a random body
    for (testa, lunghezza) in [(64, n), (512, n), (512, 4 * n), (n / 4, n)] {
        out.push(Mutazione::TestaPoiCaso {
            testa,
            length: lunghezza,
            seme: r.next(),
        });
    }
    out
}

/// The runs of digits of a text, as `(start, end)`.
fn cifre(s: &[u8]) -> Vec<(usize, usize)> {
    let mut v = Vec::new();
    let mut i = 0;
    while i < s.len() {
        if s[i].is_ascii_digit() {
            let da = i;
            while i < s.len() && s[i].is_ascii_digit() {
                i += 1;
            }
            v.push((da, i));
        } else {
            i += 1;
        }
    }
    v
}

/// What a case did: the keys of the errors it met, the peak and the largest request of memory, the largest read asked of a member.
pub struct Outcome {
    pub keys: Vec<String>,
    pub peak: usize,
    pub large: usize,
    pub request: u64,
}

/// The keys of the errors are `err.*` (the older ones `error.*`).
pub fn key_ok(k: &str) -> bool {
    k.starts_with("err.") || k.starts_with("error.")
}

/// Notes the key of an error in `chiavi` and gives back the value of an `Ok`.
pub fn note<T>(r: Result<T>, keys: &mut Vec<String>) -> Option<T> {
    r.map_err(|e: Error| keys.push(e.key.to_string())).ok()
}

/// The code under test: it receives the members of a case and gives the keys of the errors it met.
pub type Execute = fn(&[(String, Vec<u8>)]) -> Vec<String>;

fn one_case(job: Box<dyn FnOnce() -> Vec<String> + Send>) -> Outcome {
    VIVI.with(|v| v.set(0));
    PEAK.with(|v| v.set(0));
    LARGE.with(|v| v.set(0));
    REQUEST.with(|v| v.set(0));
    let chiavi = job();
    Outcome {
        keys: chiavi,
        peak: PEAK.with(Cell::get).max(0) as usize,
        large: LARGE.with(Cell::get),
        request: REQUEST.with(Cell::get),
    }
}

/// The ceiling of memory for a case: a fixed part and a part proportional to the bytes of the input.
pub fn cap(total: usize) -> usize {
    (8 << 20) + 64 * total
}

/// Runs every mutation of every member of `seme` (the other members unchanged) and returns the failures, one line each.
pub fn corri(seme: &[(String, Vec<u8>)], text: &[bool], execute: Execute) -> (usize, Vec<String>) {
    let (mut casi, mut guasti) = (0, Vec::new());
    let tracciato = std::env::var_os("ROBUSTEZZA_TRACCIA").is_some();
    for (i, (nome, base)) in seme.iter().enumerate() {
        for m in mutazioni(base, text[i]) {
            let mut members = seme.to_vec();
            members[i].1 = apply(&m, base);
            let total: usize = members.iter().map(|x| x.1.len()).sum();
            let label = format!("{nome} {m:?}");
            if tracciato {
                eprintln!("caso {label}");
            }
            casi += 1;
            guasti.extend(one(&label, total, Box::new(move || execute(&members))));
        }
    }
    (casi, guasti)
}

/// One case in a thread of its own: panic, hang (20 s), keys, memory and reads are judged against `totale`, the bytes of the input.
pub fn one(label: &str, total: usize, job: Box<dyn FnOnce() -> Vec<String> + Send>) -> Vec<String> {
    let (tx, rx) = mpsc::channel();
    let h = std::thread::Builder::new()
        .stack_size(8 << 20)
        .spawn(move || {
            let _ = tx.send(one_case(job));
        })
        .expect("thread");
    match rx.recv_timeout(Duration::from_secs(20)) {
        Err(mpsc::RecvTimeoutError::Timeout) => vec![format!("{label}: no answer in 20 s")],
        Err(_) => vec![format!("{label}: panic {}", panic_case(h))],
        Ok(e) => {
            let _ = h.join();
            judge(label, &e, total)
        }
    }
}

fn panic_case(h: std::thread::JoinHandle<()>) -> String {
    match h.join() {
        Err(p) => p
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default(),
        Ok(()) => String::new(),
    }
}

fn judge(label: &str, e: &Outcome, total: usize) -> Vec<String> {
    let mut out = Vec::new();
    let t = cap(total);
    for k in e.keys.iter().filter(|k| !key_ok(k)) {
        out.push(format!("{label}: key without err.* prefix: {k}"));
    }
    if e.peak > t || e.large > t {
        out.push(format!(
            "{label}: memory peak {} B, largest request {} B, ceiling {t} B",
            e.peak, e.large
        ));
    }
    if e.request > total as u64 {
        out.push(format!(
            "{label}: read of {} B from a member of at most {total} B",
            e.request
        ));
    }
    out
}

/// The test passes when no case failed; the failures are listed (the first 25).
pub fn verify(name: &str, casi: usize, guasti: &[String]) {
    eprintln!("{name}: {casi} cases, {} failures", guasti.len());
    assert!(
        guasti.is_empty(),
        "{name}: {} failures in {casi} cases\n{}",
        guasti.len(),
        guasti
            .iter()
            .take(25)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
