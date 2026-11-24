# mzLab for AI agents, part 2: topics 5-9 and the rules for every item

Part 1 (`AGENT_USE_STUDY.md`) holds the evidence (§0), the ranked roadmap and topics 1-4. Ids AU-n refer to its roadmap table.

## R. Rules that hold for every roadmap item
- **Parity**: every new step, reader, export or route exists in the site (`.wasm` + OPFS) and in the app (native + CLI) with a byte-for-byte parity test (`tests/test_parita_passi.py` lists it with both executors), or the PR body states the real technical limit. New code goes into lazy modules; a `.wasm` cap is never raised (`docs/studies/FLOW_GAPS.md`, `wasm-caps.md`).
- **CLI for agents**: `--json` with stable keys and a versioned contract name, data on stdout, progress on stderr (`--progress jsonl` where it runs long), the exit codes of `cli-agenti.md` (0 ok, 1 data error with partial outputs, 2 usage, 3 plan/schema/integrity, 4 missing input, 5 selector not found, 6 output write, 7 internal, 8 input still being written), `--dry-run` where it writes, an entry in `commands/1`, `docs/schemi/` and the `tools/1` catalog (regenerated with their tool).
- **The user decides**: every automatic choice (a merge, an estimate, a dropped row) is in the output with its evidence and in `prov/1`; no result is hidden or blocked.
- **Files never leave the user's computer**: no item opens the network; AU-11 is the only one that returns data to a model, and only within the sharing level the user set (§7).
- **Tests**: synthetic data only (`tools/dati_sintetici.py`), never skipped.

## 5. `estimate` of parameters from the data
Today `features.estimate@1` (params `polarity`, `axis` ppm | Da) writes `estimates/1`: noise floor from the intensity histogram, ppm tolerance (HR), FWHM and `rt_tol` = 2 × FWHM, dead time, and at unit axis the `da` half-window and the mass defect. It only proposes: `features.detect` does not read it. `llm params` gives `param-proposal/1` from deterministic rules (`R-*`, `llm-params.md`), for example `blank_ratio` 3.

