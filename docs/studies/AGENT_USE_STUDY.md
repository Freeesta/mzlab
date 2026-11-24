# mzLab for AI agents running low-resolution LC-MS campaigns: study and roadmap

What an AI agent needs to run a whole low-resolution (unit mass, triple-quad full scan, ESI+) campaign with `mzlab-cli` without help, what is missing today, and in which order to build it. Part 1 (this file): evidence, ranked roadmap, topics 1-4. Part 2 (`AGENT_USE_STUDY_2.md`): topics 5-9 and the rules that hold for every item.

Inputs: two field reports from an agent that ran a 245-file kinetic campaign with the CLI and then repeated it with MZmine 4 from the command line. Every claim was checked against the code and the docs; the verdicts are in §0. Compound and study names are placeholders.

## Already running (not duplicated here)
| chat | scope | what this study asks of it |
|---|---|---|
| SAMPLE-NAMES-ROLES | `file convert --name`, sample roles | output names that keep the sample id, so `sample table` proposes roles from converted names (AU-2 builds on it) |
| DIG-ROBUST | `tp.dig_lr` with standards, merging close-RT rows, value states in the Dig | use the state vocabulary of §2 and keep per-peak S/N in the Dig rows |
| QUANT-TARGETS | bulk targeted integration | meet the acceptance tests of §1.1; AU-3 and AU-4 depend on it |
| CONTRACT-ENGLISH | English JSON labels, parameter names equal between output and `--set` | `flow defaults` (AU-5) prints the same names |
| MZML-SPECTRUM-TYPE | spectrum type in mzML | none |
| mzDraw pathways | pathway drawing | none |
| AGENT-DISCOVERY | task → command table in `for-agents.md`, agent-facing command search, near-miss hints | the biggest win of the field reports: most friction came from not finding commands that exist (`peak table --traces --auto`, `chrom.transitions`, `table.filter` blank rule, `features.group`, `spectra.centroid`, `mol draw`); AU-8 links its recipe into that table |
| SPEED-STUDY | campaign speed: sparse data labelled profile is centroided scan by scan (60-70 % of features and Dig time), 23 separate Dig runs, ~68,000 processes, 527 MB of JSON, uncompressed or binary outputs | AU-1 gives it the macOS arm64 binary so the agent computes where the data are |

