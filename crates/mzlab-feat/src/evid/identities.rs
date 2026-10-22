//! The identity of every row of the aligned table, in the order of the rows: what `row_signs` reads and `sort_graph` orders. Native builds spread the
//! independent rows over threads; `evid::graph` and `run_evid` use it.

use super::graph::{Identity, Inputs};

/// The identity of every row of the aligned table, in the order of the rows (what `row_signs` reads and `sort_graph` orders).
pub fn identities(inp: &Inputs) -> Vec<Identity> {
    let prep = super::prepared::Prepared::new(inp);
    let n = inp.aligned.len();
    let one = |r: usize| prep.identity(inp, r);
    // rows are independent: native builds split them over threads and join the chunks in row order
    #[cfg(not(target_arch = "wasm32"))]
    {
        let threads = std::thread::available_parallelism()
            .map_or(1, |t| t.get())
            .min(8);
        if threads > 1 && n >= 256 {
            let size = n.div_ceil(threads);
            return std::thread::scope(|s| {
                let parts: Vec<_> = (0..n)
                    .step_by(size)
                    .map(|a| s.spawn(move || (a..(a + size).min(n)).map(one).collect::<Vec<_>>()))
                    .collect();
                parts
                    .into_iter()
                    .flat_map(|h| h.join().expect("identity thread"))
                    .collect()
            });
        }
    }
    (0..n).map(one).collect()
}
