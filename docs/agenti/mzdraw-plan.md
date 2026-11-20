# mzDraw: feature priorities, risks, owner decisions, ready prompts

Report only. Ranks the mzDraw and `mol draw` features by value for a chemist and cost, lists the risks and the decisions Federico must take, and gives ready prompts for separate implementation chats. Inputs: `mzdraw.md` (scope, architecture, slices), `agent-draw.md` (`mol draw`, D1-D4), `mzdraw-competitors.md`, `mzdraw-formats.md`.
Cost: S = one small PR (about half a day of an agent), M = 2-3, L = 4 or more. Value: what a chemist gains, from 1 (nice) to 5 (cannot work without it).

## 1. Priorities
P0 = the editor is not usable without it; P1 = needed to replace Ketcher on a lab machine; P2 = competitive; P3 = later or only if asked.
| P | feature | value | cost | why this rank | owner |
|---|---|---|---|---|---|
| P0 | atoms, bonds, rings 3-8, chains, charge, select/move/delete, undo/redo | 5 | M | the editor itself | `mzdraw.md` slice 1 |
| P0 | MOL V2000/V3000, SMILES, CXSMILES in and out; `mzdraw/1` | 5 | S (readers exist in `mzlab-mol`) | exchange with every other tool | slice 1 |
| P0 | SVG out with the embedded document | 5 | S | figures for reports, re-openable | slice 1, D2 |
| P0 | 2D clean-up (D1 layout) with E/Z and wedges right | 5 | L | SMILES in is useless without it | D1 (`agent-draw.md`) |
| P0 | live `check/1` marks (valence, charge), never blocking | 4 | S | catches drawing errors early | slice 1 |
| P0 | `mol draw` from the CLI with the same bytes | 5 | M | agents, batch figures | D1-D3 |
| P1 | hover hotkeys and one-key groups, user shortcuts | 5 | M | speed of drawing (ChemDraw, JSME) | slice 2 |
| P1 | wedge/hash/wavy, CIP labels | 5 | M | stereochemistry is chemistry | slice 2-3 |
| P1 | abbreviations (editable list, contract/expand) | 4 | M | drawing protecting groups, metabolites | slice 2 |
| P1 | reaction arrows, plus, text with sub/superscript; RXN in/out | 4 | M | transformation products, mechanisms | slice 2, ready prompt P5 |
| P1 | clipboard (molfile text, PNG, KET for Ketcher, own JSON) | 5 | M | copy into Word and into the Drawing page | slice 2, `mzdraw-formats.md` §2 |
| P1 | OPFS autosave with 10 versions | 4 | S | no lost work | slice 2 |
| P1 | settings file with `acs-1996` preset | 4 | M | journal figures | ready prompt P2 |
| P1 | KET and CDXML read | 4 | M | open what colleagues send (ChemDraw) | D4, DRAW-CDXML |
| P1 | PNG out with the embedded document | 4 | S | slides, chat, email | slice 3 |
| P1 | SDF read/write with data fields | 4 | S | lists of standards and suspects | slice 3 |
| P1 | touch and pen, keyboard-only editing, screen-reader outline | 4 | M | tablets in the lab, accessibility | slice 2 |
| P2 | formula, exact mass, m/z of chosen adducts pinned on the canvas | 4 | S (engines exist: `label/1`) | the mass spectrometrist's question | later |
| P2 | per-structure annotations (level, evidence, scores) rendered | 4 | M | identification reports (`levels.md`) | later; model fixed now (`mzdraw-formats.md` §5) |
| P2 | one input bar: name (molecule database), SMILES, InChI, template, command | 4 | M | fastest start | later |
| P2 | CDX read, MRV, CML read; RDF; SMARTS | 3 | M each | rarer files | ready prompt P6 |
| P2 | PDF out | 3 | M | print-ready figures | decision 4 |
| P2 | conformity harness + published benchmark | 4 (trust) | M | proves the matrix and the speed | ready prompts P1, P3 |
| P2 | templates library editor | 3 | M | lab-specific scaffolds | later |
| P3 | XYZ/PDB read, CDXML/CDX/MRV/CML write, InChI write in the app | 2 | M each | rare for this lab | later |
| P3 | Windows clipboard CDX, EMF | 2 | M | ChemDraw users on Windows | desktop only |
| P3 | OCR of a structure image | 2 | L | Ketcher has it (server) | not planned |
| - | macromolecules, Markush enumeration, NMR prediction, names from structure | 1-2 | L | out of scope (`mzdraw.md`) | never, unless the owner asks |

