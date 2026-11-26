# mzflow: a feature table for transformation products, built step by step (study)

Study only: no product code. It answers one question of the owner: how can a chemist build, from blocks, a workflow whose every step is visible and that ends in a feature table of the transformation products (TP) of one parent compound, with each feature built piece by piece (XIC, kinetics, MS2, UV-Vis, isotopes, adducts, structure). The screens are in `docs/studies/MZFLOW_SCREENS.md`; the work is cut into the rows of `docs/coda/CODA.md` named in §5.

Short answer: most of the blocks exist on main. What is missing is the **frame** (a start screen with the scheme and the list of feature tables, a per-feature view with fixed slots, one card export) and a few **glue pieces** (a CLI route for one row, a kinetic class column, the three-pillar link, the actor in the log). New engines come after the frame.

## 1. What "mzflow" means here
- **mzflow = the Flow tab, seen as a project.** The graphical Flow already holds a plan (`mzflow/2`), draws it as cards on a canvas, runs it live and opens the result. mzflow adds no new engine and no new file format for the plan: it adds a home screen and a feature view around it.
- **One run of a detection block = one feature table.** Each table appears in a list on the right, like the feature lists of MZmine, with where it came from (block, parameters, files, polarity, time).
- **A feature is filled slot by slot.** The detail view has fixed slots; a slot shows the evidence when it exists, or says which block or command fills it. Nothing is computed in JS; every slot reads a Rust output that the CLI gives too.
- **The user decides.** Classes and links are evidence and sorting keys, never a filter that hides rows; levels are set by the user; the assistant proposes on request only.

## 2. Inventory: wanted capability | exists (where) | gap
Checked on main with `tools/indice.py`, `docs/agenti/INDICE.md` and `mzlab-cli commands --json` (109 commands, 61 Flow steps). "All three" = site `.wasm`, app and CLI with a parity test.

