//! Writes a synthetic DDA-like `.mzidx` of N scans (no real data) for the memory-limit measurements (`tests/memoria_limite_node.js`, `docs/agenti/memory-limits.md`).
//! usage: cargo run --release -p mzlab-core --example mem_idx -- OUT.mzidx SCANS
#[path = "../tests/mem_gen/mod.rs"]
mod mem_gen;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (Some(out), Some(n)) = (a.get(1), a.get(2).and_then(|n| n.parse::<usize>().ok())) else {
        eprintln!("usage: mem_idx OUT.mzidx SCANS");
        std::process::exit(2);
    };
    let bytes = mem_gen::mzidx(n, 9);
    std::fs::write(out, &bytes).expect("write");
    eprintln!("{out}: {} scans, {} bytes", n, bytes.len());
}