## 2. Risks and trade-offs
- **Layout quality is the hardest part.** Every engine is 90-93 % clean on set A and bridged systems fail for all (`agent-draw.md`); microsecond speed tempts shortcuts. Mitigation: the quality gates of `agent-draw.md` plus the 11 Mayfield obstacles (P3), a per-molecule time cap with a fallback layout and a warning in the import report.
- **Fidelity promises.** CDX/CDXML is huge; claiming "reads ChemDraw" invites bug reports. Mitigation: the import report lists every lost item; the matrix is generated from tests (P1); the text never says "full compatibility".
- **wasm size creep.** Every reader adds bytes; CDX, MRV, CML and XML parsing could double the module. Mitigation: own cap per module (`wasm-caps.md`); rare readers can live in a second lazy module loaded on first use; measured in every PR.
- **Two editors at once.** Ketcher stays on the Drawing page and on `qqq`; features land twice or diverge. Mitigation: mzDraw consumes the same engines (molfile + stable ids), and the Drawing page panels move only when the owner decides (decision 1).
- **Licence contamination.** Reading GPL/LGPL code (CDK, Open Babel, ChemDoodle) or copying their test files would taint the crate. Mitigation: `mzdraw-competitors.md` §5; prompts forbid opening those repositories' code; third-party golden files carry a `SOURCE.md`.
- **Text metrics.** Byte parity across wasm/app/CLI needs fixed advance-width tables, so the SVG may look slightly different from what a browser would set with the real font. Accepted trade-off (`mzdraw.md` "Limits stated").
- **Clipboard limits.** Custom web formats work only in Chromium; Safari and Firefox get molfile text and PNG only*. Accepted; stated in the UI help.
- **Single maintainer.** Docs, tests and the CLI catalog are the protection: an agent can pick any slice from the docs alone.

## 3. Five decisions for Federico (with the recommendation)
1. **When mzDraw replaces Ketcher on the Drawing page** (not on `qqq`). Recommendation: only after P0 + P1 pass the conformity harness and a side-by-side test week on your own drawings; until then mzDraw stays on its hidden page, Ketcher stays everywhere it is.
2. **The identification level in the annotation: the `levels.md` strings (`2a`, `2b`, `3a`, ...) or bare 1-5.** Recommendation: the strings, as mzLab already uses them; `mzdraw.md` says "1-5" and should follow this.
3. **Third-party golden files in the repo.** Recommendation: yes, a capped set (≤ 2 MB, ≤ 300 files) from Ketcher (Apache-2.0) and RDKit (BSD-3) with a `SOURCE.md` each, plus synthetic files for everything else; never CDK, Open Babel, OpenSMILES, BIOVIA examples or PubChem bulk files.
4. **PDF in the page.** Options: (a) own small PDF writer on the display list in Rust (base-14 fonts, same bytes in page and CLI); (b) `svg2pdf`/`krilla` natively only, the page prints. Recommendation: (a) for parity, base-14 fonts only (Helvetica covers the `acs-1996` preset); revisit if a journal needs embedded fonts.
5. **CDX writing and InChI writing.** The ChemDraw format code is BSD-3 now (`mzdraw-formats.md` §1), so a CDX/CDXML writer is legally clean; InChI is MIT but a C library. Recommendation: CDXML write at P2 (ChemDraw users can open it), CDX write only for the Windows clipboard at P3; InChI written only in the app and CLI if ever, never in the `.wasm`, and identity stays InChIKey from the database (`mol.md`).

