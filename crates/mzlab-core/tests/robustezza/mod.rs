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
use mzlab_core::lettori::{FileSet, Membro};

/// Counts, per thread, the bytes alive, the peak and the largest single request. A request over `TETTO_REALE` is refused (null), so that a hostile count cannot
/// bring the machine to its knees: the reader that handles the refusal is flagged all the same, because its count was never compared with the file.
pub struct Contatore;

const TETTO_REALE: usize = 256 << 20;

thread_local! {
    static VIVI: Cell<isize> = const { Cell::new(0) };
    static PICCO: Cell<isize> = const { Cell::new(0) };
    static GRANDE: Cell<usize> = const { Cell::new(0) };
    static RICHIESTA: Cell<u64> = const { Cell::new(0) };
}

fn conta(delta: isize, size: usize) {
    VIVI.with(|v| {
        v.set(v.get().saturating_add(delta));
        PICCO.with(|p| p.set(p.get().max(v.get())));
    });
    GRANDE.with(|g| g.set(g.get().max(size)));
}

unsafe impl GlobalAlloc for Contatore {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if l.size() > TETTO_REALE {
            conta(0, l.size());
            return std::ptr::null_mut();
        }
        let p = unsafe { System.alloc(l) };
        if !p.is_null() {
            conta(l.size() as isize, l.size());
        }
        p
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        if l.size() > TETTO_REALE {
            conta(0, l.size());
            return std::ptr::null_mut();
        }
        let p = unsafe { System.alloc_zeroed(l) };
        if !p.is_null() {
            conta(l.size() as isize, l.size());
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) };
        conta(-(l.size() as isize), 0);
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        if new > TETTO_REALE {
            conta(0, new);
            return std::ptr::null_mut();
        }
        let q = unsafe { System.realloc(p, l, new) };
        if !q.is_null() {
            conta(new as isize - l.size() as isize, new);
        }
        q
    }
}

/// A member of the set: the bytes of the case, with the largest request of the reader noted.
pub struct Letture(pub Vec<u8>);

impl ReadAt for Letture {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        RICHIESTA.with(|r| r.set(r.get().max(buf.len() as u64)));
        self.0.read_at(off, buf)
    }
}

