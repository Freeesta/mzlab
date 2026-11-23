# Agent drawing: headless 2D depiction for `mol draw` (study)

Question: can agents build and render structures from the CLI (SMILES, molfile, name) without an editor, and with which engine? `mol label|anal|check|cip|hilite|mech|cut|sel|link|export|search` work on a molfile the agent already has; there is no 2D layout and no image from the CLI; SVG/PNG/KET/CDXML come only from Ketcher + Indigo in the page.

## Ketcher vs RDKit vs a layout engine
- Ketcher (EPAM, Apache-2.0, `vendor/ketcher`): an interactive editor (React) with Indigo compiled to wasm inside; it needs a page and a worker, so no CLI and no app parity.
- RDKit (BSD-3): a toolkit (parsing, 2D layout with its own method or CoordGen, MolDraw2D SVG/PNG, reactions); no editor. Already our test oracle (`pip install rdkit`, `tools/`), banned from the capped `.wasm` (`mol.md`, "What not to do").
- What `mol draw` needs is only the middle piece: 2D coordinates for a `mol/1` record plus an SVG writer. Parsing, valences, rings, aromaticity, CIP, wedges (`to_molfile`), highlights and SDF already exist in `mzlab-mol`/`mzlab-feat`.

## Candidates (measured here; sizes with `opt-level="z"`, LTO, no wasm-opt)
| route | size | licence | parity site=app=CLI | notes |
|---|---|---|---|---|
| (a) own Rust layout + SVG in `mzlab-mol` | target ≤150 KB raw wasm (not measured; basis in Method) | ours | same Rust code, byte for byte | reuses the reader, rings, stereo, molfile writer |
| (b) RDKit MinimalLib 2026.09.2 (site) + C++ RDKit (app/CLI) | 7,394,153 B wasm, 2,415,496 B gz (13x the 560,000 B `molkit` cap); wheel 74 MB on disk | BSD-3 | two builds of C++, CMake + Boost + Emscripten in CI | breaks "light", all logic in Rust, no C++ toolchain |
| (c1) `chematic-depict` 1.2.1 (pure Rust) | probe 923,239 B (292,446 gz); layout alone +510,880 B raw | MIT/Apache-2.0 | yes | one owner, born May 2026, 150 releases in 6 months; E/Z wrong 57/100 |
| (c2) `omgkit-depict` 0.0.8 | not probed | BSD-3 | yes | one owner, 0.0.x; E/Z wrong 46/100 |
| (c3) `cosmolkit` 0.6.2 (RDKit port to Rust) | probe 2,240,764 B (711,988 gz); depict 37,703 lines | MIT | yes | one owner, born Sep 2026; RDKit quality, RDKit size |
| (c4) `coordgen` 0.2.2 | C++ wrapper | MIT | needs C++ | last release 2023 |
| (d) Indigo 1.49 headless | page: `indigo-*.wasm` 11,803,444 B (3,764,441 gz), already shipped inside Ketcher only; native wheel 27 MB | Apache-2.0 | C++ native in app/CLI | same objections as (b) |

## Quality (synthetic set, RDKit as oracle)
Set A: 200 invented molecules from 9 scaffold families with 0-3 random substituents (seed 20261120): chain 20, monocycle 25, fused 40, bridged/spiro 20, macrocycle 20 (12-20 rings, crown ether, cyclic peptide, porphyrin), charged 20, stereo 25, salt 20, drug-like 10. Set B: 100 acyclic and macrocyclic E/Z double bonds (C=C, C=N, N=N). Coordinates scaled to median bond = 1. "Clean" = no non-bonded pair < 0.45, no atom < 0.3 from a bond it is not on, no bond crossing, bond-length CV < 0.15, mean ring-angle error < 10° (rings ≤ 8).

| engine | clean A | atom clash | atom on bond | crossing | fragments < 1 bond apart | E/Z wrong (B) | median / p95 µs |
|---|---|---|---|---|---|---|---|
| RDKit own | 90.5 % | 3 | 9 | 6 | 17 | 0/100 | 115 / 369 |
| RDKit CoordGen | 92.5 % | 2 | 4 | 8 | 3 | 0/100 | 176 / 1445 |
| Indigo | 91.0 % | 5 | 5 | 10 | 0 | 1/100 | 41 / 129 |
| chematic | 91.5 % | 2 | 7 | 8 | 0 | 57/100 | 10 / 69 |
| omgkit | 90.5 % | 6 | 15 | 11 | 0 | 46/100 | 134 / 454 |
| cosmolkit | 90.5 % | 3 | 9 | 10 | 17 | 0/100 | 181 / 535 |

