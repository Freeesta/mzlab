//! The inputs of a plan before any step runs: each file hashed (SHA-256) and, for scan files, its facets (polarity, resolution) read.
//! Files are independent, so they are read on several threads; the results are applied in plan order, so the plan, the first error
//! and every output are the same as one file at a time.

use std::path::{Path, PathBuf};

use mzlab_core::flow::Plan;
use mzlab_core::Result;

/// Threads for reading the inputs: one per file, up to every core of the machine (no step runs while the inputs are hashed, and with one core
/// held back 4 files on 4 cores took two rounds; the desktop window is another process the system still schedules); one file is read in place,
/// and 1 where the platform gives no count of its cores (WASI). At most `MAX_READERS`: past a few readers one disk only seeks between the files and
/// each reader holds its own buffers.
fn threads(files: usize) -> usize {
    std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(files)
        .clamp(1, MAX_READERS)
}

/// Upper bound of the input readers, whatever the cores and the files.
const MAX_READERS: usize = 8;

/// `f` on every element of `items` over a few threads; the results come back in the order of `items`.
pub(crate) fn map_par<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let n = threads(items.len());
    if n <= 1 {
        // one core, or WASI (`flow.wasm`), where a thread cannot be spawned: in place
        return items.iter().map(f).collect();
    }
    let next = AtomicUsize::new(0);
    let mut out: Vec<(usize, R)> = std::thread::scope(|s| {
        let work = || {
            let mut got = Vec::new();
            loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(x) = items.get(i) else { return got };
                got.push((i, f(x)));
            }
        };
        let hands: Vec<_> = (0..n).map(|_| s.spawn(work)).collect();
        hands
            .into_iter()
            .flat_map(|h| h.join().expect("input reader"))
            .collect()
    });
    out.sort_unstable_by_key(|(i, _)| *i);
    out.into_iter().map(|(_, r)| r).collect()
}

/// `flow_set::digest_for` of every path given (`None` for the inputs not read), over a few threads, in their order.
pub(crate) fn digest_all(
    paths: &[Option<PathBuf>],
    sha1: bool,
) -> Vec<Option<Result<(u64, String)>>> {
    map_par(paths, |p| {
        p.as_deref().map(|p| crate::flow_set::digest_for(p, sha1))
    })
}

/// The inputs of the plan that name no hash get it from the file `locate` gives (one that names a hash must match it), and the scan inputs the facets
/// their file gives where the plan writes none (`mzlab_feat::nature::fill_facets`).
pub fn fill_inputs(plan: &mut Plan, locate: impl Fn(&str) -> PathBuf + Sync) -> Result<()> {
    let wants: Vec<bool> = plan
        .inputs
        .iter()
        .map(mzlab_feat::nature::wants_facets)
        .collect();
    let jobs: Vec<(PathBuf, bool)> = plan
        .inputs
        .iter()
        .zip(&wants)
        .map(|(f, w)| (locate(&f.name), *w))
        .collect();
    let sha1 = crate::flow_set::wants_sha1(plan);
    let read = map_par(&jobs, |(p, w)| {
        (
            crate::flow_set::digest_for(p, sha1),
            if *w { scan_facts(p) } else { None },
        )
    });
    let mut facts = Vec::with_capacity(read.len());
    for (f, (dig, fa)) in plan.inputs.iter_mut().zip(read) {
        let (bytes, sha) = dig.map_err(|e| e.with("input", &f.name))?;
        if !f.sha256.is_empty() && f.sha256 != sha {
            return Err(mzlab_core::Error::new("err.flow.input.sha").with("input", &f.name));
        }
        f.sha256 = sha;
        f.bytes = Some(bytes);
        facts.push((f.name.clone(), fa));
    }
    // a name may come twice (NAME@ID samples of one file): the facts of its first entry, as the file is the same
    mzlab_feat::nature::fill_facets(plan, |n| {
        facts.iter().find(|(m, _)| m == n).and_then(|(_, fa)| *fa)
    });
    Ok(())
}

/// The (polarity, high resolution) of a scan file, from its index or from the scan headers of an mzML; None for a file read by another reader or
/// that does not open (its facets stay unknown and the step checks them when it starts).
fn scan_facts(path: &Path) -> Option<(i8, Option<bool>)> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let mut f = std::fs::File::open(path).ok()?;
    let size = f.metadata().ok()?.len();
    match ext.as_str() {
        "mzidx" => mzlab_core::idxread::Index::open(f, size)
            .ok()
            .map(|ix| mzlab_feat::nature::index_facts(&ix)),
        "mzml" => mzlab_core::scantable::read(&mut f, size)
            .ok()
            .map(|t| mzlab_feat::nature::table_facts(&t)),
        _ => None,
    }
}

#[cfg(test)]
#[path = "flow_inputs_tests.rs"]
mod tests;