## 0. Field report against the code
| claim of the report | verdict | where it stands | consequence |
|---|---|---|---|
| no downloadable binary per release, 7.5 min source build, Mac VM without cargo | partly true | `release.yml` builds `mzLab-X.Y.Z-cli-Linux-x86_64`, `-Linux-aarch64`, `-Mac-aarch64` with `SHA256SUMS`, but the only release is an unpublished draft, so `releases/latest` is 404; asset names carry the version (no fixed URL); no install script; `for-agents.md` still says Linux only | AU-1: publishing is the owner's click; fixed names and the script are small |
| `file convert` naming | true | `NAME.mzML`, `NAME-ID.mzML` for multi-sample sets, no naming option (`cli-convert.md`) | SAMPLE-NAMES-ROLES |
| sample roles | partly true | roles are proposed from names (`sample table`, `samples/names.rs`; types sample, t0, blank, dark, infusion, std, calibration, qc, pool); a CSV reader exists in the engine (`samples/csv.rs`, wasm op `csv_read`) but there is no CLI route, so an agent writes `samples/1` JSON by hand | AU-2 |
| `tp.dig_lr` with standards | true | standards become a column only when the series compares conditions (`legacy/lr/plan.rs`); in MRM they are the ion-ratio reference, no calibration | DIG-ROBUST |
| Dig rows split by close RT | true | an unexpected ion near a larger kept one is dropped, isomers of one hypothesis stay separate (≥ 10 % of the strongest, at most 4; `legacy/lr/rows.rs`) | DIG-ROBUST |
| no bulk targeted integration (~67,000 `trace xic` and `peak auto-lr` calls) | false as stated: bulk routes exist, the agent did not find them | `chrom.transitions@2` integrates a target table on every sample in one call, with S/N, on stored SIM/SRM chromatograms; `peak table --traces PATH --auto` finds and groups every peak of every supplied trace in one call; `trace xic` takes up to 500 m/z per file; missing: one call from full-scan files and an m/z + RT list to a table | AGENT-DISCOVERY (finding them), QUANT-TARGETS (the full-scan call); interim recipe in §1.1 |
| no centroiding or drawing command | false | `spectra.centroid` is a Flow step with a native executor (`flow.rs`, `flow_centroid_tests.rs`); `mol draw SPEC --format molfile|scene|ket|cdxml|svg|png` exists (`cmd/ion.rs`) | AGENT-DISCOVERY |
| parameter names differ between output and `--set` | mostly false | `prov/1` and `flowdry/1` use the canonical `--set` names (tested); the exception is `features.table`, which takes the panel keys `ppm`, `rt_tol` and splits them | CONTRACT-ENGLISH |
| Italian labels in JSON | partly true | no Italian keys found in `crates/mzlab-cli/src`; legacy Italian keys remain in engine crates and summaries (`english-remaining.md`) | CONTRACT-ENGLISH |
| MZmine: one aligned table per polarity | mzLab: per series | `features FILE...` aligns one series into one CSV (slabs for large series); nothing joins series | AU-4 |
| MZmine: explicit value states | mzLab has them | cell states DETECTED, TARGETED, FILLED, ABSENT, NA (`aligned.rs`); CSV `datafile:<F>:feature_state`; ABSENT and NA both print `UNKNOWN` (`export.rs`) | AU-3 |
| MZmine: blank subtraction with fold change | mzLab has the filter | `table.filter` `blank_ratio` (off when empty), `blank_quantity`, `blank_stat`, `blank_scope`, `blank_match`; subtraction (`blank_subtract`) refuses `height` and batch/matrix matching (`filters_blank.rs`), and LR tables use height | AU-7 |
| MZmine: defaults only in the Java classes | mzLab is better, not perfect | `flow steps --json --id ID` gives every default as JSON Schema; `--template` writes a plan at defaults but omits parameters without a default (`blank_ratio`) | AU-5 |
| MZmine assistant: parameter estimation from data | mzLab has a step | `features.estimate@1` (noise floor, FWHM, `rt_tol`, dead time, Da half-window at unit axis; `estimates/1`), `llm params` rules (`param-proposal/1`); `features.detect` does not read the estimate | AU-6 |
| MZmine 13C filter in ppm, unusable at unit mass | mzLab partly | `features.group` groups correlated ions (`ionnet/1`) but its table port requires `resolution: ["hr"]` (`steps/group.rs`); LR has per-file +1/+2 Da isotopes (`isotopes.rs`), per-file co-elution families (r ≥ 0.9), table dedup within 0.6 Da | AU-10 |
| MZmine: no S/N export | mzLab advantage, not exported everywhere | S/N in `chrom.transitions`, `peak table`, `peak auto-lr`, Dig `api.detail`; the aligned CSV writes only area, height, state; the Dig summary rows have no S/N | AU-3 |
| MZmine assistant as a desktop connector | absent in mzLab, by choice | `mzlab-cli mcp --root FOLDER` is a "proposal not implemented, only with the go" (`llm-choices.md`); external agents call the CLI from the shell | AU-11 |
| split archives with checksums | absent | no `file pack`/`unpack`; SHA-256 appears only as a side output (`file index`, `flowdry/1`, `file convert --report`, `prov/1`) | AU-9 |

## Ranked roadmap
Rank = agent time saved per campaign × how often it blocks, divided by cost. Size: S ≤ 1 worker chat, M 1-2, L 3 or more. Every item follows the parity and CLI-for-agents rules of part 2, §R.

