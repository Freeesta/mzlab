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
| ION-COUNTS | `docs/coda/prompt/ION-COUNTS.md` | - | correctness: cps and ions per scan type, from the method; reuse `raw xic` ionstat |
| FEATURES-NOMINAL | `docs/coda/prompt/FEATURES-NOMINAL.md` | - | a `features` profile for nominal-resolution triple-quad or QTRAP data; low-mass products kept |
| FILL-LEAN-CENTROID | `docs/coda/prompt/FILL-LEAN-CENTROID.md` | - | the page fills a centroided unit-resolution series on whole indexes; a lean index with the profile flag from the file facts would use less memory |
| MS2-PEAK-AUTO | `docs/coda/prompt/MS2-PEAK-AUTO.md` | chat:MS2-SUM | lost data: `--peak auto` apex, 2 x FWHM, two-sided background |
| FIND-VALUE-STATES | `docs/coda/prompt/FIND-VALUE-STATES.md` | chat:DETECT-VS-FORCED | discoverability: value states already exist; find why they were not reached |
| FIND-SAMPLE-ROLES | `docs/coda/prompt/FIND-SAMPLE-ROLES.md` | chat:ROLES-IN-TP | discoverability: sample roles and `sample import` |
| FIND-MZ-OFFSET | `docs/coda/prompt/FIND-MZ-OFFSET.md` | chat:MZ-RT-CORRECTION | discoverability: `--mz-offset auto`, `dad delay` |
| FIND-MULTI-MASS | `docs/coda/prompt/FIND-MULTI-MASS.md` | chat:CENTROID-MULTITARGET | discoverability: `--sweep`, many-mass `trace xic`, `raw xic`, `quant targets` |
| FIND-BLANK | `docs/coda/prompt/FIND-BLANK.md` | - | discoverability: blank subtraction, `counts.blank`, `spectrum get --minus` |
| FIND-ESTIMATE | `docs/coda/prompt/FIND-ESTIMATE.md` | - | discoverability: `estimate`, `features.union`, `raw xic` as entry points |
| RAW-ISOTOPES | `docs/coda/prompt/RAW-ISOTOPES.md` | RAW-SCAN | lost data: `raw isotopes` (AU-18, study §7) |
| ISOTOPES-UNIT | `docs/coda/prompt/ISOTOPES-UNIT.md` | RAW-ISOTOPES | lost data: group M, M+1, M+2 at unit resolution; Q1 spill-over flag; optional `tp.generate` filter |
| NOMINAL-BIN | `docs/coda/prompt/NOMINAL-BIN.md` | - | `--bin nominal` for low-resolution profile data, honours `--mz-offset` |
| SPECTRUM-COMPARE | `docs/coda/prompt/SPECTRUM-COMPARE.md` | NOMINAL-BIN | `spectrum compare A B`: cosine at the same CE with the matched ions |
| TIMESERIES | `docs/coda/prompt/TIMESERIES.md` | - | fixed-window areas, UV injection check, first-order k; reuse the AU-8 recipe and `quant curve` |
| POLARITY-PAIR | `docs/coda/prompt/POLARITY-PAIR.md` | - | pair [M+H]+ with [M-H]- across the two polarity tables |
| BLANK-CONTAMINANTS | `docs/coda/prompt/BLANK-CONTAMINANTS.md` | - | flag contaminant ions seen in blanks before an acquisition slot is spent; builds on AU-7 |
| STATES-PROV | `docs/coda/prompt/STATES-PROV.md` | - | AU-3: `table.stats` and `quant.curve` name the states they used in `prov/1` |
| NA-CELL-STATE | `docs/coda/prompt/NA-CELL-STATE.md` | needs owner decision: printing `NA` for `na` changes an existing cell value; opt-in step parameter or not | AU-3: aligned CSV `na` versus `absent`, Dig points, per-cell S/N |
| BLANK-BAND | `docs/coda/prompt/BLANK-BAND.md` | - | AU-6: blank level per m/z band in `estimate`; `rotte` must not grow |
| RT-DRIFT-PROPOSAL | `docs/coda/prompt/RT-DRIFT-PROPOSAL.md` | chat:MZ-RT-CORRECTION | AU-6: `estimate` proposes `rt_warp=piecewise` from measured drift |
| LLM-PARAMS-RULES | `docs/coda/prompt/LLM-PARAMS-RULES.md` | - | AU-6 and AU-7: merge the rules of `llm params` with `estimate`; fix the R-blank-1 text |
| FWHM-DEAD-PARAMS | `docs/coda/prompt/FWHM-DEAD-PARAMS.md` | - | AU-6: map `fwhm` and `dead` of `estimate` to step parameters (stop if none fits) |
| UNION-DATAFILE | `docs/coda/prompt/UNION-DATAFILE.md` | - | AU-4: `features.union` reads aligned tables with `datafile:` states, the accepted states logged |
| UNION-POLARITY-NODE | `docs/coda/prompt/UNION-POLARITY-NODE.md` | - | AU-4: page node of `features.union` with a polarity split |
| UNION-WEB-TESTS | `docs/coda/prompt/UNION-WEB-TESTS.md` | - | AU-4: cache-hit path of flow-web-companions, real `lr_union` answer shape, module sizes |
| BLANK-SUMMARY | `docs/coda/prompt/BLANK-SUMMARY.md` | - | AU-7: CLI summary line of the blank filter |
| BLANK-FOLD-COLUMN | `docs/coda/prompt/BLANK-FOLD-COLUMN.md` | - | AU-7: `blank_fold` as an opt-in per-row export column |
| RAWXIC-PARITY | `docs/coda/prompt/RAWXIC-PARITY.md` | - | AU-13: equality with `trace xic` + `peak auto-lr` and `quant targets` on the wasm leg; app-leg `--sample` if the limit is not real |
| RAW-SCAN | `docs/coda/prompt/RAW-SCAN.md` | - | AU-16: `raw scan` (study §4); new lazy module |
| RAW-COMPANIONS | `docs/coda/prompt/RAW-COMPANIONS.md` | - | AU-16: `raw companions` (study §5); new lazy module |
| RAW-PROFILE | `docs/coda/prompt/RAW-PROFILE.md` | - | AU-17: `raw profile` (study §6); new lazy module |
| RAW-MS2 | `docs/coda/prompt/RAW-MS2.md` | chat:MS2-SUM | AU-15: `raw ms2` list, spectrum, xic (study §8); new lazy module |
