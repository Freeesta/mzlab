//! The default number of encoding threads of `Opzioni::thread`.

/// The cores minus one (the reader and the writer share the calling thread), at least 1.
#[cfg(not(target_family = "wasm"))]
pub fn thread_predefinito() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get().saturating_sub(1).max(1))
}

/// Always 1 in the browser, which has no thread group.
#[cfg(target_family = "wasm")]
pub fn thread_predefinito() -> usize {
    1
}