| wanted (owner's spec) | exists (where) | gap |
|---|---|---|
| workflow from blocks, shown as a graph | Flow canvas: cards and links, Canvas / List / Plan tabs, templates, issues box (`flow/lazy/flow-canvas.js`, `flow-shell.js`, `docs/agenti/flow-graphics.md`) | no home screen around it |
| every step shown while it runs | live run: per-card state, progress bar, per-file table (`flow-live*.js`, scheduler `mzlab_core::flow::sched`, `docs/agenti/flow-live.md`) | nothing after the run says "what this step found" in one line per card (counts) for every step |
| list of feature tables on the right (MZmine-like) | outputs kept in OPFS `mzlab/flow/live/<plan_key>.*`; `flow-run-end.js` opens the last table; `flow-views-features.js` opens a `.mzfeat` from disk | **no list**; no CLI listing of the tables of a run folder |
| feature table (filter, sort, columns) | `TPTAB` (`TP_Mine/js/42_tp_table.js`): sortable headers, chips, columns menu, pinned and virtual rows, CSV/Excel of the view; `PIPET` (`37_feature_table.js`) | no `origin` column (A/B/C); kinetic class not one column |
| per-feature detail view | `TPRIGA` (`TP_Mine/js/44_tp_riga.js`): tabs overview (XIC of all files coloured by time, blank/dark dashed, MS1/MS2/mirror), samples, identity, quality, structure; model `row/2`, `row_traces/2` (`crates/mzlab-feat/src/row.rs`) | slots for kinetics, UV-Vis, isotopes/adducts not in it; **no CLI route for `row/2`** (wasm `table_row` and desktop only) |
| parent: SMILES / InChI / drawing -> formula, mass, adducts, isotopes | `mol draw`, `mol measure` (logP "Prev."), `ion mz`, `ion pattern`, adduct calculator (`calc-addotti.js`), Draw tab (Ketcher) and mzDraw; recipes take `parent.json` with a formula | no parent card as one Flow input; pKa only from a table of typical values; reference MS2 of the parent not linked at the start |
| file metadata grid, roles, from names, CSV | metadata grid (`flow/lazy/flow-meta-grid.js`, `samples/1`): roles sample, t0, blank, dark, std, qc...; time from names (`samples/names.rs`); CSV/XLSX import; `sample table/import/group` | pre-flight warnings exist and show in the grid, never blocking (`samples/validate.rs`: `noT0`, `noBlank`, `noDark`, `onePoint`); dedicated MS2, SIM and PDA files are file kinds, not roles |
| QTRAP 3200 reading | `.wiff` + `.wiff.scan` reader (`crates/mzlab-core/src/wiff/`), DAD channels; step presets `lr` and `hr` of `features.detect`, tolerances from `features.estimate` | no preset named after an instrument |
| HR reading: mzML, Thermo `.raw` | mzML/mzXML read everywhere; **Thermo `.raw` read natively** (`crates/mzlab-core/src/rawreader.rs`, vendored `opentfraw`), only for the file versions checked on real files (57, 63, 64, 66); others refused with `err.file.rawVersion` | the owner's Orbitrap versions must be checked (format facts only) |
| engine A, direct untargeted | `features.estimate/detect/align/gapfill`, `table.filter`, `features.group` (HR ion network, neutral mass), `features.companions` (unit mass: isotopes, NH4/Na/K, in-source losses); `features` CLI | ESI- companions list open; tolerances already printed |
| engine B, kinetic classification | `table compare` (vs T0 and blank: ratios, log2FC, rho, classes, never deletes); kinetic forms by AICc: formation, rise-and-fall, flat, decay (`crates/mzlab-feat/src/kinetic/`); LR Dig labels per condition (`tp.dig_lr`) | the owner's class names in one column; **treated / dark ratio** (dark is pooled with blank in the Dig); tmax and t1/2 per row printed |
| engine C, suspect | `tp.generate` (parent x photocatalytic rules, 1-2 steps, user lists), `quant.targets` (targeted XIC in every file, states detected/forced/absent), `screen.suspects` | 3 generations, editable rule list, BioTransformer |
| one table with the origin of each row | `features.union` (`union/1`, conflicts listed), `table.match` | origin A/B/C per row and "found by two engines" not written |
| isotopes Cl/Br/S | `raw isotopes` (observed vs expected, Poisson z, `cl_br_s_possible`); HR isotope finder + `formula_id/1` | at unit resolution the element is never named (by design) |
| adducts, neutral mass, in-source fragments | `features.group` (HR), `features.companions` (LR, `companion_of`), `lr/isf.rs` | flagged, never deleted: already so |
| ESI+ / ESI- bridge | wasm `cross_polarity` (`polpair/1`, two HR tables); `POLARITY-PAIR` queued (LR) | `polpair/1` has no CLI route |
| MS2 net spectrum, CE, apex, scans | `spectrum get --peak auto` (window at the apex minus the sides; `--minus` a blank), `spectrum dda-window`, `spectrum compare` (cosine, matched and missing ions, CE warning) | neutral losses vs the parent not in `spectrum compare`; mirror card queued (`SPECTRUM-COMPARE-MIRROR`) |
| MS2 linked by m/z + RT window, manual link/unlink | `tp.link` (HR: DDA MS2 with purity, fragments vs parent); study `docs/studies/TABUI_MS2_LR.md` for separate LR MS2 injections | `features.ms2_link` has no executor; dedicated MS2 files and manual link with provenance not built |
| three pillars of the parent-TP link | MS2 vs parent (`tp.link`), kinetic coupling to the parent k (`kinetic/evidence.rs`), RT-logP check (Attributio) | not shown together; RT-shift direction vs parent not a field |
| kinetics: area AND height, normalised, not detected vs not acquired | `kinetics.rs` (first order, A->B->C, bootstrap, AICc); value states `absent` vs `na` (`docs/agenti/value-states.md`); "Compare conditions" chart (`TP_Mine/lazy/groups-view.js`) | no per-feature kinetics slot with both area and height |
| UV-Vis at the apex, difference spectrum | `dad spectrum/subtract/trace/peaks/ms-at/delay` (`docs/agenti/dad.md`), `uv-spettro.js` | UV slot in the feature view; co-elution flag not found; delay estimated, not applied (`DAD-DELAY-APPLY` queued) |
| SIM | SIM/MRM transition table (`chrom.transitions`, `docs/agenti/transitions.md`) | SIM trace as a feature slot |
| structure: RDKit / 2D editor | `mol draw` (own Rust layout; RDKit is a test yardstick, not a dependency), Draw tab, `TPSTRUCT`, `trasf/1` record (`docs/studies/TRANSFORMATION_RECORD.md`) | `trasf/1` wired to no consumer |
| levels 1-5, review state | `annot.level`, `tp.review` (chemist's level and note, refused decisions listed), `TPDEC`; LR capped at 4/5 | - |
| AI on request, reads one feature | assistant `mzlab-llm`: opt-in, sees Rust summaries only, policies L0-L2, every action logged and undoable (`docs/agenti/llm-choices.md`) | no "propose structures for this feature" task; no test that an agent cannot overwrite a user decision |
| audit log user / auto / agent | `logOp` `{op, t}`; `auto:` lines of `sample table`; "by assistant" undo entries; `llm` block in `prov/1` | **no `actor` field** |
| feature card, one page, provenance footer | report `mzreport/1` (HTML with SHA-256), MSn figure SVG/HTML/PNG, `prov/1` | no single-feature card export |
| workflow saved as a template | `mzflow/2` plans, templates in `flow-names.js`, `docs/recipes/kinetic-lr.mzflow.json` | a TP template with parent and metadata inputs |
| phone width | on a phone the app does not load: one card "You are on a smartphone" (`telefono-scheda.js`); Flow warns "made for computers" | a read-only phone view (PHONE-VIEWER) |
| MZmine table import | `features` writes the MZmine "full" CSV; `table compare` reads a CSV table | import of an MZmine table as a list entry: to check |

## 3. The concept in one picture
```
 inputs (left)            the scheme (centre, the Flow canvas)               feature tables (right)
 +-------------+   +---------------------------------------------------+   +---------------------+
 | Parent card |-->| [metadata] -> [A direct] -> [B kinetic class] --+ |   | A direct     ESI+   |
 | Files +     |   |                \-> [C suspect XIC] ------------+--+-->| A direct     ESI-   |
 | metadata    |   |                                    [union + origin] | | C suspect    ESI+   |
 +-------------+   |                                 -> [link: 3 pillars] | | union (A+B+C) *     |
                   +---------------------------------------------------+   +---------------------+
                                                    click a table -> table view -> click a row -> feature view
```

## 4. Phases, re-examined with the owner's new wish
The owner's spec put engine C in v2. Two facts change that: the suspect half of C already runs (`tp.generate` + `quant.targets`), and the owner now wants to see each step as it builds the table. So the phases are cut by **what the user sees**, not by engine.

| phase | the user can | built from |
|---|---|---|
| 0 see | open any table of a run in a list, open a row in a fixed-slot feature view | existing outputs only (rows FLOW-TABLE-LIST, TABLE-ROW-CLI, FEATURE-DETAIL) |
| 1 start | fill a parent card; the metadata grid with its warnings exists | `mol`, `ion`, `samples/1` (PARENT-CARD, INSTRUMENT-PRESETS) |
| 2 detect | run A, B and C (the rules that exist, 1-2 steps) as blocks; each adds a table to the list; one union table with the origin of each row | existing steps + KINETIC-CLASS, ROW-ORIGIN |
| 3 workbench | fill the kinetics, UV and MS2 slots; link MS2 files by hand; see the three pillars | FEATURE-KINETICS, FEATURE-UV, MS2-LINK-FILES, LINK-PILLARS |
| 4 identify | draw or accept a structure, set the level; ask the assistant for 1-3 proposals | Draw, `tp.review`, AI-FEATURE-PROPOSAL, AUDIT-ACTOR |
| 5 close | export one card per feature, save the workflow as a template | FEATURE-CARD-EXPORT, TP-TEMPLATE |
| v2 | 3 generations and editable rules, BioTransformer and ECOSAR by file exchange, a phone viewer, 2 parents, biotic | SUSPECT-GEN3, BIOTRANSFORMER-IO, ECOSAR-IO, PHONE-VIEWER |

## 5. Slices (rows of `docs/coda/CODA.md`, each with a prompt in `docs/coda/prompt/`)
Each slice is one chat and one PR, with parity site = app = CLI and a CLI route for agents. Order: the owner sees something first, engines after. The CLI routes of one feature share the `table` family (`table row`, `table slots`, `table kinetics`, `table uv`, `table card`) so that an agent finds them together.

| # | id | the user gets | needs first |
|---|---|---|---|
| 1 | TABLE-ROW-CLI | `table row`: `row/2` and `row_traces/2` of one row from the CLI (the site already has them) | - |
| 2 | FLOW-TABLE-LIST | the list of feature tables on the right of the Flow; `flow tables` over a run folder | - |
| 3 | FEATURE-DETAIL | the feature view with fixed slots; `feature-slots/1` says per slot what fills it | 1 |
| 4 | PARENT-CARD | one Flow input: structure -> formula, mass, adducts per polarity, isotopes, logP and pKa as predictions, reference MS2 | - |
| 5 | KINETIC-CLASS | step `table.kinetic_class`: class column and Max/T0, Max/blank, treated/dark, tmax, t1/2 | - |
| 6 | FEATURE-KINETICS | the kinetics slot: area and height per condition, mean and SD bars with the replicate points | 3 |
| 7 | ROW-ORIGIN | the origin of each row of the union table | - |
| 8 | FEATURE-UV | the UV-Vis slot: spectrum at the apex, difference spectrum, co-elution list | 3 |
| 9 | MS2-LINK-FILES | an executor for `features.ms2_link`: DDA and dedicated MS2 files, manual link and unlink, provenance | - |
| 10 | LINK-PILLARS | step `tp.pillars`: the three pillars of one row side by side | 5 |
| 11 | FEATURE-CARD-EXPORT | `table card`: one HTML page per feature with SVG charts and a provenance footer | 3 |
| 12 | INSTRUMENT-PRESETS | presets named after the instrument (unit +-0.5 Da, Orbitrap +-5 ppm) | - |
| 13 | POLPAIR-CLI | a CLI route for the wasm `cross_polarity` (`polpair/1`, HR) | - |
| 14 | AUDIT-ACTOR | `actor` user / auto / agent in the decision log; an agent never overwrites a user decision | owner: saved bytes change |
| 15 | TP-TEMPLATE | a TP template: parent, files, metadata, A, B, C, union, pillars | 4, 7 |
| 16 | AI-FEATURE-PROPOSAL | the assistant proposes 1-3 structures for one feature from its text card | 11, 14 |
| 17 | PHONE-VIEWER | a read-only phone view of the tables and of one feature | 2, 3 |
| 18 | SUSPECT-GEN3 | `tp.generate` up to 3 generations with an editable rule list | - |
| 19 | BIOTRANSFORMER-IO | BioTransformer by file exchange: export the parent, import its products as hypotheses | - |
| 20 | ECOSAR-IO | ECOSAR by file exchange: export the accepted structures, import the predictions as a column | - |
| 21 | RAW-VERSIONS | Thermo `.raw` versions beyond 57, 63, 64, 66 | owner: version facts |

Engine B is a classification layer: it marks the rows that A and C find and never adds or removes a row; ROW-ORIGIN therefore writes the engines that found a row (A, C or both) and KINETIC-CLASS the class. Rows 1-3 are the first thing the owner can use: they show what already exists.

## 6. Decisions
Asked to the owner one at a time in the PR chat; when the owner left the rest to the study, the study chose and marked it so. Every choice below is a default the user can change.

| question | decision | by |
|---|---|---|
| start screen | scheme in the centre, feature tables on the right, parent and files on the left, log at the bottom (screens A) | owner |
| engine C in v1 | the base C (rules that exist, 1-2 steps, targeted XIC) is v1; 3 generations, editable rules and BioTransformer are v2 | owner |
| phone | read-only: table list and one feature, one slot at a time; no runs, no editing (v2) | owner |
| replicates in kinetics | mean with SD bars, the replicate points drawn on top; n printed per point | owner |
| card export | HTML page plus SVG charts first (print to PDF from the browser); PDF and PNG later | owner |
| assistant privacy | only the text card of one feature, shown to the user first and sent by the user to a remote model; never raw files | owner |
| Thermo `.raw` | read today for versions 57, 63, 64, 66; other versions wait for the owner's version facts (RAW-VERSIONS) | open |
| BioTransformer and ECOSAR | file exchange (export a list, import the result); nothing bundled, no licence taken, no network call from mzLab | study |
| instrument presets | unit resolution +-0.5 Da and Orbitrap +-5 ppm as named presets; `features.estimate` may propose other values, logged as `auto` | study |
| pKa and logP in v1 | logP from `mol measure` and pKa from the typical-value table, both labelled "prediction"; no computed pKa | study |
| kinetic class names | bell (rise and fall), rising (formation or plateau), decreasing (decay or present at T0), background (flat or as high in controls); thresholds are `--set` parameters whose defaults come from the existing engines | study |
| actor in the log | proposed (AUDIT-ACTOR); it changes the bytes of saved decision files, so the row waits for the owner | open |
