# Roadmap queue (CODA)

The queue of ready-to-run roadmap tasks. The coordinator never has to remember what to open next: every row has a ready prompt file.

## How it works
1. One row is one task; the table below is in launch order, by impact: correctness, lost data, discoverability, the rest, follow-ups, the `raw` commands.
2. A row is launched by opening a chat titled `DEV <ID>` whose prompt is `Read docs/coda/prompt/<id>.md and do it` (so a forgotten coordinator loses nothing).
3. A row whose chat or PR is open is skipped. A task already implemented on main is a discoverability row (`FIND-*`), never dropped: the task is to find why the feature was not reached and fix that. The coordinator checks open chats and PRs; this file holds no running state, so it never needs a push to `main`.
4. At most the owner's global chat cap of worker chats is open at once, mzDraw included (the coordinator holds the current number); Sonnet only. Chats started with an inline prompt have no row; the PR of each is named below as `chat:<ID>`.
5. A row whose `needs first` names ids is launched only after those rows are gone (merged); `chat:<ID>` is a running chat with no row: wait for its PR to merge (its PR deletes the token); `-` means it can start now.
6. A row with `needs owner decision: <what>` is not launched by the coordinator; the owner answers first.
7. A chat's PR deletes its row and `docs/coda/prompt/<id>.md`, deletes its id from the `needs first` cell of other rows, and adds one row plus one prompt per follow-up named in its PR body. The queue is therefore always current at merge.
8. If the task turns out blocked or obsolete, the PR says so and still removes the row.
9. Every prompt starts with `Read docs/coda/BRIEF.md.` (the shared rules) and is at most 3,500 B; ids are upper case words joined by hyphens, at most 24 characters.
10. `python3 tools/controlla_coda.py` checks all of this (it also runs inside `python3 tools/controlla.py`).

