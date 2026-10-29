// Timing of the target CSV parser on 10^5 rows (the test list repeated); prints a fingerprint of the entries.
// Run with `cargo run --release -p mzlab-target --example bench_csv`.

use std::time::Instant;

use mzlab_target::parse_target_csv;

fn main() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/data/liste/sostanze_hrms.csv"
    );
    let text = std::fs::read_to_string(path).unwrap();
    let (head, body) = text.split_once('\n').unwrap();
    let mut big = String::with_capacity(body.len() * 160);
    big.push_str(head);
    big.push('\n');
    while big.matches('\n').count() < 100_001 {
        big.push_str(body);
        if !big.ends_with('\n') {
            big.push('\n');
        }
    }
    let t = Instant::now();
    let e = parse_target_csv(big.as_bytes()).unwrap();
    let el = t.elapsed();
    let mut h: u64 = 0xcbf29ce484222325;
    for x in &e {
        for b in format!("{x:?}").bytes() {
            h = (h ^ b as u64).wrapping_mul(0x100000001b3);
        }
    }
    println!(
        "rows {} hash {h:016x} time {:.0} ms ({} bytes)",
        e.len(),
        el.as_secs_f64() * 1e3,
        big.len()
    );
}
