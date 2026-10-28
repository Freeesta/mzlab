// Timing and output fingerprint of `composition::compose`: 20 typical masses on the default element ranges and on a ~2.1·10^6 cell grid.
// Run with `cargo run --release -p mzlab-core --example lotto9_composition`; the hashes must not change between versions.

use std::time::Instant;

use mzlab_core::composition::{compose, parse_elements, Opts};

fn fnv(h: &mut u64, s: &str) {
    for b in s.bytes() {
        *h = (*h ^ b as u64).wrapping_mul(0x100000001b3);
    }
}

fn main() {
    let masses = [
        116.0706, 138.0662, 152.0706, 180.0634, 194.0802, 232.1801, 256.0922, 285.1359, 301.1410,
        313.1313, 328.1523, 341.1760, 365.1052, 394.2013, 413.2662, 446.1747, 478.2900, 520.3400,
        609.2807, 784.5000,
    ];
    let wide =
        parse_elements("N:0-10,O:0-15,S:0-3,P:0-3,F:0-6,Cl:0-3,Br:0-2,Na:0-2,K:0-2").unwrap();
    for (name, elements, max_results) in [
        ("default ranges", None, 10usize),
        ("2.1M grid", Some(wide), 10),
        ("2.1M grid, all candidates", None, 5000),
    ] {
        let (mut h, mut total_ms) = (0xcbf29ce484222325u64, 0.0);
        for &mz in &masses {
            for ion in ["[M+H]+", "[M+Na]+"] {
                let o = Opts {
                    elements: if name == "default ranges" {
                        None
                    } else {
                        elements.clone().or_else(|| {
                            parse_elements(
                                "N:0-10,O:0-15,S:0-3,P:0-3,F:0-6,Cl:0-3,Br:0-2,Na:0-2,K:0-2",
                            )
                            .ok()
                        })
                    },
                    ion,
                    max_results,
                    rules: if max_results > 10 {
                        vec![]
                    } else {
                        vec!["HC".into(), "LEWIS".into()]
                    },
                    ..Opts::default()
                };
                let t = Instant::now();
                let out = compose(mz, &o).unwrap_or_else(|e| format!("err {}", e.key));
                total_ms += t.elapsed().as_secs_f64() * 1e3;
                fnv(&mut h, &out);
            }
        }
        println!("{name:<28} {total_ms:>9.1} ms  hash {h:016x}");
    }
}
