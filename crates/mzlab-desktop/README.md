# mzlab-desktop

Desktop app of mzLab (Tauri v2): the same web page with a native engine behind the `mzlab://` protocol.
Files are memory-mapped from disk (`memmap2` lives only in this crate); answers are binary arrays except errors and summaries.
Area file: `docs/agenti/rust-desktop.md`.