## 4. Ready prompts (for separate implementation chats)
Each prompt is self-contained. Common rules for all of them (copy into the prompt): read `CLAUDE.md` and the docs named in the prompt; English only; work on `claude/<topic>` from `origin/main`, PR ready for review, never merge; all logic in Rust, Python only for tests and `tools/`; parity: the same function in the site (`.wasm`) and in the app and CLI with a byte-for-byte parity test;
CLI for agents: `--json` with stable keys, exit codes per `cli-agenti.md`, data on stdout, progress and errors on stderr, `--dry-run` where sensible, listed in the agent catalogs; never copy code or test files from GPL/LGPL projects (CDK, Open Babel, ChemDoodle, JChemPaint) and never open their source; no real lab data; never skip a test; run `python3 tools/pre_pr.py` before pushing; regenerate generated files with their tool. Do not duplicate D1-D4 or DRAW-CDXML (`agent-draw.md`, `mzdraw.md`): consume them.

### P1. Format catalog and conformity harness
Build `formats/1`, the machine-readable format catalog of `mzdraw-formats.md` §1, and the conformity test harness that keeps it true. In `mzlab-mol`, a static table per format (`id`, `ext`, `mime`, `read`, `write`, `detect`, `lost_on_read`, `lost_on_write` as stable keys) and `detect_format(bytes) -> {format, confidence, candidates}` with the content rules of §6.
CLI: `mzlab-cli mol formats [--json]` (exit 0) and `mzlab-cli mol detect FILE|- [--json]` (exit 0, 2 on an unknown format with candidates in `error/1`); wasm: the same two functions in the `molkit` module (check the cap). Tests: a new `conformity.rs` in the tests of `mzlab-mol` reads `conformity.json` in its test data (file, format, expected core hash or error key, expected lost keys) and fails on any mismatch; start with the synthetic MOL V2000/V3000/SMILES/CXSMILES files that exist and add a capped third-party set only after the owner's decision 3.
Acceptance: the catalog lists every row of §1 with today's real status (formats not yet readable say `read: false`); `mol detect` recognises each fixture; parity test site = CLI on the catalog JSON bytes; `docs/agenti/mzdraw-formats.md` says the catalog is the source of truth.

