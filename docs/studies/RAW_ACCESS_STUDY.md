# Raw data access for agents: `mzlab-cli raw`, study and specification

Part 1 (this file): the problem, the evidence, what exists (gap table), best practice, the shared ion statistics, the cache, parity, the implementation order and the owner decisions. Part 2 (`RAW_ACCESS_STUDY_2.md`): the specification of every `raw` command and the regression plan. Roadmap ids AU-12..AU-19 continue `AGENT_USE_STUDY.md`.

Input: the report of an agent that ran a field campaign (low-resolution triple quad, ESI+, full scan plus some EPI) with mzLab and MZmine. Every claim was checked against the code at `origin/main` and the open PRs (#3678, #3681, #3682). Compound, sample and study names are placeholders.

## 1. Problem
The agent verified every result that mattered with about 300 lines of throw-away Python that decoded the mzML itself (base64 + zlib, sum to nominal mass, moving average, a grid of PNGs):
1. a product present only in one water matrix (series A) and absent in series B;
2. adduct companions of a product (same apex, +22 and +17 Da from [M+H]+);
3. noise near one ion: sporadic single counts at a mass next to a real product;
4. carryover in the washes after the highest sample, falling wash after wash.

Neither mzLab nor MZmine answered the plain question "in this file, at this mass and this time, is there a peak?". The pipeline (detect, align, gap-fill) was the only door to the data, so the agent could either trust the feature table or rewrite a reader every session, with different mistakes each time (a different bin edge, a different smoothing, a different noise estimate). The cost is not the 300 lines: it is that the verification is not reproducible, not cited, not the same maths as the table it checks, and invisible to the user.

Expanded, the problem has six parts:
- **P1 No question-shaped command.** `trace xic` gives points, `peak auto-lr` gives peaks of one CSV trace, `quant targets` (#3681) gives a table over files. None answers "peak / trace / nothing" with its threshold, per file, with the sample's series and time next to it.
- **P2 S/N undefined near zero counts.** On sparse single-ion data the noise estimators of the code (MAD, with an RMS or SD fallback) fall below one ion, so S/N explodes (§3, measured).
- **P3 No view of the raw scans around a result.** Mean spectrum of a window minus its sides, co-eluting masses, M+1/M+2, product spectra per precursor: each needs a script today.
- **P4 Every call re-reads.** Without `--cache` each command decodes and indexes the file again (82 ms and 4.4 passes per call, SPEED-STUDY); there is no long-lived process.
- **P5 Unbounded output.** `trace xic --sweep` and `peak table --auto` can print hundreds of MB of JSON; an agent needs a bounded summary first and the full data on request.
- **P6 No provenance on checks.** A verification that changes a conclusion must name the program, version, files (SHA-256), command and parameters, like `prov/1` does for Flow runs.

## 2. Evidence
### 2.1 Measured on synthetic traces (`peak auto-lr`, `origin/main`, release build)
Traces of 300 scans every 0.02 min, Poisson counts times a quantum q = 200 cps per ion, window 4.8-5.2 min (script in the PR body, no data kept):

| trace | noise (cps) | peak found | S/N today | S/N with a one-ion floor |
|---|---|---|---|---|
| all zero | 1e-9 | none | - | - |
| one ion | 11.6 | none | - | - |
| two adjacent ions on zero | 11.6 | yes, height 200 | **17.3** | 1.0 |
| five ions on zero | 16.4 | yes, height 400 | **24.5** | 2.0 |
| 8 ions, background 0.05 ion/scan | 41.7 | yes, height 400 | 9.6 | 2.0 |
| 40 ions, background 0.3 ion/scan | 126 | yes, height 1600 | 12.7 | 8.0 |
| 40 ions, background 2 ion/scan | 210 | yes, height 2671 | 12.7 | 12.7 |

The noise is a fraction of one ion whenever most scans are zero: `peakq_lrauto::robust_noise` falls back to the RMS of the first differences when the MAD is 0, `peak1d::noise_of` to the SD and then to 1.0 (a count, not an ion). Two ions pass `lr_sn` 3 by a factor of 6. The agent's "MAD gave noise 0 and exploding S/N" is confirmed for mzLab too.

### 2.2 S/N definitions in the code today (one question, five answers)
- `peak1d` (chrom.transitions, `peak table` windows): smoothed height over 1.4826 MAD of the raw trace far from the apex; SD fallback; floor 1.0.
- `peakq_lrauto` (`peak auto-lr`, `peak table --auto`, `quant targets`): smoothed height over the MAD of first differences; RMS fallback; floor 1e-9.
- `peakq_lr` (teaching integrator): residual MAD or RMS over sqrt(1 - c0), or a manual region.
- `mrm/resolver.rs` (MRM features): generalised Anscombe transform with the ion quantum (smallest positive value) plus MAD: the only Poisson-aware estimator.
- `features.detect` LR: `LR_MIN_SNR` 3 on the detector's own noise (`mzfeat-lr.md`).

So "the same S/N maths as the pipeline" has to name which pipeline. The proposal (§4) keeps every existing number and adds one shared, Poisson-aware module used by `raw` and, after an owner decision, by the others.

## 3. Gap table: need -> what exists today -> real gap
| need | exists today (command, limits) | real gap |
|---|---|---|
| polarity, scan type, RT, scan count | `file info` (`file_info/1`), `file items` (`items/1`: polarity, kind, `ms2_exps`, `prof1`, `mode1`), `file channels` (`channels/1`: `ms:<key>` per scan type with code EMS/EPI/Q1/MRM/SIM, `n_points`, RT) | nothing missing as facts; spread over three schemas, no one-call summary |
| mass grid, ion quantum (cps) | none; the quantum exists inside `mrm/resolver.rs::Stabilise` only | **new**: grid step and phase of the m/z values, quantum and how integer the values are |
| mass offset | `trace xic --mz-offset auto:MZ[@RT]`, `features --mz-offset`, `quant targets --offset auto` (`mzcal`) | measured only on a named ion; no file-level report |
| centroid/profile label with a warning | `items/1` `prof1`; #3678 MZML-SPECTRUM-TYPE (`spectrum_type.rs`, `--spectrum auto`) decides from the points at conversion | **small**: report label against the data on any file, not only at conversion |
| cache once | `--cache DIR` / `MZLAB_CACHE` (`.mzidx` per SHA-256 + engine version, verified, stdout identical with and without), `file index` pre-builds (`index-build/1`) | exists; each hit still hashes the whole file and copies the entry (SP6, SP15) |
| file x scan x nominal-mass bins | `.mzidx` peaks sorted by m/z with a fence (any window is a binary search + slice); `mapscan.rs` per-scan 1 Da cells (maximum, in memory, for maps) | **small**: a sum-per-bin view (not max) exposed to commands; no new file needed (§5) |
| xic: apex, area, S/N, baseline in a window, many files | `trace xic` (points, up to 500 ions, no channel option), `peak auto-lr` (one trace), `peak table --traces --auto`, `quant targets` (#3681: one pass per file, area/height/sn/state/flags per file x target) | **new**: verdict peak/trace/nothing with its rule, Poisson S/N with a one-ion floor, sample metadata and per-series summaries, bounded output |
| mean spectrum with side background | `spectrum get --rt A,B` (mean), `--minus FILE` (a blank file, lines above zero) | **new**: sides of the same file, ratio to background and net S/N per mass, top-N |
| co-eluting companions | per-file LR families (apex within 0.05 min, Pearson >= 0.9) inside `features`; `features.group` HR only; AU-10 planned at table level | **new** at raw level: correlated masses around one target with known mass differences |
| time course and shape class | `table.drift`, Dig kinetics (`tp.dig_lr`, DIG-ROBUST #3675) on feature tables | **new** on raw verdicts: per series curve and a shape class |
| M+1/M+2 against a formula | `ion pattern FORMULA ADDUCT` (`ion_pattern/1`); per-file +1/+2 Da isotopes in `isotopes.rs` (no M+1/M ratio) | **new**: observed ratios from counts, expected from the formula, Poisson uncertainty, z |
| ms2: precursors, mean spectra, neutral losses | `items/1` `ms2_exps` (prec, CE, n), `spectrum list`, `spectrum get --scan/--id dd:2`, `spectrum dda-window`, `spectra.ms2` step | **partial**: no product-ion XIC per precursor (`trace xic` level 2 sums every precursor), no loss list |
| bounded output, `--full` CSV | `--out PATH`, `--format csv`; no limit, no summary-first | **new** convention for the group |
| provenance | `prov/1` in Flow, `--report` of convert/features | **new** block on every `raw` answer |
| fast repeat queries, long-lived | `--cache`; in-process `mzlab_cli::execute` (tests, benches) | **new**: `raw batch` (JSONL in, JSONL out, one process) |
| same maths as the pipeline | integration of `quant targets` = `peak auto-lr` engine | reuse it; add the shared ion statistics (§4) |

Conclusion: the facts exist; what is missing is the question-shaped layer (verdict, threshold, metadata, summaries, bounds, provenance), a Poisson-aware S/N, three raw views (side-subtracted spectrum, companions, isotopes), the product-ion XIC per precursor, and a one-process batch route.

## 4. Shared ion statistics (`mzlab-core::ionstat`)
One module, used by every `raw` command, documented once, with an estimator name written next to each number (as `peak1d::SNR_ESTIMATOR` does).

- **Ion quantum q** (cps per ion), per file, level and polarity: the smallest positive intensity of the probed scans (the `Stabilise` rule), accepted when at least 90 % of positive values are within 2 % of an integer multiple of q (`quantum_support`). Otherwise q is `null` (continuous data, an analog detector or a processed file) and every count-based number is `null` with `warn.raw.noQuantum`; the MAD estimators of the pipeline stay.
- **Counts** c = y / q in a trace that sums a mass window per scan.
- **Background** of a window [a, b]: the two sides [a - w, a - g] and [b + g, b + w] (defaults: w = b - a, at least 0.3 min; gap g = 0.05 min), mean count per scan λ_B over n_B scans.
- **Net counts** of the peak region (the pipeline's start..end, n_P scans): N = Σc - λ_B n_P, variance σ² = Σc + λ_B n_P² / n_B (gross plus the estimated background).
- **Noise per scan with a one-ion floor**: σ_scan = q · max(1, sqrt(λ_B)) in cps. `sn_floor` = the pipeline height above its baseline / σ_scan. This is the agent's "floor of one ion": no S/N above 1 for a single count.
- **Count significance** (Currie 1968; exact Poisson for B = 0, Currie 1972): critical level L_C = 1.645 sqrt(λ_B n_P (1 + n_P / n_B)), detection limit L_D = 2.71 + 2 L_C (3.0 counts when λ_B = 0). `z_counts` = N / sqrt(σ²) with σ² floored at 1.
- **Upper limit** when nothing is seen: 95 % upper limit of the net counts (3.0 at zero background, Poisson), reported as counts and as an area in counts x s (q times the scan time), so "absent" carries a number.
- **Pipeline numbers kept**: `area`, `height`, `sn`, `start_rt`, `end_rt`, `points`, `shape` come from `peakq_lrauto::lr_auto_integrate` and `peakq::quality` exactly as `quant targets` computes them (same rounding of the trace: rt 4, y 1 decimals), so a raw-versus-table difference means a data problem, not a code difference.

### Verdict (defaults; every threshold printed in `rule`)
| verdict | rule (all must hold) |
|---|---|
| `peak` | N >= L_D; `sn_floor` >= 3 (`min_sn`); at least 3 consecutive scans with c >= 1 in the peak region (`min_points`); apex inside the window (else flag `edge`) |
| `trace` | not `peak`; gross counts in the region >= 2 and N >= L_C |
| `nothing` | neither; `ul95_counts` and `ul95_area` given |
| `no_data` | no scan of that level, polarity and channel in the window (with the reason) |

The `rule` field names the numbers: `"N 7.4 >= L_D 6.5; sn_floor 2.0 < 3 -> trace"`. The thresholds are parameters (`--set min_sn=`, `min_points=`, `alpha=0.05`); changing them is logged in the provenance. No verdict hides a number.

## 5. Cache design
**Decision proposed: no new file format in the first chats.** The `.mzidx` is already the cache the agent asked for: peaks sorted by m/z per level x polarity section with a fence every 16 384 peaks, so a mass window across all scans is a binary search and one slice read; the scan table (RT, TIC, polarity, kind) is in the header.

The `--cache` entry is keyed by the SHA-256 of the file, the sample, the index version and the engine version, verified before use, and stdout is identical with and without it (`cli-agenti.md`, `idx_cache_tests.rs`). `raw cache FILE...` is therefore `file index --cache` with a `raw-cache/1` answer (per file: status built | reused, key, size, scans, sections, quantum).

**Binned view** (in memory, from the `.mzidx`): for one section, a sparse bin-major table (per bin: scan positions and summed intensity) built in one walk over the m/z-sorted peaks.
- Bins: `nominal` (width 1 Da, [n - 0.2, n + 0.8) as the unit cells of the session map, `mapgrid.rs`/`mapscan.rs`, shifted by the file's mass offset when one is given) or `0.1` Da; half-open; the edges are printed in every answer (`bin_edges`).
- Sums in m/z order inside a bin (the order of `Index::sum_window`), so a bin queried through the view and the same half-open window queried through the index give the same doubles; a test asserts bytes equal.
- Cost: one pass over the section (a sparse 3.9 MB LR file has about 130 000 peaks: well under 10 ms expected); memory 12 B per (scan, bin) cell present. An LRU keeps the views of the files a `raw batch` process touches (cap shared with the index block cache).

**When a persistent `.mzbin` would pay** (deferred, owner decision D3): HR profile files where a view costs seconds, or campaigns of thousands of files. Then: key = SHA-256 of the file + bin spec + engine version, stored next to the `.mzidx` entry, checksum file as the `.mzidx.sum`, atomic write, never required (a missing or stale entry is rebuilt, `warn.cache.stale`), and byte-identical answers with and without it.

**Speed target** (< 1 s per xic over a campaign, from the cache): SPEED-STUDY measured 17.6 ms per cached call, of which the whole-file SHA-256 and the entry copy dominate. `raw batch` hashes each file once per process (SP15) and opens entries read-only in place (SP6); the XIC from the index is the SP2 path (about 3 ms open + under 0.1 ms per mass). Expected: 245 files x 1 mass in about 0.2-0.8 s on 4 cores after the first hash; to be measured in AU-14 with `tools/bench/speed_study.py` (#3682).

**Determinism**: files are answered in input order whatever the worker count (`par::par_map`, as `quant targets`); no clock, no random number; floats printed through `json::Out`.

**Site half (WASM/OPFS)**: the site already keeps every `.mzidx` in OPFS (`mzlab/idx/<name>.mzidx`) and reads it in slices; the `raw` functions are pure over `Routes<R: ReadAt>` and the index, so the same Rust runs in a lazy wasm module (`mzlab-raw-wasm`, own module: QUANT-TARGETS found +19 % growth when its engine joined `flowtab`, over the cap) called by the worker with the OPFS file handles.

No new OPFS folder unless D3 adds `.mzbin` (then `mzlab/bin/`). Memory: the binned view of one file is bounded by its section (the site's 300 MB file limit does not apply to index reads); the site never holds more than one view per worker.

## 6. Parity, CLI contract and provenance
- Every `raw` command is a function of `mzlab-core::raw` (Rust) with three callers: the CLI (`crates/mzlab-cli/src/raw_*.rs`), the desktop app (native, same function behind an `api/raw/*` route) and the site (wasm op `raw_*` in the lazy module). Parity test: `tests/test_parita_passi.py` style, one synthetic mzML, the same JSON bytes from the CLI and the `.wasm` (Node runner, as the existing parity tests) for every command and its `--full` CSV.
- CLI contract (`cli-agenti.md`): `--json` default with versioned schema names (`raw-info/1`, `raw-xic/1`, ...), data on stdout, progress on stderr (`--progress jsonl` for many files), exit codes 0 ok (a `nothing` verdict is a result, exit 0), 1 data error with partial output (a file unreadable, the others answered), 2 usage, 4 missing input, 5 selector not found (sample, channel, precursor), 6 output write, 7 internal. Entries in `commands/1`, `docs/schemi/raw-*-1.json`, the `tools/1` catalog, `flow steps --compact` where a step exists, `for-agents.md` task -> command table.
- **Provenance block** in every answer (`provenance`): `program` "mzLab", `version`, `engine` (index and ionstat versions), `command` (argv as given), `files` [{`name`, `sha256`, `size`, `sample`}], `params` (every parameter, defaults resolved), `cache` (`hit`|`built`|`none` per file), `choices` (automatic choices: quantum, phase, offset, background sides), `cite` (text of `citation.rs`). The same keys as `prov/1` where they overlap.
- Errors: `error/1` on stderr with an `err.raw.*` key through `lang/` (English), the offending argument and a near-miss hint (AGENT-DISCOVERY).

## 7. Best practice from other tools, and what we take
| source | what it does | adopt / adapt / reject |
|---|---|---|
| indexedmzML (PSI mzML 1.1, ProteoWizard msconvert) | byte-offset index at the end of the file, random access by spectrum id | adapt: mzLab already indexes into `.mzidx` (stronger: m/z-sorted peaks); `file validate` checks the offsets |
| pyteomics `PreIndexedMzML` / `MzML` (offset index, `get_by_id`; Levitsky et al., J Proteome Res 2019), pymzML 2.0 `run[id]` (Kösters et al., Bioinformatics 2018) | scan iterators and random access; the user writes the XIC loop | reject as a model: that loop is the throw-away script; mzLab ships the loop with fixed maths |
| pyOpenMS `OnDiscMSExperiment` (Röst et al., PLOS ONE 2015) | metadata in memory, spectra and chromatograms read on demand | adopt: info from the index header, peaks only for the windows asked |
| mzR / MSnbase on-disk / Spectra backends (Bioconductor) | lazy backends behind one API, `header()` as a scan table | adopt: one scan table and one function set behind CLI, app and site |
| ThermoRawFileParser (Hulstaert et al., J Proteome Res 2020) | `xic` subcommand from a JSON list of m/z and tolerances, JSON out; `query` by scan numbers | adopt: list-in, JSON-out, and a batch file of queries (`raw batch`) |
| mzMLb (HDF5, Bhamber et al., J Proteome Res 2021), Toffee (HDF5, DIA), zarr chunked cubes | chunked binary arrays, chunk size trades random access for size | reject for now: a heavy dependency against the "light" rule; the `.mzidx` already gives m/z-chunked access |
| mzPeak (Parquet, Van Den Bossche et al., J Proteome Res 2025; HUPO-PSI, in development), DuckDB/Arrow scan tables | columnar long tables (file, scan, mz, intensity) queryable with SQL | adapt later: `--full` CSV columns follow a long-table layout (file, scan, rt, bin, value) so an agent can load them in DuckDB; no Parquet writer (no dependency) until mzPeak is final |
| MassQL (Damiani, Wang et al., Nat Methods 2025) | a query language over raw MS1/MS2 (conditions with tolerances, Boolean combinations), scan-level hits | adapt: the lessons are explicit tolerances, results that name the condition matched, and aggregation of scan hits into peaks; a full language is out of scope (an agent writes the arguments) |
| CAMERA (Kuhl et al., Anal Chem 2012), CliqueMS (Senan et al., Bioinformatics 2019) | group by RT, then pointwise Pearson of EIC shapes (CAMERA example cutoff 0.75), then isotope and adduct rules | adopt for `raw companions`: shape correlation inside the peak, apex offset, then a known mass-difference table; at unit mass rules are nominal and every edge keeps its r and apex shift |
| Currie (Anal Chem 1968; exact Poisson 1972), NIST counting limits, Ramebäck et al. (Appl Radiat Isot 2021) | L_C, L_D for Poisson counts with a paired blank; the normal approximation fails near zero background | adopt: §4; the zero-background case uses the exact 3.0 counts |
| Anthropic, "Writing effective tools for agents" (2025) | concise, token-efficient answers, meaningful identifiers, actionable errors, limits with a way to get more | adopt: summary first, `limit` 50 rows with `truncated` and `n_total`, `--full` writes a file and prints its path, errors with the fix |

Sources: psidev.info (mzML 1.1); pyteomics.readthedocs.io (mzml API); pyopenms.readthedocs.io (OnDiscMSExperiment); github.com/compomics/ThermoRawFileParser; Bhamber et al., J Proteome Res 20, 172-183, 2021 and biorxiv.org 10.1101/2020.02.13.947218 (mzMLb); pmc.ncbi.nlm.nih.gov/articles/PMC12604042 (mzPeak); doi 10.1038/s41592-025-02660-z (MassQL); bioconductor.org/packages/CAMERA; pmc.ncbi.nlm.nih.gov/articles/PMC6792096 (CliqueMS); nvlpubs.nist.gov/nistpubs/sp958-lide/164-166.pdf (Currie limits); anthropic.com/engineering/writing-tools-for-agents.

## 8. Regression plan
In part 2, §R (golden campaign on synthetic data, the agent's summary table as the golden, a Python reference, consistency tests).

## 9. Implementation chats (size, depends on)
| id | chat | size | depends on |
|---|---|---|---|
| AU-12 | `raw info` + `raw cache` (grid, quantum, label check with the #3678 detector, offset on request), lazy wasm module scaffold | S-M | #3678 MZML-SPECTRUM-TYPE, CONTRACT-ENGLISH |
| AU-13 | `ionstat` + `raw xic` (verdicts, series summaries, bounded output, provenance), golden campaign and reference | M | AU-12, #3681 QUANT-TARGETS (integration), AU-2 (sample metadata) |
| AU-14 | `raw batch` (JSONL, one process, hash once, entries in place) and the speed measurement | S-M | AU-13, #3682 SPEED-STUDY (SP2, SP6, SP15) |
| AU-15 | `raw ms2` (precursors, mean product spectra, losses, product XIC per precursor channel) | M | AU-13 |
| AU-16 | binned view + `raw scan` + `raw companions` (mass-difference table shared with AU-10) | M | AU-13; coordinate with AU-10 |
| AU-17 | `raw profile` (time course, shape class) | S | AU-13, AU-2 |
| AU-18 | `raw isotopes` (counts, `ion pattern`, Poisson z) | S-M | AU-16 |
| AU-19 | `--figure` small SVG/PNG grid for xic and profile | S | AU-13, D4 |

Order: AU-12, then AU-13 (the agent's priority "info+cache, xic"); AU-14 and AU-15 in parallel; AU-16; AU-17, AU-18, AU-19 in parallel. Each chat ships CLI + app + site with its parity test.

Risks: verdicts at the threshold flip with one ion (mitigated: the numbers and the rule are printed, the threshold is a parameter); a wrong quantum on processed files (mitigated: `quantum_support`, null with a warning); naming clash with the Thermo `.raw` extension in agent queries (help text says "raw scans of any file the app reads"); wasm size (own lazy module).

## 10. Owner decisions
- **D1 S/N model.** A (recommended): keep every existing S/N, add `sn_floor` and the count statistics in `raw` and as new columns where useful (AU-3); B: change the fallbacks of `peak1d` and `peakq_lrauto` to the one-ion floor everywhere (changes detection results and goldens).
- **D2 Wasm module.** A new lazy `mzlab-raw-wasm` (recommended) or growth of an existing module (needs a cap raise).
- **D3 Persistent binned cache.** Not now (recommended; in-memory view from the `.mzidx`), or a `.mzbin` saved format now.
- **D4 Figures.** SVG only (recommended: text, small, same bytes in wasm), or PNG too (the encoder of `msn_png.rs` moves to the core).
- **D5 Long-lived process.** `raw batch` JSONL (recommended); a socket daemon is rejected (state, security, the files-stay-local rule); the MCP server stays AU-11.
- **D6 Default thresholds.** `min_sn` 3, `min_points` 3, alpha 0.05 (L_C, L_D at 95 %), side windows as §4.
- **D7 Name.** `raw` (the agent's word, recommended) or `probe`.