| rank | id | what | why | size | depends on | owner-only? |
|---|---|---|---|---|---|---|
| 1 | AU-1 | publish the release; fixed-name assets at `releases/latest/download/`; `install.sh` that checks `SHA256SUMS`; `for-agents.md` install text fixed (it lists Linux only, the Mac arm64 binary is built) | every agent on a fresh machine pays a 7.5 min build or cannot build at all; with the macOS arm64 binary the agent computes where the data are (SPEED-STUDY) | S | none | publishing and the Apple signing choice: yes; `release.yml` asset names: Opus review; script and doc: no |
| 2 | AU-8 | recipe "kinetic LR campaign" as a sixth tested chain in `workflow_tests.rs`, linked from the AGENT-DISCOVERY task → command table | a tested chain with its call count stops the 67,000-call loop; it ships with today's routes and is updated when QUANT-TARGETS lands | S-M | AGENT-DISCOVERY | no |
| 3 | AU-2 | `sample import CSV` → `samples/1` (engine `csv_read` already exists) and `sample export` | an agent writes roles by hand today; MZmine reads a metadata CSV | S | SAMPLE-NAMES-ROLES | no |
| 4 | AU-3 | value states and S/N in every table output: ABSENT and NA distinct, `snr` per cell in the aligned CSV, states and S/N in the targeted and Dig outputs | the agent must know which numbers were measured; S/N is mzLab's advantage over MZmine | M | DIG-ROBUST, QUANT-TARGETS | no |
| 5 | AU-5 | `flow defaults STEP` printing `step-params/1` (every parameter with default or "off when empty", type, limits, unit, `--set` key) | one call answers "what can I change" with the real names | S | CONTRACT-ENGLISH | no |
| 6 | AU-4 | one alignment over many series: union target list from per-series tables, then one targeted pass over all files (`features.union` + QUANT-TARGETS step) | one table per polarity for a whole campaign, as MZmine gives | M | QUANT-TARGETS | no |
| 7 | AU-6 | `estimate` route: `mzlab-cli estimate FILE... --axis da --json` and `--emit-set` (the `--set` lines), plus blank level and RT drift for LR | parameters from the data, shown and logged, the user decides | M | AU-5 | no |
| 8 | AU-7 | blank subtraction at unit mass: allow `height` and `batch`/`matrix` matching in `blank_subtract` | LR tables use height; the fold-change filter already exists | M | AU-2 | no |
| 9 | AU-9 | `file pack` / `file unpack` / `file verify-pack`: split parts plus a `pack/1` manifest with SHA-256 per file and per part | moving 245 vendor files between a lab PC and a VM without loss, verified | M | none | no |
| 10 | AU-10 | `features.companions@1` at nominal mass: M+1/M+2 evidence, co-eluting correlated pairs, in-source fragment candidates, as edges, never merged rows | `features.group` covers HR only; MZmine's ppm filter is useless at unit mass | M-L | AU-3 | no |
| 11 | AU-11 | `mzlab-cli mcp --root FOLDER`: stdio MCP server over the CLI with propose → review → execute and a sharing level fixed by the user | desktop assistants (Claude Desktop, Cowork) can drive mzLab without a shell | L | AU-5, AU-6, AU-8 | the owner's "go" (`llm-choices.md`) |

Raw data access (`mzlab-cli raw`: info, cache, xic with verdicts, scan, companions, profile, isotopes, ms2, batch) continues the roadmap as AU-12..AU-19 in `AGENT_USE_STUDY_2.md` ("Roadmap, continued"); evidence and specification in `RAW_ACCESS_STUDY.md` and `RAW_ACCESS_STUDY_2.md`.

Not ranked here because other chats own them: the task → command table, command search and near-miss hints (AGENT-DISCOVERY, the largest single win), and every speed item (SPEED-STUDY).

Order of the worker chats: AU-1 (owner) and AU-8, AU-2, AU-5 in parallel; AU-3 and AU-4 after QUANT-TARGETS and DIG-ROBUST merge; AU-6, AU-7; then AU-9, AU-10; AU-11 last, after the go.

