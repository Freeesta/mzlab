# Build prompts of the performance study

Study: `docs/studi/OTTIMIZZAZIONE_PRESTAZIONI.md`. At most 3 chats at a time; a wave starts when the previous one is in `main`.

| Wave | Prompt | Zone (owned files) | Gate |
|---|---|---|---|
| 1 | `01-spettro-una-scansione-hr.md` | `crates/mzlab-core/src/hrroutes.rs`, `spectrum.rs`, `examples/visore_tempi.rs`, new test; `visore-veloce.md`, `misure-velocita.md`, `ottimizzazioni-tentate.md` | — |
| 1 | `02-impronte-o3.md` | `Cargo.toml` (profiles), `tools/banco_wasm/`, `wasm-potenziale.md` | stop if a cap would be crossed |
| 1 | `03-lista-file-molti.md` | `mzlab/web/explore.js`, calls in `lista-file.js`, new `tests_e2e/misura_lista_molti.py` (+ `e2e_lista_molti.py`) | change only if the N = 100 measure passes |
| 2 | `04-hrp-hr15-diagnosi.md` | `tools/bench_visore.py`, `visore-veloce.md`, files named by the diagnosis | stop at the diagnosis if the cause is a cap |
| 2 | `05-impronte-due-worker.md` | `impronta.js`, `impronta-worker.js`, `rust-nativo.js` (call), `nativi.rs` (`Passata`), worker tests; `misure-velocita.md`, `ottimizzazioni-tentate.md` | only if PERF-02 finds the hash on the critical path |