Counts are molecules (of 200). Per family every engine is clean on monocycles, charged, stereo (tetrahedral), salts and drug-like; bridged/spiro is 5/20 for all (adamantane, cubane, quinuclidine need a crossing or a squashed ring; RDKit has the fewest crossings, 9 vs 14-20); macrocycles 17/20 (RDKit, Indigo, cosmolkit, omgkit) vs 20/20 (CoordGen, chematic).
Eyeballing: an angular OMe on a steroid sits on a ring bond in 4 of 6 engines; RDKit and Indigo put a bridge atom on the N...C axis of quinuclidine. The two light Rust crates ignore the written double-bond geometry half the time, which is a chemistry error, not a style issue.

## Method
- Sizes: ad hoc probes, not committed. Each is a scratch `cdylib` with one exported function calling the crate's SMILES parser plus layout (and SVG), built with `cargo +1.97.0 build --release --target wasm32-unknown-unknown` (`opt-level="z"`, `lto=true`, `codegen-units=1`, `panic="abort"`, `strip=true`, no wasm-opt); "gz" = `gzip -9 -c FILE | wc -c`. "Layout alone" = the parse+layout probe minus the parse-only probe.
- RDKit MinimalLib: `dist/RDKit_minimal.wasm` of the npm tarball `@rdkit/rdkit` 2026.9.2, measured, not run (cold start not measured). Indigo: `mzlab/web/vendor/ketcher/assets/indigo-*.wasm` of the vendored Ketcher 3.18.0. Wheels: `du -sh` of `pip install rdkit` 2026.09.2 (33 MB `rdkit` + 41 MB `rdkit.libs`) and `epam.indigo` 1.49.0 (27 MB).
- Crate facts: the crates.io API (`/api/v1/crates/NAME`: licence, owners, creation date, number of versions); line counts by `wc -l` on the published sources (`cosmolkit-depict` 0.6.2 `src/`).
- Quality: an ad hoc Python 3.13 script (not committed) on RDKit 2026.09.2 (`rdDepictor.Compute2DCoords`, `SetPreferCoordGen(False/True)`), Indigo 1.49.0 (`layout()`) and a scratch Rust bench (release) calling `chematic-depict` 1.2.1 `compute_layout`, `omgkit-depict` 0.0.8 `generate` with the default `Style`, `cosmolkit` 0.6.2 `with_2d_coordinates`. Times are the layout call alone, wall clock, one cloud container; Python timings include the binding overhead. E/Z read back with RDKit (`DetectBondStereochemistry` on the engine's coordinates).
- Sets A and B: built by that script from the scaffolds and seed above (synthetic, no real data). The generator and the metrics are rebuilt as `draw_quality.py` under `tools/` by the first build PR, which commits the sets as fixtures and re-measures RDKit's reference counts.
- `molkit` cap: `tools/moduli/molkit.toml` (`tetto` 560000, `misura` 352069; rules in `wasm-caps.md`). The 150 KB budget is a target: the room under the former cap (512,000 − 352,069 = 159,931 B) rounded down; the cap was then raised to 560,000 (owner-approved, 23 Nov 2026), leaving 207,931 B; it is split per build PR in the prompts.

## Recommendation: (a), own Rust
- Quality is a solved, measurable target: every engine sits at 90-93 % clean on set A; the gaps are bridged systems and E/Z, both testable against the RDKit oracle in `tools/`.
- Only (a) keeps all logic in Rust, one code path for site, app and CLI (byte-for-byte SVG and molfile), no new dependency and a small `.wasm`. (b) and (d) are 7-12 MB of C++ and two builds; (c) is young single-owner code that either draws wrong E/Z (c1, c2) or is as large as RDKit (c3).
- Gates for the build (a new quality script under tools/, RDKit test-only): clean ≥ 90 % on set A and ≥ RDKit per family; E/Z wrong 0/100 and tetrahedral wedges read back to the same `mol/1` (`from_molfile(to_molfile(draw(m)))`); fragments ≥ 1.5 bonds apart; atom-on-bond ≤ 9; native p95 ≤ 1 ms per molecule; added `.wasm` ≤ 150 KB raw, a target (basis in Method; above that a new module with its own cap, decided by the COO).
- PNG: `resvg` behind the existing `png` cargo feature of `mzlab-cli` (as `msn_png.rs`), never in a `.wasm`; the page rasterises the same SVG with a canvas (stated technical limit: parity is on SVG bytes).
- InChI input: we write no InChI reader (`mol.md`); an `InChI=` or InChIKey spec resolves only through the molecule database (`inchikey` column), otherwise exit 2 with a hint.
- Out of scope: ChemDraw fidelity (CDXML is written as the open schema only), an editor (`--edit` is a JSON patch on the scene, re-laid out by the same code).
