# PERF-02: the hash crates at opt-level 3 in the .wasm builds

You are a chat of mzLab (repository Freeesta/mzlab). Read CLAUDE.md, `docs/studi/OTTIMIZZAZIONE_PRESTAZIONI.md` §1-2 and §7, `docs/agenti/wasm-potenziale.md` (profiles, `tools/banco_wasm.py`), `docs/agenti/wasm-tetti.md` (caps). Branch `claude/perf-impronte-o3` from `origin/main`. English for code, commits and PR.

**Problem (measured).** The fingerprints of a file (`Passata`, `crates/mzlab-wasm/src/nativi.rs`; `mzlab/web/impronta.js`, `impronta-worker.js`) are computed in wasm while the file is copied to OPFS. For a `.raw`/`.wiff` both SHA-256 and SHA-1 are computed (`rust-nativo.js:122`); for an mzML only SHA-256 (`rust-carica.js:9`). The workspace builds `sha1` and `sha2` at the default `opt-level = "z"`. A scratch crate with the site's flags (`+simd128`, LTO, `codegen-units = 1`, `panic = "abort"`, no wasm-opt; Node 22; 256 MB, best of 5) measured:

| opt-level | SHA-256 MB/s | SHA-1 MB/s |
|---|---|---|
| z | 245 | 223 |
| 2 | 242 | 599 |
| 3 | 281 | 626 |

Serial SHA-256 + SHA-1 goes from 117 to 194 MB/s, i.e. −40 % of hash time for vendor files and −13 % for mzML.

**Do.**
1. In the workspace `Cargo.toml` add `[profile.release.package.sha1] opt-level = 3` and `[profile.release.package.sha2] opt-level = 3`. Put one comment above them in the style of the existing ones, with the numbers you measure in step 3. Check with `cargo tree -i sha1 -i sha2` whether `sha1`/`sha2` pull a separate compression-function crate. If they do, that crate needs the same line; measure it.
2. `tools/banco_wasm/`: add a job `hash(mb)` to `src/lib.rs` (SHA-256 and SHA-1 over a fixed buffer, returns a check value built from the digests) and call it from `js/suite.js`. Add a profile `sol_hash` to `tools/banco_wasm/Cargo.toml` (only `sha1`/`sha2` at 3), in the style of `sol_core`. `sha1`/`sha2` are already dependencies of `mzlab-core`: no new dependency. The check value must be identical in every variant.
3. Run `python3 tools/banco_wasm.py` for `base` and `sol_hash` in Node and with `--chromium` (and Firefox/WebKit if they run in your container). Median of at least 5 runs. Report MB/s per engine.
4. Measure the size of EVERY `.wasm` that links `sha1`/`sha2` (engine `mzlab-wasm`, the readers of `tools/build_lettori_wasm.sh`, the modules of `tools/build_feat_wasm.sh` that pull `mzlab-core`) before and after, after `wasm-opt` exactly as the build scripts do. Every module must stay under its cap. **Never raise a cap.** If a module crosses its cap, stop and report the bytes in the PR for Federico.
5. Critical path: measure whether the hash is on the critical path of opening a vendor file in Chromium. Time the OPFS copy alone and the copy + `Passata` on a synthetic file of 300 MB (any bytes are fine for a hash; you can use `tools/bench_raw.py`-style harnessing or a page hook in `impronta.js` that you do not commit). Write both numbers in the PR. Prompt 05 depends on this answer: "copy waits for the hash" or "hash waits for the copy".

**Zone (only these files).** `Cargo.toml` (workspace root, profiles only; you are its only owner in this wave), `tools/banco_wasm/src/lib.rs`, `tools/banco_wasm/js/suite.js`, `tools/banco_wasm/Cargo.toml` (+ its `Cargo.lock` only if cargo rewrites it), `docs/agenti/wasm-potenziale.md` (one row: the hash crates and their numbers). Do not touch `misure-velocita.md`. Put your row in the PR body under "Row for misure-velocita.md"; prompt 05 copies it in.

**Identity.** A hash is fixed by its input. Proof: `cargo test --release -p mzlab-core --test upload_passata --test impronte_parallele --test upload_late`, the new bench job's check value equal across variants, and `sha256` of the `.mzidx`/table of `e2e_wiff` and `e2e_mzidx` unchanged (CI). Parity: the desktop and CLI use `profile.desktop`/`profile.cli` (already O3 everywhere): state that this change concerns the `.wasm` builds only and changes no output.

**Rules.** No `unsafe`, no new dependency, caps unchanged, deterministic. `python3 tools/pre_pr.py` before pushing.

**PR.** Title `Build the SHA crates at opt-level 3 for the browser fingerprints`. Body starts with `VISIBLE: no`. Include the table of MB/s per engine, the bytes per `.wasm` before/after against its cap, and the critical-path answer. Ready for review. Send the Coordinator: PR number, 40-char SHA at green CI, the critical-path answer (it decides prompt 05).