### P2. Settings file `mzdraw-settings/1` and presets
Implement the settings types of `mzdraw-formats.md` §4 in Rust (serde, fixed key order, unknown keys kept), the generated JSON schema (with the repo's schema tool), the presets `mzlab`, `acs-1996`, `slides`, `dark` as files, and `resolve(settings, preset) -> Style` with physical units. Place: the `mzdraw` crate if it exists on `main`, otherwise a `style` module in `mzlab-mol` that `mzdraw` will use (no new dependency).
CLI: `mzlab-cli mol style list|show NAME|check FILE [--json]` (check exits 2 with the key of the first invalid field; shortcut conflicts listed); `mol draw --set style=NAME|FILE` uses it when D2 exists. Tests: round trip of each preset file to identical bytes; the `acs-1996` numbers (14.4 pt, 0.6 pt, 2.0 pt, 2.5 pt, 18 %, 1.6 pt, 10 pt, 120°) pinned; an unknown key survives read and write; wasm and CLI give the same resolved-style JSON. Acceptance: schema generated, `controlla.py` green, docs row in `mzdraw-formats.md` updated to "implemented".

### P3. Layout benchmark: Mayfield obstacles and published speed
Add to the quality script of `agent-draw.md` (`draw_quality.py` under tools, create it if D1 has not) a synthetic set covering the 11 depiction obstacles of Mayfield 2016 and Clark 2006 (avoidable and unavoidable overlaps, chain blocks, cis/trans, congested small rings, bonded counterions, spiro centres, macrocycles, ring templates, planar embedding, 3D ring systems): write the molecules yourself as SMILES (no file copied from CDK or the slides), at least 4 per obstacle, committed as a fixture.
Metrics as in `agent-draw.md` (clean, clash, atom on bond, crossing, E/Z read back), RDKit 2026.09 as oracle in Python only. Speed: a native Rust bench (`cargo bench` or a release binary with `--json`, one thread) for layout and layout + SVG per molecule, median and p95 in µs, plus a per-molecule time cap with fallback reported as `fallback: true`. Publish the numbers and the method (versions, machine class, commands) in a new area file `measurements-draw.md` with a row in `INDICE.md`.
Acceptance: the script runs in CI without RDKit for our engine (RDKit numbers stored as reference JSON); a regression of clean % or of p95 beyond 20 % fails a test.

### P4. Images that carry their molecule
Implement `mzdraw-formats.md` §2 for SVG and PNG: the SVG writer (D2) puts the `mzdraw/1` document and the V3000 molfiles in `<metadata>`; the PNG writer (behind the existing `png` feature, `resvg`) adds `iTXt` chunks `mzdraw` and `molfile`; readers `doc_from_svg(bytes)` and `doc_from_png(bytes)` in Rust (PNG chunk parsing needs no image decoding; a small own reader of the chunk list, no new dependency). CLI: `mol draw IMAGE.svg|IMAGE.png` reopens the document; `mol detect` recognises them; wasm: the same readers for paste and drop.
Tests: document → SVG → document identical bytes; document → PNG → document identical; an SVG or PNG without our metadata gives exit 2 with `error.image.no_structure`; parity wasm = CLI. Optional, behind a test against RDKit in `tools/`: an extra RDKit-readable PNG chunk.

### P5. RXN and RDF reader and writer
In `mzlab-mol`, read RXN V2000/V3000 and RDF into a reaction record (reactants, agents, products as `mol/1` records with their coordinates, atom maps kept) and write them back; the `mzdraw/1` `reactions` list maps to it (`mzdraw-formats.md` §5). Follow the BIOVIA CTfile spec chapters 5, 8, 11 (read the free PDF; copy no text or example).
CLI: `mzlab-cli mol rxn FILE|- [--json]` (summary: counts, maps, formulas per side, mass balance per element) and conversion through `mol draw --set format=rxn` when D1 exists; exit 4 for a damaged file with the line number. wasm: the same in `molkit` (check the cap). Tests: synthetic RXN files first (written by hand: esterification, SN2 with maps, a three-component reaction, V3000 with agents), then Ketcher test files if decision 3 allows; read-write-read identity; fuzz with damaged counts (as the molfile readers).
Acceptance: matrix rows RXN and RDF updated, parity test, catalog P1 updated.

### P6. XML formats: CML and MRV readers (CDX later)
Read CML (structure, 2D/3D coordinates) and Marvin MRV (structure, arrows, text, basic `MDocument` graphics) into `mzdraw/1`. First decide the XML parser: measure a minimal own pull parser (elements, attributes, CDATA, entities; no DTD) against `quick-xml` (MIT) for wasm size, and record the numbers in the PR; the CDXML reader of DRAW-CDXML must share the same parser (coordinate with it, do not write a second one). Put the rarer readers in a lazily loaded wasm module if they push `molkit` over its cap.
CLI: through `mol draw FILE --json` (import report `import/1` of `mzdraw-formats.md` §6 with the lost items). Tests: synthetic CML and MRV files written by hand from the public schemas (CML schema Artistic-2.0; Chemaxon docs), RDKit `MarvinParse` files only if decision 3 allows; every unsupported element appears in `lost`. Acceptance: matrix rows CML and MRV updated, parity test, wasm size reported.
