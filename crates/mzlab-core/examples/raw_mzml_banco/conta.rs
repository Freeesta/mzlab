//! What the bench counts around a run: heap allocations (a counting global allocator), process CPU time, page faults and the peak resident set
//! (Linux `/proc/self`; the peak is reset before each run through `clear_refs`).
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

pub struct Conta;
static ALLOC: AtomicU64 = AtomicU64::new(0);
static LIBERE: AtomicU64 = AtomicU64::new(0);
static BYTE: AtomicU64 = AtomicU64::new(0);
static VIVI: AtomicUsize = AtomicUsize::new(0);
static PICCO: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every call forwards to `System` with the same layout; the counters are atomics.
unsafe impl GlobalAlloc for Conta {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOC.fetch_add(1, Ordering::Relaxed);
        BYTE.fetch_add(l.size() as u64, Ordering::Relaxed);
        let v = VIVI.fetch_add(l.size(), Ordering::Relaxed) + l.size();
        PICCO.fetch_max(v, Ordering::Relaxed);
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIBERE.fetch_add(1, Ordering::Relaxed);
        VIVI.fetch_sub(l.size(), Ordering::Relaxed);
        System.dealloc(p, l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        ALLOC.fetch_add(1, Ordering::Relaxed);
        BYTE.fetch_add(n as u64, Ordering::Relaxed);
        let v = (VIVI.fetch_add(n, Ordering::Relaxed) + n).saturating_sub(l.size());
        VIVI.fetch_sub(l.size(), Ordering::Relaxed);
        PICCO.fetch_max(v, Ordering::Relaxed);
        System.realloc(p, l, n)
    }
}

/// Allocation counters at one moment; subtract two to get the allocations between them.
#[derive(Clone, Copy, Default, Debug)]
pub struct Alloc {
    pub chiamate: u64,
    pub libere: u64,
    pub byte: u64,
}

impl Alloc {
    pub fn ora() -> Self {
        Alloc {
            chiamate: ALLOC.load(Ordering::Relaxed),
            libere: LIBERE.load(Ordering::Relaxed),
            byte: BYTE.load(Ordering::Relaxed),
        }
    }
    pub fn da(self, prima: Alloc) -> Alloc {
        Alloc {
            chiamate: self.chiamate - prima.chiamate,
            libere: self.libere - prima.libere,
            byte: self.byte - prima.byte,
        }
    }
}

impl std::ops::AddAssign for Alloc {
    fn add_assign(&mut self, o: Alloc) {
        self.chiamate += o.chiamate;
        self.libere += o.libere;
        self.byte += o.byte;
    }
}

/// Peak live heap in MB since the last call.
pub fn heap_picco_mb() -> f64 {
    PICCO.swap(VIVI.load(Ordering::Relaxed), Ordering::Relaxed) as f64 / 1048576.0
}

fn stat() -> Vec<String> {
    let s = std::fs::read_to_string("/proc/self/stat").unwrap_or_default();
    // the command name sits in parentheses and may hold spaces: the fields count from the closing one
    let dopo = s.rsplit_once(')').map_or("", |x| x.1);
    dopo.split_whitespace().map(str::to_string).collect()
}

/// User + system CPU seconds of the whole process, threads that ended included (10 ms resolution).
pub fn cpu_s() -> f64 {
    let f = stat();
    let t = |i: usize| f.get(i).and_then(|x| x.parse::<f64>().ok()).unwrap_or(0.0);
    (t(11) + t(12)) / 100.0
}

/// Minor and major page faults of the process.
pub fn guasti() -> (u64, u64) {
    let f = stat();
    let t = |i: usize| f.get(i).and_then(|x| x.parse::<u64>().ok()).unwrap_or(0);
    (t(7), t(9))
}

fn status_mb(chiave: &str) -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with(chiave))
                .and_then(|l| l.split_whitespace().nth(1)?.parse::<f64>().ok())
        })
        .map_or(f64::NAN, |kb| kb / 1024.0)
}

/// Peak resident set (MB) since `azzera_rss_picco`.
pub fn rss_picco_mb() -> f64 {
    status_mb("VmHWM:")
}

/// Sets the peak of the resident set back to the current one; false when the system refuses (the peak is then cumulative).
pub fn azzera_rss_picco() -> bool {
    std::fs::write("/proc/self/clear_refs", "5").is_ok()
}
