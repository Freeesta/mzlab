//! Runs blocks of work on the calling thread and on spare cores (none in the browser), for the passes that measure many independent
//! rows of one slab: `quality::assess_with`.

use std::sync::Mutex;

/// Runs `work(&mut state, k)` for the chunks `0..n`, on the calling thread and on spare cores (none in the browser), each worker with a
/// state from `make`; the outputs come back in chunk order whoever computed them.
pub(crate) fn chunked<W: Send, O: Send>(
    n: usize,
    make: impl Fn() -> W + Sync,
    work: impl Fn(&mut W, usize) -> O + Sync,
) -> Vec<O> {
    let mut out: Vec<Option<O>> = (0..n).map(|_| None).collect();
    #[cfg(not(target_family = "wasm"))]
    if n > 1 {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let lease = mzlab_core::spare::lease(n - 1);
        if lease.count() > 0 {
            let next = AtomicUsize::new(0);
            let slots = Mutex::new(&mut out);
            let run = || {
                let mut w = make();
                loop {
                    let k = next.fetch_add(1, Ordering::Relaxed);
                    if k >= n {
                        break;
                    }
                    let o = work(&mut w, k);
                    slots.lock().unwrap_or_else(|e| e.into_inner())[k] = Some(o);
                }
            };
            std::thread::scope(|s| {
                for _ in 0..lease.count() {
                    s.spawn(run);
                }
                run();
            });
        }
    }
    let mut w = None;
    for (k, slot) in out.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(work(w.get_or_insert_with(&make), k));
        }
    }
    out.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_come_back_in_order() {
        let got = chunked(
            37,
            || 0usize,
            |w, k| {
                *w += 1;
                k * 3
            },
        );
        assert_eq!(got, (0..37).map(|k| k * 3).collect::<Vec<_>>());
    }
}