Options:
- A. Document `features.estimate` in a one-step plan and let the agent copy the numbers.
- B. `mzlab-cli estimate FILE... [--axis da|ppm] [--polarity 1|-1] --json` (runs the same step on the files, prints `estimates/1`) and `--emit-set PLAN.json` (prints the `--set INSTANCE.name=VALUE` lines that the estimate maps to the plan's instances, each with the evidence it came from).
- C. An optional `estimates` input port on `features.detect` that, when linked, overrides the defaults.

Recommendation: B. The numbers are shown and the agent or the user passes them explicitly; C would change detection results silently when a link is added. For LR add two estimates: the blank level per m/z band (from files with role `blank`, needs AU-2) and the RT drift between files (anchors of `rtmap.rs`, reported as median and max shift, with a proposal for `rt_warp`). Merge the `llm params` rules into the same output (`proposals` with rule ids) so there is one answer to "which parameters for this data".

Cost: M. Risks: estimates on few files are noisy: print the count of points behind each value and refuse nothing; a polarity-switching file (estimate per polarity). Parity: the step already exists in both halves; the CLI shortcut and `--emit-set` call the same Rust. CLI: exit 1 when a file gives no estimate (others written), 4 missing file; `--emit-set` never writes the plan.

Acceptance tests:
- On a synthetic LR series with known noise and FWHM, `estimate --axis da` gives the noise within 10 % and the FWHM within one scan, and the same bytes as `features.estimate` in a plan.
- `--emit-set` on a plan with instance `det` prints `--set det.min_height=...` lines whose values `flow check` accepts.
- With one blank file the output has `blank_level` per band; without blanks the field is absent and the reason is listed.
- An imposed shift of 0.15 min on half the files is reported as RT drift with a `rt_warp=piecewise` proposal.

## 6. `features.companions` at nominal mass
Today `features.group` (`ionnet/1`) is HR-only (isotopes in ppm with a 0.001 Da floor). At unit mass there are pieces: per-file isotopes at +1 and +2 Da (`isotopes.rs`, M+1/M not computed), per-file families by co-elution (apex within 0.05 min, trace Pearson ≥ 0.9; in-source fragments declared only with MS2 proof; adducts and fragments without MS2 stay `Undetermined`), table dedup within 0.6 Da with area correlation r ≥ 0.9 over 3 files.

Options:
- A. Lower the `features.group` floor to Da: the HR rules (exact mass differences, adduct identity) are wrong at unit mass.
- B. A new step `features.companions@1` on the aligned LR table that writes edges `companions/1` between rows, never merging them:
  - `isotope` (+1, +2 Da, same apex within one FWHM, M+1/M ratio across samples with its spread, and the carbon count range it implies at about 1.1 % per carbon; M+2/M high enough to suggest Cl, Br or S as evidence only);
  - `coelution` (Pearson of areas across samples ≥ `min_corr` 0.9 over ≥ 3 samples and apex within `rt_tol`);
  - `insource` (a lighter row that co-elutes and whose nominal difference is in a short list of common neutral losses, for example 18, 17, 44, 46 Da, editable as a user list `liste/1`).
- C. B plus automatic removal of companion rows from the table.

Recommendation: B. C conflicts with the rejected rule "never merge adduct or in-source rows silently". Each edge carries its score and the numbers behind it; the table gains a `companion_of` column (row id of the strongest parent candidate, empty if none) so a reviewer can filter, and the user removes rows by an explicit `table.filter` option if they choose.

Cost: M-L. Risks: at unit mass +1 Da also catches unrelated co-eluting ions (the correlation and ratio stability are the guard: print both); a fragment list that matches by chance (score by correlation, never by mass alone). Parity: lazy `flowtab` module; byte-for-byte test on a synthetic table. CLI: `--json` summary with counts per edge type; listed in `commands/1` and `tools/1`.

Acceptance tests:
- A synthetic LR table with a compound at nominal m/z M, its +1 at 11 % for 10 carbons and an in-source loss of 18 gives one `isotope` edge (carbon range containing 10) and one `insource` edge, and no row is removed.
- An unrelated ion at M+1 with a different time profile across samples (r < 0.5) gets no edge.
- `companion_of` points at M for both companions; `table.filter` with the new option drops them only when set.
- The same input in the site and in the CLI gives the same `companions/1` bytes.

## 7. An MCP connector for mzLab
Today external agents call `mzlab-cli` from the shell (`for-agents.md`); `llm-choices.md` lists `mzlab-cli mcp --root FOLDER` (stdio JSON-RPC, tools over `flow steps`, check, dry run, run, report, `read_output` ≤ 4 MB inside `--root`) as a proposal that needs the owner's go; `llm-subagents.md` notes the reason held against it: results would reach the client's model provider without the caps and the L0-L2 policy of the built-in assistant.
The built-in assistant has the policy (`policy::decide`: read and propose always allowed; L0 denies edit/run/ui, L1 asks, L2 asks from the 4th run) and consent per data class.

What an MCP server means for the hard rule: a tool result is sent by the client to its model provider, so every byte returned by a tool leaves the machine. Raw files never do; summaries and derived tables do only at the level the user fixed.

Options:
- A. No server: keep the shell route (works for Claude Code, Cowork with a shell).
- B. `mzlab-cli mcp --root FOLDER --share LEVEL`, a thin stdio server inside the CLI binary: each tool is a CLI command run in-process (`mzlab_cli::execute`), so codes, JSON and bytes are the same as the shell; tools generated from `commands/1`, not hand-written.
- C. A separate server in another language: rejected (a wrapper, `llm-choices.md`).

Recommendation: B, after the go, with this flow:
1. **Propose**: `plan_propose` writes a plan under `--root/plans/` and returns `flowcheck/1`; `estimate` and `flow defaults` are read-only tools.
2. **Review**: `plan_review` returns `flowdry/1` (inputs found with SHA-256, instances, canonical parameters, the difference from the defaults) and a `review_id` = SHA-256 of plan + inputs.
3. **Execute**: `plan_execute(review_id)` runs only a reviewed plan whose hash still matches; the client's own tool approval is the first gate, the `review_id` the second; L1 behaviour (ask on every run) is the default.
4. **Read**: `read_output` returns only what `--share` allows: `none` (status, counts, file names and hashes), `summary` (default: `flowrun/1`, `run-report/2`, row counts per state, top N rows), `tables` (derived CSV/JSON up to 4 MB). Raw spectra, mzML, vendor files and paths outside `--root` are never returned at any level.
5. **Log**: every tool call and every returned payload's SHA-256 and size go to `--root/mcp-log.jsonl` (`llmlog/1` form), so the user sees what was shared.
`--share` is set only on the command line that starts the server (the user's config), never by a tool call; there is no tool that changes it, deletes files, writes outside `--root` or opens the network.

Cost: L (protocol, tool generation from `commands/1`, approval hash, log, tests).
Risks: a new component to maintain (mitigated by generating tools from `commands/1`); prompt injection through file names or CSV text returned in summaries (escape and cap strings, mark them as data); a client that auto-approves tools (the `review_id` still binds execution to a reviewed plan).
Parity: technical limit, stated: a stdio MCP server cannot run in a browser page; the site keeps the built-in assistant, and the desktop app can expose the same server through `mzlab cli mcp` (proposal in `llm-choices.md`). CLI: `mzlab-cli mcp --list-tools --json` prints the generated catalog for tests.

Acceptance tests:
- `--list-tools` contains exactly the commands marked agent-usable in `commands/1`; a new command appears without editing the server.
- `plan_execute` with a stale `review_id` (an input changed after review) is refused with `err.mcp.review.stale` and runs nothing.
- With `--share none`, no tool result contains a table row or a file name from outside `--root`; with `summary`, no result exceeds the cap; the log lists every payload hash.
- Path traversal (`../`) in any argument is refused with exit-equivalent 2.
- The same plan run through MCP and through `flow run` gives the same `prov.json` bytes.

## 8. Recipe page for a kinetic LR campaign; `file pack` / `file unpack`
### 8.1 Recipe
The five tested chains (`for-agents.md`, `workflow_tests.rs`) cover folder to CSV, reproducible run, trace to peaks, statistics and vendor triage; none covers a campaign.

Recommendation: a section "Kinetic LR campaign" in `for-agents.md` (or its own area file if it passes 25 KB) and a sixth chain in `workflow_tests.rs` with its asserted call count:
1. `file list DIR --json`; `file convert FILE... --out MZML --name ...` (SAMPLE-NAMES-ROLES).
2. `sample table MZML/*.mzML > samples.json`; `sample import meta.csv --in MZML --out samples.json` (AU-2).
3. `estimate MZML/*.mzML --axis da --json`; `flow defaults table.filter` (AU-5, AU-6).
4. `flow check PLAN.json --json`; `flow run PLAN.json --in MZML --out RUN --dry-run --json`, then run: detection, alignment, gap filling, `table.filter` with `blank_ratio`, per series.
5. Targets: `features.union` over the series, then `quant.targets` over all files (AU-4, QUANT-TARGETS); until they exist, the interim route: one `trace xic --mz LIST` per file then one `peak table --traces T.json --auto` (246 calls, not 67,000); on SIM/SRM files one `chrom.transitions` plan instead. The first lines of the recipe send the agent to the AGENT-DISCOVERY task → command table, `commands --json` and `flow steps --compact` before it writes any script.
6. `tp.dig_lr` for transformation-product hypotheses (DIG-ROBUST); `quant.curve` with the standards' responses.
7. `flow record RUN --roles samples.json`; `flow verify`.
Each step names the states and S/N columns the agent must check before reporting a number.

Acceptance tests: the sixth chain passes on synthetic data with its call count asserted; every command and `--set` cited in the section exists (the same check proposed for `llms.txt`).

### 8.2 `file pack` / `file unpack`
Need: moving a campaign (hundreds of vendor files, folders such as `.D` or `.raw` directories) across a size-limited transfer between the instrument PC and the analysis machine, with proof nothing changed.

Options:
- A. Document `zip -s` and `sha256sum`: different tools per system, folders with many small files, no tie to mzLab's hashes.
- B. `mzlab-cli file pack DIR... --out OUT --part-size 2G` writes `NAME.part0001...` (plain concatenation of a store-only stream, no compression: vendor files compress poorly and speed matters) and `NAME.pack.json` (`pack/1`: every file with relative path, size, SHA-256, and every part with size and SHA-256); `file unpack NAME.pack.json --out DIR` checks every part, rebuilds, checks every file; `file verify-pack NAME.pack.json [--parts-only]`.
- C. B with compression: new dependency or slower; rejected unless measured worthwhile.

Recommendation: B, reusing the stored-method ZIP writer of `crates/mzlab-core/src/zip.rs` so a single-part pack is a valid ZIP; it has no ZIP64 and builds the archive in memory, so it needs a streaming writer and ZIP64 (campaigns pass 4 GB); if that costs more than the item, the parts are a plain concatenation and only the manifest is the contract. The file SHA-256 values are the same as in `prov/1` and `flowdry/1`, so a run can be traced back to the pack.

Cost: M. Risks: a missing or damaged part (exit 3 naming the part; files from intact parts are still written and listed); file names that escape the target folder (refused); a folder still being written by the instrument (exit 8). Parity: the site gets `unpack` and `verify-pack` (parts chosen by the user, written to OPFS, checked by the same Rust in `.wasm`); `pack` in the site is optional and may be stated as a limit (reading large folders in a browser). CLI: `--json` prints `pack-run/1` or `unpack-run/1`; `--dry-run` lists files and the planned parts.

Acceptance tests:
- Pack then unpack of a synthetic folder tree gives identical bytes and SHA-256 for every file.
- A flipped byte in part 2 makes `verify-pack` exit 3 with that part; `unpack` writes the files of the other parts and lists the lost ones.
- A single-part pack opens as a ZIP with the files at their relative paths.
- A manifest entry with `../` is refused.
- The site and the CLI give the same `unpack-run/1` for the same parts.

## 9. Release binaries at a fixed URL, SHA-256 and an install script (describe only)
Today `release.yml` builds `mzLab-X.Y.Z-cli-Linux-x86_64`, `-Linux-aarch64` (static musl) and `-Mac-aarch64` (unsigned, not notarized), lists them in `SHA256SUMS`, and stops at a draft; the owner presses Publish. The only release is an unpublished draft, so `releases/latest` answers 404 and agents build from source (7.5 min, or impossible without cargo).

Plan, owner-only steps marked:
1. **Owner**: publish the draft release; decide on Apple signing and notarization (until then the docs say how an unsigned binary fetched by `curl` runs).
2. **Opus review (`.github`)**: also upload version-free names (`mzlab-cli-linux-x86_64`, `mzlab-cli-linux-aarch64`, `mzlab-cli-macos-aarch64`, `SHA256SUMS`) so `https://github.com/Freeesta/mzlab/releases/latest/download/<name>` is a fixed URL; optionally add `x86_64-apple-darwin` and `x86_64-pc-windows-msvc` CLI builds if users need them.
3. **Worker**: `tools/install.sh` (POSIX sh, no other dependency than `curl` or `wget` and `sha256sum` or `shasum`): detects OS and arch, downloads binary and `SHA256SUMS` from the fixed URL or a given `--version`, refuses on a checksum mismatch, installs into `~/.local/bin` (or `--prefix`), prints `mzlab-cli --version`; a smoke test runs it against a local folder of fake assets. Fix the install text of `for-agents.md` (it lists Linux only).
4. **Worker**: `mzlab-cli --version --json` already gives the commit; the install script checks it equals the tag.

Risks: the fixed URL serves whatever the latest publish is (the script prints the version, and `--version X.Y.Z` pins it); an unsigned macOS binary may be blocked when downloaded by a browser (the docs say to use `curl`). Parity: not applicable (distribution). Acceptance tests: the smoke test above in CI; after publishing, the command in `cli-agenti.md` and the script both verify the checksum on Linux and macOS.

## Roadmap, continued: raw data access (AU-12..AU-19)
From `RAW_ACCESS_STUDY.md` (problem, evidence, gap table, ion statistics, cache, owner decisions D1-D7) and `RAW_ACCESS_STUDY_2.md` (command specification, regression plan). Same columns as the table of part 1; rank continues it, but AU-12 and AU-13 weigh like AU-3: they replace the reader an agent rewrites every session.

| rank | id | what | why | size | depends on | owner-only? |
|---|---|---|---|---|---|---|
| 12 | AU-12 | `raw info` (polarity, scan types, grid, ion quantum, label against data, offset on request) and `raw cache` (= `file index --cache`, `raw-cache/1`); lazy `mzlab-raw-wasm` scaffold | one call says whether ions can be counted and whether the centroid/profile label is wrong | S-M | MZML-SPECTRUM-TYPE (#3678), CONTRACT-ENGLISH | D2 (wasm module), D7 (name) |
| 13 | AU-13 | `ionstat` (quantum, Poisson counts, one-ion S/N floor, L_C/L_D, upper limits) and `raw xic` (verdict peak/trace/nothing with its rule, series summaries, `exclusive`, bounded output, provenance); golden campaign and Python reference | "is there a peak here" without a throw-away reader; two ions on zero background give S/N 17 today | M | AU-12, QUANT-TARGETS (#3681), AU-2 | D1 (S/N model), D6 (thresholds) |
| 14 | AU-14 | `raw batch` (JSONL in and out, one process, files hashed once, cache entries opened in place) with its speed measurement | < 1 s per xic over a campaign from the cache | S-M | AU-13, SPEED-STUDY (#3682: SP2, SP6, SP15) | D5 |
| 15 | AU-15 | `raw ms2` list, spectrum, xic (precursors, mean product spectra per precursor channel, neutral losses, product XIC per precursor) | `trace xic` at level 2 sums every precursor | M | AU-13 | no |
| 16 | AU-16 | binned view (sum per nominal or 0.1 Da bin from the `.mzidx`), `raw scan` (side background, ratio, net S/N), `raw companions` (shape correlation + mass-difference table shared with AU-10) | adducts, in-source fragments and dimers checked at the raw level | M | AU-13; coordinate with AU-10 | D3 (no `.mzbin` yet) |
| 17 | AU-17 | `raw profile` (time course per series, shape class absent/sporadic/rise/decay/rise_fall/flat with its rule) | carryover in washes and formation curves read in one call | S | AU-13, AU-2 | no |
| 18 | AU-18 | `raw isotopes` (M+1/M, M+2/M from counts, expected from `ion pattern`, Poisson z, carbon range) | formula proposals checked with counting error | S-M | AU-16 | no |
| 19 | AU-19 | `--figure` small SVG grid (series x time) for `raw xic` and `raw profile` | the grid of PNGs the agent drew by hand | S | AU-13 | D4 (SVG only or PNG) |

Already covered by running chats (not duplicated): the integration engine and per-file S/N of a target list (QUANT-TARGETS, #3681, reused by AU-13); the from-index path, cache-hit speed and one hash per file (SPEED-STUDY, #3682, SP2/SP6/SP15, used by AU-14); the centroid/profile decision from the points (MZML-SPECTRUM-TYPE, #3678, used by AU-12); sample roles from names (SAMPLE-NAMES-ROLES, #3674) and the task -> command table with near-miss hints (AGENT-DISCOVERY), where the `raw` rows go.

Order: AU-12, AU-13; then AU-14 and AU-15 in parallel; AU-16; AU-17, AU-18, AU-19 in parallel. Every chat ships CLI, app and site with its parity test (§R).

Screening checks for transformation products (one-ion noise floor, kinetic shape, isotope ratio, adduct pairs, matrix blank) continue as TP-0..TP-6 in `TP_SCREENING_RULES.md`; TP-0 starts `mzlab_core::ionstat` (the one-ion floor), which AU-13 extends with counts, L_C/L_D and upper limits; TP-2 shares the shape classes of AU-17, TP-4 the mass-difference table of AU-10/AU-16, TP-5 the function of AU-18.

## Not copied from MZmine
- Defaults hidden in code: mzLab's descriptors stay the single source (AU-5 only prints them).
- An isotope filter in ppm for unit-mass data: AU-10 uses Da and correlation.
- Exports without S/N, and irregular profiles for weak single-ion peaks: keep per-peak S/N in every output (AU-3) and the LR integrator that gives regular profiles at few ions.
