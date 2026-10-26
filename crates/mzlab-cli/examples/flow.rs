//! Runs a workflow file (`mzflow/2`, `docs/studi/workflow/contract.schema.json`) with `mzlab_cli::flow` and writes `prov.json` next to the outputs: SHA-256 of
//! every file, time and heap peak of every instance. The files of the plan are read from the folder of the plan file.
//! Native: `cargo run --release -p mzlab-cli --example flow -- FLOW.json WORKDIR [WORKERS]`; the same source builds for `wasm32-wasip1`.
use std::alloc::{GlobalAlloc, Layout, System};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use mzlab_cli::flow::{run_plan, Meter};

struct Counter;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
// SAFETY: forwards every call to the system allocator and only counts the bytes.
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = System.alloc(l);
        if !p.is_null() {
            let v = LIVE.fetch_add(l.size(), Ordering::Relaxed) + l.size();
            PEAK.fetch_max(v, Ordering::Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l);
        LIVE.fetch_sub(l.size(), Ordering::Relaxed);
    }
}
#[global_allocator]
static A: Counter = Counter;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 3 {
        eprintln!("usage: flow FLOW.json WORKDIR [WORKERS]");
        std::process::exit(2);
    }
    let (plan, work) = (Path::new(&a[1]), Path::new(&a[2]));
    let workers = a.get(3).and_then(|w| w.parse().ok()).unwrap_or(1);
    // the heap peak is the one of the whole process, so it is only exact with one worker
    let meter = Meter {
        reset: || PEAK.store(LIVE.load(Ordering::Relaxed), Ordering::Relaxed),
        peak: || PEAK.load(Ordering::Relaxed) as u64,
    };
    let result = std::fs::read(plan)
        .map_err(|e| mzlab_core::error::Error::new("err.flow.io").with("detail", e))
        .and_then(|text| {
            run_plan(
                &text,
                plan.parent().unwrap_or(Path::new(".")),
                work,
                workers,
                Some(meter),
            )
        });
    match result {
        Ok((_, prov)) => {
            let _ = std::fs::write(work.join("prov.json"), &prov);
            print!("{prov}");
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    }
}
