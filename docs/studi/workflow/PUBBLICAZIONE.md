# Publishing the libraries (analysis only)

Part of `docs/studi/WORKFLOW_MODULARI.md` (§12). Nothing is published: every outward action (crates.io, npm, reserved names, DOI) needs Federico's go-ahead. Marks: **V** verified in the repository, **I** hypothesis.

## 1. Candidates and order
| Order | Crate | Why | What is missing |
|---|---|---|---|
| 1 | numpress codec, split from `mzlab-core` (`src/numpress.rs`, 321 lines, V) | small, no dependencies, external specification (MS-Numpress, Teleman 2014) and public reference files of the specification: external truth ready | a crate of its own, English names, a test against the specification vectors |
| 2 | `mzlab-lib` (`.mzlib` format, library parsers, scores; no internal dependency, V) | useful alone, API already gathered in one crate | stable API, documentation, a test against public MassBank/MoNA |
| 3 | `mzlab-lettori` + the reading part of `mzlab-core` (trait `Lettore`, `apri`, mzML writing) | the rarest piece (native reading without vendor libraries) | depends on the large part of `mzlab-core`: to be split first; mzML to validate with the PSI validator (test 2 of §7) |
| 4 | npm: `mzlab-lettori-wasm` | the same as 3 in the browser | only after 3; package with `.wasm` + glue, byte ceilings unchanged |

**Not candidates**: `mzlab-feat` (≈ 68 thousand lines, contracts still moving), `mzlab-desktop`, `mzlab-cli`, the page `*-wasm` crates (tied to the interface). `opentfraw` (in `vendor/`, Apache-2.0 with our changes in `vendor/opentfraw/MODIFICHE-MZLAB.md`, V): not republished; the changes are offered to the original project.

## 2. Licence and dependencies
- `LICENSE`: MIT, © 2026 Federico Cristaudo (V); crates inherit it with `license.workspace = true` (V in `mzlab-lib`).
- Dependency licences are already checked on every PR by the `deny` job of `.github/workflows/rust.yml` with `deny.toml` (V): allowed MIT, Apache-2.0, BSD-2/3, MPL-2.0, ISC, Unicode, Zlib, OFL-1.1, CDLA-Permissive-2.0 (`webpki-roots`, certificate data only), sources only from crates.io. All compatible with MIT for distribution; MPL-2.0 asks to publish changes to ITS files, not to our code.
- Dual licence: where a dependency is «MIT OR Apache-2.0» (e.g. `r-efi`) MIT is chosen.
- `opentfraw` (Apache-2.0): whoever redistributes it must carry the licence and the NOTICE of the changes; this applies to the site and the app today (I: the app package was not checked in this study).
- patRoon is GPL-3: no code copied (§9 of the main file), hence no GPL constraint on the crates.
- No crate has `publish = false` today (0 of 18, V): an accidental `cargo publish` would go through. Proposal: add it to all crates until Federico gives the go-ahead, and remove it only from the chosen crate.

## 3. What a public library needs
- **Stable API with semver**: public names are Italian today (`apri`, `scrivi`, `Limiti`, `Lettore`): an English facade is needed, then `1.0` only when it stops changing; `0.x` before.
- **Documentation**: rustdoc on every public item (`#![deny(missing_docs)]`), a README with an example that compiles (doctest), a declared MSRV.
- **Tests with external truth**: reference vectors of the specification (numpress), public files with known values (the `verita/1` series of the ROSINA.md F bench), mzML validated by the PSI validator; no test that needs the private repository.
- **Citation**: `CITATION.cff` already exists at the root (V); a DOI (Zenodo) per crate only with Federico's go-ahead.
- **No private data**: the package is checked with `cargo package --list` (no file of `mzlab-dati`, no surname of `nomi_vietati.txt`: the same check as `tests/test_esempi_anonimi.py`).
- **Free name** on crates.io and npm: not verified (I).
- **Maintenance**: a CHANGELOG per crate, `cargo semver-checks` in CI before each release (I: not installed).