## 1. One-step bulk targeted integration and a single alignment over many series
### 1.1 Bulk targeted integration (scope of QUANT-TARGETS)
Today: on SIM/SRM files `chrom.transitions@2` integrates a target table over every sample in one call, with S/N. On full-scan LR files: `trace xic FILE --mz LIST` (one call per file, up to 500 ions), then one `peak table --traces T.json --auto` (every peak of every trace, grouped by time) or `--windows W.json` (given windows): 246 calls instead of ~67,000, the interim recipe for AU-8.

Options:
- A. Keep the shell loop and document it: no code, 67,000 processes, ~15 min, no states.
- B. `peak table --traces` as the official route: exists, but the agent builds a traces JSON per file and the output has no `index` reuse.
- C. A Flow step `quant.targets@1` (inputs: `index` list, `targets` CSV with `name, mz, rt, rt_window, da|ppm, polarity`, optional `samples`; output: one long table `targets/1` and a wide CSV) plus the shortcut `mzlab-cli quant targets FILE... --targets T.csv --out OUT`.

Recommendation: C, reusing the existing XIC extraction (`trace-set/1` code path) and the `peak auto-lr` integrator on each cell, so a cell equals what the loop would give. One index load per file, files in parallel with `--workers`.

Output per cell: `sample, target, state, rt_apex, area, height, snr, fwhm, rt_left, rt_right, reason`. `state` uses §2. Cost: M. Risks: memory on 245 files (stream file by file; never hold all XICs); target windows that overlap (report both, never sum).

Parity: the step runs in the site in a lazy module (no `.wasm` cap raised); byte-for-byte test site against app on 3 synthetic files × 5 targets. CLI: `--json` prints `targets-run/1` (counts per state, output SHA-256), exit 1 if any cell is in error but the table is written, 3 for a bad targets CSV (one problem per row and column), 4 for a missing file; `--dry-run` lists files × targets and the windows.

Acceptance tests:
- `quant_targets_tests.rs`: on `tools/dati_sintetici.py` data with 3 files × 5 targets, every cell equals `trace xic | peak auto-lr` on the same window (area, height, apex, S/N bit for bit).
- A target absent from a file gives `state=ABSENT` with area 0, not a missing row; a file whose RT range does not cover the window gives `NA`.
- A targets CSV with a non-numeric `mz` in row 4 exits 3 with `targets.csv:4:mz`.
- 245 synthetic files × 270 targets run in one call, and the time goes to `measurements-rust.md`; target ≤ 2 min with 4 workers in the cloud runner.
- `1` and `4` workers give the same bytes.
- `tests/test_parita_passi.py` lists `quant.targets` with both executors.

### 1.2 A single alignment over many series
A kinetic campaign is several series (conditions, replicates, polarities). Today each series is aligned alone; joining tables by hand loses the states.

Options:
- A. One plan over all files of all series: works now for small campaigns (alignment in slabs), but one RT drift model for every series and memory grows with the file count.
- B. `features.join@1` on aligned tables by (m/z, RT) with the `rtmap.rs` anchors: new matching logic, a second source of truth for alignment.
- C. `features.union@1`: union of the rows of the per-series tables (merge rule: same nominal m/z within `da`, RT within `rt_tol` after per-series RT maps), written as a targets CSV, then `quant.targets` over all files: one table per polarity, every cell measured or explicitly ABSENT/NA.

Recommendation: C. It reuses QUANT-TARGETS, keeps one integration rule, and matches what MZmine gives (one aligned table per polarity) while keeping S/N. A stays documented for one series.

Cost: M. Risks: two rows of the union that fall inside one window (the union step reports them as `conflict` and keeps both, the user decides); RT drift between series larger than `rt_tol` (union logs every merge with both RTs). Parity and CLI: as 1.1; `features.union` prints `union/1` (rows in, rows out, every merge with its reason).

Acceptance tests:
- Two synthetic series sharing 10 ions and with 3 own ions each: the union has 16 rows, and every merge is in `union/1`.
- A shift of 0.2 min applied to series 2 still merges with `rt_warp=piecewise`, and not with `off`; both are logged.
- Union followed by `quant.targets` gives one table per polarity whose cells for series 1 equal the per-series targeted pass.

