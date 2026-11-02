# 05 — One index sort for the engine's float keys

Prompt ready for a DEV chat: copy everything after the «---» line. Wave 2 (with 04), after 01 is merged (01 touches other files of `mzlab-core`; no overlap, but measure on the merged main).

---

**Role.** DEV chat for mzLab (public repo Freeesta/mzlab). English for code, commits, PR and messages. Read AGENTS.md (in context), `docs/agenti/wasm-tetti.md` («Ordinamenti»), `docs/studi/OTTIMIZZAZIONE_CORE.md` §4.1 and `docs/studi/OTTIMIZZAZIONE_CODICE.md` §2.1. Branch `claude/engine-index-sort` from `origin/main`.

**Goal.** Shrink the engine `.wasm` with no change of speed class or output. twiggy on the engine (`tools/cargo_wasm.sh build --locked --release --target wasm32-unknown-unknown -p mzlab-wasm`, then `twiggy top -n 100000 -f csv target/wasm32-unknown-unknown/release/mzlab_wasm.wasm`; `cargo install twiggy --locked` works in the cloud) shows five copies of the SAME std stable sort, one per closure, each sorting `usize` indices by one `f64` key with `total_cmp`:

| Call site | Key | Order | Bytes (twiggy, pre wasm-opt) |
|---|---|---|---|
| `crates/mzlab-core/src/mapgrid.rs:147` `rt_groups` | `points[a].rt` | ascending | 7 363 + 217 |
| `crates/mzlab-core/src/specpeaks.rs:172` `pad_zeros` | `om[a]` | ascending | 7 338 + 217 |
| `crates/mzlab-core/src/specdiff.rs:178` `subtract_hr` | `rmz[a]` | ascending | 7 310 + 217 |
| `crates/mzlab-core/src/specpeaks.rs:133` `merge_unit` | `mz[a]` | ascending | 5 421 + 217 |
| `crates/mzlab-core/src/mapcells.rs:162` `sticks` | `pts[i].2` | descending (`pts[j].2.total_cmp(&pts[i].2)`) | ≈4 600 |

They are marked hot on purpose (std is ~2x faster than `sortc`): keep the std algorithm, only stop paying it five times.

**Files you own.** `crates/mzlab-core/src/specpeaks.rs`, `specdiff.rs`, `mapgrid.rs`, `mapcells.rs`, and the file of the new helper (e.g. a `pub(crate)` function in an existing small module of `mzlab-core` such as the one that re-exports `sortc`; do not edit `mzlab-picchi/src/sortc.rs`). `docs/agenti/wasm-tetti.md` one line on the helper.

**Work.**
1. Add `pub(crate) fn order_by_f64(key: &[f64], desc: bool) -> Vec<usize>`: `let mut o: Vec<usize> = (0..key.len()).collect();` then ONE `o.sort_by(|&a, &b| { let c = key[a].total_cmp(&key[b]); if desc { c.reverse() } else { c } })`, with the same `#[allow(clippy::disallowed_methods)]` + hot-path comment the call sites carry today, and `#[inline(never)]` so it stays one copy. `c.reverse()` of `key[a].total_cmp(&key[b])` equals `key[b].total_cmp(&key[a])`, so the descending call keeps exactly the order of `sticks` today (stable sort, same comparisons).
2. Replace the five sorts by the helper. `rt_groups` and `sticks` build the key slice first (`points.iter().map(|p| p.rt).collect::<Vec<f64>>()`, `pts.iter().map(|p| p.2)`). Keep everything else of each function as it is (the `truncate(cap)` of `sticks`, the uses of the order).
3. Add one unit test of the helper against the old closure on data with ties, `-0.0`/`+0.0`, NaN and both directions: the permutations must be identical.
4. Measure the site file: build the engine as the site does (`tools/build_site.py` or the wasm part of it, then `tools/ci_wasm_dimensioni.py site/static`) before/after; put raw and gzip in the PR. Expected −15…20 KB (est.; twiggy shows −24…29 KB before `wasm-opt`). Never raise a cap. Speed: run `tools/banco_wasm.py` on the map and spectrum-difference routes if available, else `cargo bench`/a timing test of `pad_zeros` and `rt_groups` on 10^5 points before/after; a slowdown over 5 % stops the PR (report it to the COO).
5. Parity: the same Rust code serves the desktop app (`mzlab-desktop` links `mzlab-core`), so app and site change together; nothing to add.

**Tests.** `cargo test -p mzlab-core` (all goldens and parity tests unchanged, no expected file edited), `cargo clippy -p mzlab-core -- -D warnings`, `python3 tools/pre_pr.py`, e2e of the map and of the spectrum difference run by CI (`e2e_mappa*`, `e2e_differenza*`, `e2e_spettro*`). Never skip, disable or loosen a test.

**PR.** Title «Engine: one std index sort for float keys instead of five». Body starts `VISIBLE: no`, then the twiggy before/after lines, the size table, the timing numbers.

**Delivery.** At green CI ONE message to the COO (≤5 lines: PR, 40-char SHA, engine bytes before → after). PR events: skill `steward`.