/// The members of a case as the `FileSet` a reader receives.
pub fn fileset(membri: &[(String, Vec<u8>)]) -> FileSet {
    FileSet {
        membri: membri
            .iter()
            .map(|(nome, b)| Membro {
                nome: nome.clone(),
                dim: b.len() as u64,
                dati: Box::new(Letture(b.clone())),
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
    pub fn sotto(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

#[derive(Debug, Clone)]
pub enum Mutazione {
    Vuoto,
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
    Ripeti,
    TestaPoiCaso {
        testa: usize,
        lunghezza: usize,
        seme: u64,
    },
    NumeroGrande {
        da: usize,
        a: usize,
        val: &'static str,
    },
}

const GRANDI: [&str; 2] = ["4294967295", "18446744073709551615"];

pub fn applica(m: &Mutazione, s: &[u8]) -> Vec<u8> {
    match m {
        Mutazione::Vuoto => Vec::new(),
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
        Mutazione::Ripeti => s.repeat(3),
        Mutazione::TestaPoiCaso {
            testa,
            lunghezza,
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
pub fn mutazioni(s: &[u8], testo: bool) -> Vec<Mutazione> {
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    let n = s.len();
    let mut out = vec![
        Mutazione::Vuoto,
        Mutazione::UnByte(0),
        Mutazione::UnByte(0xFF),
        Mutazione::Ripeti,
    ];
    // truncation at the small offsets (every magic, every header) and at samples of the rest
    let mut tagli: Vec<usize> = (0..n.min(40))
        .chain([
            63, 64, 65, 127, 128, 255, 256, 511, 512, 513, 1023, 1024, 4095, 4096, 4097,
        ])
        .collect();
    tagli.extend([n.saturating_sub(1), n.saturating_sub(2), n / 2, n * 3 / 4]);
    tagli.extend((0..24).map(|_| r.sotto(n)));
    tagli.retain(|&t| t < n);
    tagli.sort_unstable();
    tagli.dedup();
    out.extend(tagli.into_iter().map(Mutazione::Tronca));
    // counts and lengths at their maximum
    if testo {
        let cifre = cifre(s);
        let passo = (cifre.len() / 60).max(1);
        for &(da, a) in cifre.iter().step_by(passo) {
            out.extend(
                GRANDI
                    .iter()
                    .map(|&val| Mutazione::NumeroGrande { da, a, val }),
            );
        }
    } else {
        let fitti = (0..n.min(1024)).step_by(4);
        let sparsi: Vec<usize> = (0..128).map(|_| r.sotto(n) & !3).collect();
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
        .chain((0..40).map(|_| r.sotto(n)))
    {
        out.push(Mutazione::Zeri { at, n: 16 });
    }
    // flipped bytes: single and in groups
    for k in 0..160 {
        let cambi = (0..if k < 100 { 1 } else { 8 })
            .map(|_| (r.sotto(n), r.next() as u8))
            .collect();
        out.push(Mutazione::Cambia(cambi));
    }
    // a valid head and a random body
    for (testa, lunghezza) in [(64, n), (512, n), (512, 4 * n), (n / 4, n)] {
        out.push(Mutazione::TestaPoiCaso {
            testa,
            lunghezza,
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
pub struct Esito {
    pub chiavi: Vec<String>,
    pub picco: usize,
    pub grande: usize,
    pub richiesta: u64,
}

/// The keys of the errors are `err.*` (the older ones `error.*`).
pub fn chiave_ok(k: &str) -> bool {
    k.starts_with("err.") || k.starts_with("error.")
}

/// Notes the key of an error in `chiavi` and gives back the value of an `Ok`.
pub fn nota<T>(r: Result<T>, chiavi: &mut Vec<String>) -> Option<T> {
    r.map_err(|e: Error| chiavi.push(e.key.to_string())).ok()
}

/// The code under test: it receives the members of a case and gives the keys of the errors it met.
pub type Esegui = fn(&[(String, Vec<u8>)]) -> Vec<String>;

fn un_caso(job: Box<dyn FnOnce() -> Vec<String> + Send>) -> Esito {
    VIVI.with(|v| v.set(0));
    PICCO.with(|v| v.set(0));
    GRANDE.with(|v| v.set(0));
    RICHIESTA.with(|v| v.set(0));
    let chiavi = job();
    Esito {
        chiavi,
        picco: PICCO.with(Cell::get).max(0) as usize,
        grande: GRANDE.with(Cell::get),
        richiesta: RICHIESTA.with(Cell::get),
    }
}

/// The ceiling of memory for a case: a fixed part and a part proportional to the bytes of the input.
pub fn tetto(totale: usize) -> usize {
    (8 << 20) + 64 * totale
}

/// Runs every mutation of every member of `seme` (the other members unchanged) and returns the failures, one line each.
pub fn corri(seme: &[(String, Vec<u8>)], testo: &[bool], esegui: Esegui) -> (usize, Vec<String>) {
    let (mut casi, mut guasti) = (0, Vec::new());
    let tracciato = std::env::var_os("ROBUSTEZZA_TRACCIA").is_some();
    for (i, (nome, base)) in seme.iter().enumerate() {
        for m in mutazioni(base, testo[i]) {
            let mut membri = seme.to_vec();
            membri[i].1 = applica(&m, base);
            let totale: usize = membri.iter().map(|x| x.1.len()).sum();
            let etichetta = format!("{nome} {m:?}");
            if tracciato {
                eprintln!("caso {etichetta}");
            }
            casi += 1;
            guasti.extend(uno(&etichetta, totale, Box::new(move || esegui(&membri))));
        }
    }
    (casi, guasti)
}

/// One case in a thread of its own: panic, hang (20 s), keys, memory and reads are judged against `totale`, the bytes of the input.
pub fn uno(
    etichetta: &str,
    totale: usize,
    job: Box<dyn FnOnce() -> Vec<String> + Send>,
) -> Vec<String> {
    let (tx, rx) = mpsc::channel();
    let h = std::thread::Builder::new()
        .stack_size(8 << 20)
        .spawn(move || {
            let _ = tx.send(un_caso(job));
        })
        .expect("thread");
    match rx.recv_timeout(Duration::from_secs(20)) {
        Err(mpsc::RecvTimeoutError::Timeout) => vec![format!("{etichetta}: no answer in 20 s")],
        Err(_) => vec![format!("{etichetta}: panic {}", panico(h))],
        Ok(e) => {
            let _ = h.join();
            giudica(etichetta, &e, totale)
        }
    }
}

fn panico(h: std::thread::JoinHandle<()>) -> String {
    match h.join() {
        Err(p) => p
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default(),
        Ok(()) => String::new(),
    }
}

fn giudica(etichetta: &str, e: &Esito, totale: usize) -> Vec<String> {
    let mut out = Vec::new();
    let t = tetto(totale);
    for k in e.chiavi.iter().filter(|k| !chiave_ok(k)) {
        out.push(format!("{etichetta}: key without err.* prefix: {k}"));
    }
    if e.picco > t || e.grande > t {
        out.push(format!(
            "{etichetta}: memory peak {} B, largest request {} B, ceiling {t} B",
            e.picco, e.grande
        ));
    }
    if e.richiesta > totale as u64 {
        out.push(format!(
            "{etichetta}: read of {} B from a member of at most {totale} B",
            e.richiesta
        ));
    }
    out
}

/// The test passes when no case failed; the failures are listed (the first 25).
pub fn verifica(nome: &str, casi: usize, guasti: &[String]) {
    eprintln!("{nome}: {casi} cases, {} failures", guasti.len());
    assert!(
        guasti.is_empty(),
        "{nome}: {} failures in {casi} cases\n{}",
        guasti.len(),
        guasti
            .iter()
            .take(25)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