Status: slice 1 of AU-4 is `mzlab-cli quant union` (the union rule of option C on the per-series feature CSVs, `rt_warp` off or piecewise, `union/1`, the app op `lr_union` and the site's `rotte` `.wasm`); the Flow step `features.union@1` is the open follow-up (`docs/agenti/quant-targets.md`).

## 2. Value states end to end
Today: `aligned.rs` has DETECTED, TARGETED, FILLED, ABSENT, NA; gap filling never counts a filled cell as detected; the CSV column `datafile:<F>:feature_state` prints DETECTED, FILLED, TARGETED (ESTIMATED with the MZmine option) and prints ABSENT and NA both as UNKNOWN; `peakchk` demotes a failed detection to FILLED; the targeted pass marks estimated peaks with a `qreason` bit (off by default).

Vocabulary (one for every output, the same strings in CSV and JSON): `DETECTED` (found by detection, passed the checks), `TARGETED` (integrated in a given window), `FILLED` (gap filling), `ESTIMATED` (shape estimated from too few points; only where the targeted pass sets `REASON_ESTIMATED`), `ABSENT` (window covered, nothing above the noise; area 0), `NA` (the file does not cover the window or the polarity). The MZmine export keeps its mapping (FILLED, TARGETED → ESTIMATED; ABSENT → absent).

Options:
- A. A state column only in the aligned CSV (today).
- B. The vocabulary above in every table output: aligned CSV, `targets/1`, Dig rows (DIG-ROBUST), `quant.curve` responses, `table.stats` inputs.
- C. B plus a per-cell `reason` bitset (why a cell is FILLED or ABSENT).

Recommendation: B now, with `reason` only where it already exists (`qreason`); C later if users ask. Also: `snr` per cell in the aligned CSV next to area and height (the value is already in `row/2` as `q_snr`), and `table.stats` and `quant.curve` state which states they used (default DETECTED + TARGETED; FILLED only with an explicit option, logged in `prov/1`).

Cost: M (touches export, the Dig summary and two readers). Risks: CSV consumers that parse `UNKNOWN` (the CHANGELOG notes the split); the column count grows by one per file (`snr`). Parity: export lives in Rust for both halves; the existing byte-for-byte export tests are regenerated with their tool. CLI: the states appear in `--json` summaries as counts per state.

Acceptance tests:
- The export of a synthetic table with one cell of each state prints the six strings, and ABSENT is distinct from NA.
- `snr` in the CSV equals `q_snr` of `row/2` for every cell.
- `table.stats` on a table with FILLED cells excludes them by default, and `prov/1` names the states used.
- The MZmine-format export still prints ESTIMATED for FILLED and TARGETED (golden file unchanged).

Status: slices 1-2 of AU-3 (state and S/N in the concentration outputs, the Dig detail, `peak auto-lr`, `peak table`, `chrom.transitions`) are in `docs/agenti/value-states.md`, with the follow-ups still open (aligned CSV `snr` and the ABSENT/NA split, `table.stats` state filters).

## 3. CSV sample metadata and blank subtraction with fold change
### 3.1 Metadata CSV
The engine reads `samples/1` from CSV (`samples/csv.rs`: separators `,` `;` tab, synonyms `condition`, `replicate`, `role`, `dose`, `time`, free columns kept), and the page uses it (`F.metaImport`); the CLI has only `sample table` (roles from names).

Options:
- A. Document the `samples/1` JSON so the agent writes it.
- B. `mzlab-cli sample import META.csv [--files FILE...|--in DIR] --out samples.json` (merge by file + sample or SHA-256, as the page does) and `sample export samples.json --csv`.
- C. B plus a `samples` input accepted as CSV directly by `flow run`.

Recommendation: B. C hides the merge rules and the row errors inside a run. MZmine's "type from the name plus a CSV" is then two calls: `sample table` then `sample import` over it.

Cost: S. Risks: a CSV row naming a file that is not in the folder (reported, never dropped silently); role words outside the vocabulary (reported with the accepted list). Parity: the wasm op `csv_read` and the CLI call the same function; byte-for-byte test on one CSV. CLI: `--json` prints `samples/1` on stdout, `--dry-run` prints the merge plan (rows matched, new, unknown), exit 3 for row errors (`meta.csv:7:role`), 4 for a missing CSV, 5 for a named file absent from the folder.

Acceptance tests:
- A CSV with `;` separator and a `Role` header gives the same `samples/1` bytes as the wasm `csv_read`.
- A CSV with an unknown role in row 3 exits 3 with that row in `problems`.
- `sample table` followed by `sample import` keeps the proposed roles of files not in the CSV and overwrites the rest.
- Round trip: `sample export --csv` then `sample import` gives the same bytes.

### 3.2 Blank subtraction with fold change
The filter exists: `table.filter` with `blank_ratio` (a row passes if its max sample ≥ ratio × blank level), `blank_quantity` area | height | norm_area, `blank_stat` mean | max, `blank_scope`, `blank_cells`, `blank_match` all | batch | matrix (with per-file `blank_fallback`), `blank_keep_istd`; `llm params` rule R-blank-1 proposes 3. Subtraction (`blank_subtract`) refuses `height` and batch/matrix matching.

Options:
- A. Keep the filter, document it in the LR recipe (AU-8), keep subtraction refused for height.
- B. Allow subtraction on height and with batch/matrix matching, writing the subtracted value and the blank level per cell, floored at 0 with state kept.
- C. A separate `table.blank@1` step that only subtracts.

Recommendation: A at once (docs), then B inside `table.filter` (AU-7); C duplicates the matching code. Show the fold change per row (`blank_fold`) in the output so the user sees why a row was dropped.

Cost: M. Risks: subtracting heights of peaks with different shapes is a rougher estimate than areas: log the quantity used in `prov/1` and print it in the summary. Parity: `filters_blank.rs` serves both halves; extend the existing tests. CLI: the `--json` summary gives rows kept, dropped by blank, and the threshold.

Acceptance tests:
- On a synthetic LR table with one blank per batch, `blank_quantity=height`, `blank_subtract=true`, `blank_match=batch` runs and each subtracted cell equals height minus that batch's blank height (floored at 0).
- `blank_fold` printed per row equals max sample / blank level.
- Fold 3 drops exactly the rows below 3 and keeps the internal standard when `blank_keep_istd=true`.

## 4. `flow defaults <step>`
Today `flow steps --json --id ID` gives `params` as a JSON Schema with `default`, `minimum`, `maximum`, `enum`, and `--template` writes a valid plan at the defaults but leaves out parameters that have no default, so the agent does not see `blank_ratio` and the other rules that are off when empty.

Options:
- A. Document `--json --id` plus `--template`.
- B. `mzlab-cli flow defaults STEP[@V] [--json]`: a table `step-params/1` with one entry per parameter: `name`, `set_key` (`INSTANCE.name`), `type`, `default` or `null` with `when_empty` ("rule off"), `minimum`, `maximum`, `enum`, `unit`, `algorithm` (the algorithm it applies to), `description` (English, from the descriptor). Text output: one aligned line per parameter.
- C. Make `--template` write every parameter, with `null` for the ones without default.

Recommendation: B. C would make a plan that `flow check` must accept with `null` values, changing the `mzflow/2` contract.

Cost: S (the data is all in `step/1`). Risks: none beyond keeping the names equal to `--set` (CONTRACT-ENGLISH). Parity: descriptors are Rust data shared by both halves; the page's parameter modules already read them (no site change needed beyond a parity test that both print the same `step-params/1`). CLI: exit 5 for an unknown step id, listed in `commands/1`.

Acceptance tests:
- For every step of `steps-1.json`, `flow defaults` lists exactly the keys of `params.properties`.
- `table.filter` lists `blank_ratio` with `default: null` and `when_empty` set.
- Every `set_key` accepted by `flow check` with its default value gives exit 0.
- An unknown id exits 5 with the closest ids in `hint`.