## Queue
| id | prompt file | needs first | notes |
|---|---|---|---|
| ION-COUNTS-PEAK-TABLE | `docs/coda/prompt/ION-COUNTS-PEAK-TABLE.md` | - | opt-in ion-scale columns for `peak table` (area, height, noise and S/N in ions), with the factor of `ionscale` per file |
| ION-COUNTS-QUANT-TARGETS | `docs/coda/prompt/ION-COUNTS-QUANT-TARGETS.md` | - | opt-in ion-scale columns for `quant targets` and the Flow step `quant.targets`, with the factor of `ionscale` per file |
| ION-COUNTS-LABELS | `docs/coda/prompt/ION-COUNTS-LABELS.md` | needs owner decision: real-file facts (counts and format only, never names or data) | read the fill time of a trap scan and the Q1 scan time, step and range of `raw xic --ions` from `method/1`, once their labels are known |
| FLOW-RECIPE-FIRST | `docs/coda/prompt/FLOW-RECIPE-FIRST.md` | - | `flow run` estimates the unit-resolution recipe per file, the browser runner and `rebuild` from the first file; decided 26 Nov: the per-file recipe everywhere (bytes change only where the estimates differ; log which recipe from which file) |
| REPEAT-HINT-FLOW | `docs/coda/prompt/REPEAT-HINT-FLOW.md` | - | the repeat hint for other repeated `flow run` plans, not only `tp.dig_lr`; Unix-only state today |
| LABEL-PEAK-WIDTH | `docs/coda/prompt/LABEL-PEAK-WIDTH.md` | - | zero-suppressed profile peaks of 3 points read as sparse in the label check; decide on a peak-width cue |
| RAW-ISOTOPES-FIGURE | `docs/coda/prompt/RAW-ISOTOPES-FIGURE.md` | - | `raw isotopes --figure PATH`: an SVG of the three traces with the region shaded and the observed against the expected ratios |
| ISOTOPES-UNIT | `docs/coda/prompt/ISOTOPES-UNIT.md` | - | lost data: group M, M+1, M+2 at unit resolution; Q1 spill-over flag; optional `tp.generate` filter |
| SPECTRUM-COMPARE-MIRROR | `docs/coda/prompt/SPECTRUM-COMPARE-MIRROR.md` | - | site card of `spectrum compare`: a mirror plot of the two spectra with its data table |
| NOMINAL-BIN-HR | `docs/coda/prompt/NOMINAL-BIN-HR.md` | - | `spectrum get --bin` on a high-resolution profile file: the route answers no scan there; give the bins or a clear refusal |
| NOMINAL-BIN-MINUS | `docs/coda/prompt/NOMINAL-BIN-MINUS.md` | - | `--bin nominal` on a blank-subtracted (`--minus`) or summed (`--sum`) spectrum; today refused with exit 2 |
| TIMESERIES | `docs/coda/prompt/TIMESERIES.md` | - | fixed-window areas, UV injection check, first-order k; reuse the AU-8 recipe and `quant curve` |
| POLARITY-PAIR | `docs/coda/prompt/POLARITY-PAIR.md` | - | pair [M+H]+ with [M-H]- across the two polarity tables |
| FLOW-TABLE-DISK | `docs/coda/prompt/FLOW-TABLE-DISK.md` | - | the facts (rows, files, polarity) of a `.mzfeat` opened from disk in the Flow list, from a head slice and `table_entry` |
| FLOW-TABLE-FINAL | `docs/coda/prompt/FLOW-TABLE-FINAL.md` | - | mzflow 2b: mark one feature table of the Flow list as the final one; the choice is kept and logged |
| PARENT-CARD | `docs/coda/prompt/PARENT-CARD.md` | - | mzflow 4: the parent card as a Flow input, `parent/1`, `mol parent` |
| KINETIC-CLASS | `docs/coda/prompt/KINETIC-CLASS.md` | - | mzflow 5: opt-in step `table.kinetic_class`: class, Max/T0, Max/blank, treated/dark, tmax, t1/2 |
| FEATURE-KINETICS | `docs/coda/prompt/FEATURE-KINETICS.md` | - | mzflow 6: the kinetics slot, `kin-series/1`, `table kinetics` |
| ROW-ORIGIN | `docs/coda/prompt/ROW-ORIGIN.md` | - | mzflow 7: opt-in origin of each row of `features.union` |
| FEATURE-UV | `docs/coda/prompt/FEATURE-UV.md` | - | mzflow 8: the UV-Vis slot, `table uv` |
| MS2-LINK-FILES | `docs/coda/prompt/MS2-LINK-FILES.md` | - | mzflow 9: an executor for `features.ms2_link`, dedicated MS2 files, manual link and unlink |
| LINK-PILLARS | `docs/coda/prompt/LINK-PILLARS.md` | KINETIC-CLASS | mzflow 10: step `tp.pillars`, the three pillars of the parent-TP link |
| FEATURE-CARD-EXPORT | `docs/coda/prompt/FEATURE-CARD-EXPORT.md` | - | mzflow 11: `table card`, one HTML page per feature with SVG charts and provenance |
| SLOTS-SOURCE-STEP | `docs/coda/prompt/SLOTS-SOURCE-STEP.md` | - | `feature-slots/1` names the exact step and output of each slot, not the step by kind; the page passes the table name |
| INSTRUMENT-PRESETS | `docs/coda/prompt/INSTRUMENT-PRESETS.md` | - | mzflow 12: detection presets named after the instrument (+-0.5 Da, +-5 ppm) |
| POLPAIR-CLI | `docs/coda/prompt/POLPAIR-CLI.md` | - | mzflow 13: CLI and app route for the wasm `cross_polarity` (`polpair/1`) |
| AUDIT-ACTOR | `docs/coda/prompt/AUDIT-ACTOR.md` | - | mzflow 14: `actor` user / auto / agent in the logs; an agent never overwrites a user decision |
| TP-TEMPLATE | `docs/coda/prompt/TP-TEMPLATE.md` | PARENT-CARD, ROW-ORIGIN | mzflow 15: a TP workflow template (`mzflow/2`) |
| AI-FEATURE-PROPOSAL | `docs/coda/prompt/AI-FEATURE-PROPOSAL.md` | FEATURE-CARD-EXPORT, AUDIT-ACTOR | mzflow 16: the assistant proposes 1-3 structures from one feature's text card |
| PHONE-VIEWER | `docs/coda/prompt/PHONE-VIEWER.md` | - | mzflow 17 (v2): a read-only phone view of tables and one feature |
| SUSPECT-GEN3 | `docs/coda/prompt/SUSPECT-GEN3.md` | - | mzflow 18 (v2): `tp.generate` up to 3 generations, editable rule list |
| BIOTRANSFORMER-IO | `docs/coda/prompt/BIOTRANSFORMER-IO.md` | - | mzflow 19 (v2): BioTransformer by file exchange |
| ECOSAR-IO | `docs/coda/prompt/ECOSAR-IO.md` | - | mzflow 20 (v2): ECOSAR by file exchange |
| RAW-VERSIONS | `docs/coda/prompt/RAW-VERSIONS.md` | needs owner decision: the version numbers of the owner's Thermo `.raw` files (format facts only) | mzflow 21: Thermo `.raw` versions beyond 57, 63, 64, 66 |
| BLANK-CONTAMINANTS | `docs/coda/prompt/BLANK-CONTAMINANTS.md` | - | flag contaminant ions seen in blanks before an acquisition slot is spent; builds on AU-7 |
| STATES-PROV | `docs/coda/prompt/STATES-PROV.md` | - | AU-3: `table.stats` and `quant.curve` name the states they used in `prov/1` |
| NA-CELL-STATE | `docs/coda/prompt/NA-CELL-STATE.md` | - | AU-3: aligned CSV `na` versus `absent`, Dig points; decided 26 Nov: opt-in step parameter prints `NA`, default bytes unchanged; per-cell S/N not decided |
| SHAPE-GATES-OTHER | `docs/coda/prompt/SHAPE-GATES-OTHER.md` | - | the `quant targets` peak-shape gates (`min_half`, `max_fwhm`, `max_asym`) and the six-check decision log in `peak auto-lr`, `peak table --auto`, `chrom.transitions`, Dig and `raw xic` |
| MZTAB-CELL-STATE | `docs/coda/prompt/MZTAB-CELL-STATE.md` | - | AU-3: per-cell value state in `export.mztab` (areas only today); decided 26 Nov: opt-in parameter, default bytes unchanged |
| BLANK-BAND | `docs/coda/prompt/BLANK-BAND.md` | - | AU-6: blank level per m/z band in `estimate`; `rotte` must not grow |
| RT-DRIFT-PROPOSAL | `docs/coda/prompt/RT-DRIFT-PROPOSAL.md` | - | AU-6: `estimate` proposes `rt_warp=piecewise` from measured drift |
| LLM-PARAMS-RULES | `docs/coda/prompt/LLM-PARAMS-RULES.md` | - | AU-6 and AU-7: merge the rules of `llm params` with `estimate`; fix the R-blank-1 text |
| FWHM-DEAD-PARAMS | `docs/coda/prompt/FWHM-DEAD-PARAMS.md` | - | AU-6: map `fwhm` and `dead` of `estimate` to step parameters (stop if none fits) |
| UNION-DATAFILE | `docs/coda/prompt/UNION-DATAFILE.md` | - | AU-4: `features.union` reads aligned tables with `datafile:` states, the accepted states logged |
| UNION-POLARITY-NODE | `docs/coda/prompt/UNION-POLARITY-NODE.md` | - | AU-4: page node of `features.union` with a polarity split |
| UNION-WEB-TESTS | `docs/coda/prompt/UNION-WEB-TESTS.md` | - | AU-4: cache-hit path of flow-web-companions, real `lr_union` answer shape, module sizes |
| BLANK-SUMMARY | `docs/coda/prompt/BLANK-SUMMARY.md` | - | AU-7: CLI summary line of the blank filter |
| BLANK-FOLD-COLUMN | `docs/coda/prompt/BLANK-FOLD-COLUMN.md` | - | AU-7: `blank_fold` as an opt-in per-row export column |
| ITEMS-BACKGROUND-SAMPLE | `docs/coda/prompt/ITEMS-BACKGROUND-SAMPLE.md` | - | follow-up of FIRST-SAMPLE-HINTS: `file items` and `trace background` take `--sample ID` like `dad`, so their hint names the flag and not `file convert` |
| RT-REF-OTHER-STEPS | `docs/coda/prompt/RT-REF-OTHER-STEPS.md` | - | an internal-reference RT mode (`rt_ref` of `quant.targets`) for `peak`, `trace xic` and the `features` alignment; reuse `quant_targets::rtref` |
| MZ-OFFSET-POLARITY | `docs/coda/prompt/MZ-OFFSET-POLARITY.md` | - | per-polarity `--mz-offset` spec (`+:AUTO,-:AUTO`) for `trace xic` and `features`, as `quant targets --offset auto` already does per file and polarity |
| DAD-DELAY-APPLY | `docs/coda/prompt/DAD-DELAY-APPLY.md` | - | opt-in PDA/UV detector delay applied at conversion from the `dad delay` estimate; `file convert` hints the command when UV and MS traces disagree |
| ROLE-WORDS-CLI | `docs/coda/prompt/ROLE-WORDS-CLI.md` | - | `sample table --role dark=dc,dk` word list and the `model` op as a site op in a lazy module (`rotte` and `hr` are full) |
| SAMPLE-PAIR-SITE | `docs/coda/prompt/SAMPLE-PAIR-SITE.md` | - | `sample pair` as a site op: the pairing notes before the run, same bytes as the CLI route |
| TP-LINK-NOTES | `docs/coda/prompt/TP-LINK-NOTES.md` | - | `tp.link` pairing notes in a result document (its output is a binary `.mzfeat`; no byte may change) |
| ROLE-SHORT-NAMES | `docs/coda/prompt/ROLE-SHORT-NAMES.md` | - | short ambiguous names (`dk`, `dc`, a lone `D`) reported as possible roles, never guessed |
| RAWXIC-PARITY | `docs/coda/prompt/RAWXIC-PARITY.md` | - | AU-13: equality with `trace xic` + `peak auto-lr` and `quant targets` on the wasm leg; app-leg `--sample` if the limit is not real |
| RAW-COMPANIONS | `docs/coda/prompt/RAW-COMPANIONS.md` | - | AU-16: `raw companions` (study §5); new lazy module |
| RAW-PROFILE | `docs/coda/prompt/RAW-PROFILE.md` | - | AU-17: `raw profile` (study §6); new lazy module |
| RAW-MS2 | `docs/coda/prompt/RAW-MS2.md` | - | AU-15: `raw ms2` list, spectrum, xic (study §8); new lazy module |
| SUM-CHANNEL | `docs/coda/prompt/SUM-CHANNEL.md` | - | `spectrum get --sum --id`: a channel's scans in the sum evidence without growing `rawxic` over 5 % |
| FILE-METHOD-FIELDS | `docs/coda/prompt/FILE-METHOD-FIELDS.md` | needs owner decision: counts and format facts of the real `.wiff` method streams (which field holds fill time and Q1 resolution, how many samples and experiments carry them), never file names or data | `file method`: decode fill time and Q1 resolution of `.wiff`; then write MS:1000927 in the mzML |
| MS2-PEAK-HR-NOISE | `docs/coda/prompt/MS2-PEAK-HR-NOISE.md` | - | `spectrum get --peak auto` at a high-resolution level keeps every unmatched noise line above zero (about 1,600 lines on a synthetic Orbitrap parent peak); a net-intensity floor as a `--set` parameter with its default printed |
| CLICK-DRAG-ONE-TRACE | `docs/coda/prompt/CLICK-DRAG-ONE-TRACE.md` | - | a manual drag window on a SIM/MRM panel integrates only the trace under the pointer, as the click does; all channels become an explicit action |
| CLICK-HR-GUARD | `docs/coda/prompt/CLICK-HR-GUARD.md` | - | the click on a high-resolution trace gets the same refusal guard and evidence as the low-resolution click, and the guard S/N matches the S/N of the table |
| CLICK-LABEL-HOVER | `docs/coda/prompt/CLICK-LABEL-HOVER.md` | needs owner decision: whether overlaid panels label only the selected or hovered integration | hover shows the area label of an integration on crowded overlaid traces (owner decides: selected-only labels), and a keyboard action integrates all channels at the cursor time |
| PANEL-DRAG-SCROLL | `docs/coda/prompt/PANEL-DRAG-SCROLL.md` | - | the page scrolls near the window edge while a panel handle is dragged, so a panel can be dropped past the visible part of a long stack |
| APP-SIGN-NOTARISE | `docs/coda/prompt/APP-SIGN-NOTARISE.md` | needs owner decision: Apple Developer account and notarisation credentials, Windows signing service | sign and notarise the Mac app (and sign the Windows installer) of `app-prebuilt` in the same build |
| APP-ROLLING-NOTICE | `docs/coda/prompt/APP-ROLLING-NOTICE.md` | - | an installed app says when `app-prebuilt` holds a newer commit than the running binary (opt-in, once a day, silent on error) |
| DASHED-HOVER-LABEL | `docs/coda/prompt/DASHED-HOVER-LABEL.md` | - | The dashed hover guide of a chromatogram has no label saying it only follows the pointer; add a short time label on the guide and an accessible description, without a tooltip on a pointer-events none line |
| CATALOGUE-IONIC | `docs/coda/prompt/CATALOGUE-IONIC.md` | - | pesticides left out of the built-in list because PubChem gives a salt or a cation (diquat, paraquat, chlormequat, dodine, fosetyl, ziram, propineb, ...): add them as charged species with their own m/z and a stable name |
| BROWSER-NOTICE-REAL | `docs/coda/prompt/BROWSER-NOTICE-REAL.md` | needs owner decision: a real Safari test needs Federico's Mac (and a Firefox install) | try the closable browser notice in real Safari and Firefox, not only Chromium with `?cap-off=` |
| MENU-LABELS-LEFTOVERS | `docs/coda/prompt/MENU-LABELS-LEFTOVERS.md` | - | short wording (no articles) for aria names of menu entries, dialog and side-panel button labels, and help text that quotes an old entry name |
| WIFF-OPEN-INSTANT | `docs/coda/prompt/WIFF-OPEN-INSTANT.md` | - | after the index ahead, Open selected still opens each index in the engine (`lrOpen`, `native_attach`, about 240 ms for 4 samples): open it while the dialog is open so Open is instant |
| PBAR-MODULE-BUTTONS-2 | `docs/coda/prompt/PBAR-MODULE-BUTTONS-2.md` | - | panel toolbar slice: the DDA buttons (`dda-elenco.js`) and `ticbg` (`tic-fondo-ui.js`) become registry actions with a `run` |
| PBAR-REMOVE-OLD | `docs/coda/prompt/PBAR-REMOVE-OLD.md` | PBAR-MODULE-BUTTONS-2 | panel toolbar slice: default `qqq.pbar` on and delete the per-panel tool rows and their overflow code |
| PBAR-PHONE-KEYS | `docs/coda/prompt/PBAR-PHONE-KEYS.md` | - | panel toolbar slice: the bar at 390 px and its keyboard use |
| PBAR-MENU-SCOPE | `docs/coda/prompt/PBAR-MENU-SCOPE.md` | - | panel toolbar slice: one right-click menu per panel kind built from the registry with `scope` (point or panel), groups, keys and the same commands as the bar |
| TIP-SINGLE-FILE-LEGEND | `docs/coda/prompt/TIP-SINGLE-FILE-LEGEND.md` | needs owner decision: whether a single-file session drops the file name from the legends and other readouts too | the chromatogram hover box drops the file name with one file loaded; the legend chips, the XIC lens and the peak label box still repeat it |
