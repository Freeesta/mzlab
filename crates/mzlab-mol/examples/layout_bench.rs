//! Times `Mol::layout_2d` over the SMILES in the second column of a TSV (header row skipped) and prints the median and p95 in
//! microseconds; with `--dump` it prints one JSON line per molecule (id, bonds, coordinates) instead.
//! Run: `cargo run --release -p mzlab-mol --example layout_bench -- tests/dati/draw_set_a.tsv`.

use std::time::Instant;

use mzlab_mol::Mol;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dump = args.iter().any(|a| a == "--dump");
    let Some(path) = args.iter().find(|a| !a.starts_with("--")) else {
        eprintln!("usage: layout_bench TSV [--dump]");
        std::process::exit(2);
    };
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("{path}: {e}");
        std::process::exit(4);
    });
    let mut micros = Vec::new();
    for line in text.lines().skip(1) {
        let mut cols = line.split('\t');
        let (Some(id), Some(smi)) = (cols.next(), cols.next()) else {
            continue;
        };
        let Ok(m) = Mol::from_smiles(smi) else {
            eprintln!("{id}: unreadable SMILES");
            continue;
        };
        let t = Instant::now();
        let c = m.layout_2d();
        micros.push(t.elapsed().as_secs_f64() * 1e6);
        if dump {
            let bonds: Vec<[u32; 3]> = m.bonds.iter().map(|b| [b.a, b.b, u32::from(b.o)]).collect();
            println!(
                "{}",
                serde_json::json!({"id": id, "bonds": bonds, "xy": c.xy, "n": m.atoms.len()})
            );
        }
    }
    if dump || micros.is_empty() {
        return;
    }
    micros.sort_by(|a, b| a.total_cmp(b));
    let p = |q: f64| micros[((micros.len() - 1) as f64 * q).round() as usize];
    println!(
        "n {} median_us {:.0} p95_us {:.0} max_us {:.0}",
        micros.len(),
        p(0.5),
        p(0.95),
        p(1.0)
    );
}
