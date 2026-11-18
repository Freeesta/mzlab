# Derivatives of a newer format (`error.newerFormat`)
- An `.mzidx` with a header version above `idx_newer::VERSION_MAX` (or magic `MZIDX00N`, N > 1) and an `.mzfeat`/`.mzfeatf` with `version` or `feat_version` above the known ones (or magic `MZFEAT0N`, N > 1) answer `error.newerFormat {kind, version}`; `error.badIndex`/`error.badMzfeat` remain for a damaged file.
- An `msn-graph/N` graph with N above `msngraph::MAX_VERSION` (1) answers `error.newerFormat {kind: msn-graph, version}` in `completa`, `msn-graph compare` and in the page (`infusione-grafo-apri.js`, `KNOWN`): it does not open; `/1` is read.
- The file is not opened with this version, not rebuilt and not overwritten: `archivio.promote` refuses to replace it (`manifesto.js`: `MASSIMI`, `nuovo`), `lrPut` does not remove its index, `lr_index` answers with the warning (it/en) without building; the source stays loaded.
- A lower version is rebuilt from the source as usual (`pulizia.js`, `vecchio`).
- The `.mzfeat`/`.mzfeatf` written by `feat-worker.js` (mzFinder, hidden) do not go through `archivio.promote`: no guard on write.
- Proof: `crates/mzlab-core/tests/newer_format.rs`, `crates/mzlab-feat/tests/new_format.rs`, `tests_e2e/e2e_formato_nuovo.py`.
