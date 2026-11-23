# Raw data access for agents, part 2: the `raw` commands and the regression plan

Part 1 (`RAW_ACCESS_STUDY.md`) holds the problem, the evidence, the gap table, the ion statistics (`ionstat`, §4), the cache, parity and the owner decisions. This file specifies each command (proposal, not implemented) and the regression plan (§R). Roadmap ids: AU-12..AU-19.

## C. Conventions shared by every `raw` command
- **Inputs**: `FILE...` (mzML or any file the app reads, a `.mzidx`, folders as `quant targets` takes them), `--sample ID` for multi-sample files, `--samples samples.json` (`samples/1`, from `sample table` or `sample import` AU-2) for series, time and role. Without `--samples` the roles proposed from names are used and logged as a choice.
- **Selectors**: `--polarity +|-`, `--ms-level N`, `--channel ID` (a `channels/1` id: `ms:<key>`, `dd:2`, `srm:...`), `--rt A,B` (minutes). An unknown sample, channel or precursor: exit 5 with the near ids in the hint.
- **Mass windows**: `--mz LIST` with `--da D` (default 0.5) or `--bin nominal|0.1`; `--mz-offset DA|auto:MZ[@RT]` (the `mzcal` spec of `trace xic`).
- **Output**: JSON on stdout (schema `raw-<cmd>/1`), rows capped by `--limit N` (default 50; `truncated`, `n_total` say what was cut, the summary is always whole); `--full DIR` writes every row as CSV (stable header, long layout `file,sample,...`), prints `{"full":[{"path","rows","sha256"}]}` inside the answer and never prints the rows; `--format csv` prints the capped rows as CSV instead of JSON.
- **Common keys**: `schema`, `query` (the question as resolved: masses, windows, selectors), `files` (one entry per file in input order), `summary`, `series` (per-series summaries when sample metadata exist), `warnings` ([{`key`, `params`}], keys `warn.raw.*`), `provenance` (part 1 §6).
- **Exit codes**: 0 answered (any verdict); 1 at least one file unreadable (the others answered, its entry has `error`); 2 usage; 4 missing input; 5 selector not found; 6 output write; 7 internal.
- **`--dry-run`**: prints the resolved query and the files with their cache status (`raw-plan/1`), reads no peak.
- **Defaults**: printed by `flow defaults`-style `raw <cmd> --defaults` (`step-params/1` shape, AU-5), the same names as `--set`.

## 1. `raw info` (AU-12)
`mzlab-cli raw info FILE... [--sample ID] [--probe N] [--offset-ion MZ[@RT]]`

What it answers: "what kind of data is this, and can I count ions in it?"

Per file: `container`, `sha256`, `samples`; per sample:
- `polarity` (list), `rt` [min, max], `n_scans`, `mass_range`;
- `scan_types` (from `channels/1`): [{`channel`, `code` EMS|Q1|EPI|SIM|MRM|..., `ms_level`, `n_scans`, `rt_min`, `rt_max`}];
- `grid` {`step_da` (median spacing of consecutive m/z inside scans), `phase_da` (median fractional part of the m/z, folded), `kind` nominal|stepped|continuous};
- `ion_quantum` {`cps`, `support` (share of positive values on multiples), `method` "min_positive", `status` ok|continuous};
- `mass_offset`: with `--offset-ion` the `mzcal` evidence, else `{"status":"not_measured","how":"--offset-ion MZ[@RT]"}`;
- `spectrum_type` {`label` (centroid|profile|mixed, from the file), `data` (the #3678 detector on N probed scans: centroid|profile), `agree` true|false}; on disagreement `warn.raw.labelMismatch` with the counts behind the decision.

Example (one file, trimmed):
```json
{"schema":"raw-info/1","files":[{"file":"S01.mzML","sha256":"9f2c...","samples":[{"id":"0","polarity":["+"],
 "scan_types":[{"channel":"ms:+ Q1 MS","code":"Q1","ms_level":1,"n_scans":2400,"rt_min":0.01,"rt_max":24.0}],
 "grid":{"step_da":1.0,"phase_da":0.3,"kind":"nominal"},"ion_quantum":{"cps":200.0,"support":0.998,"method":"min_positive","status":"ok"},
 "mass_offset":{"status":"not_measured","how":"--offset-ion MZ[@RT]"},"spectrum_type":{"label":"profile","data":"centroid","agree":false}}]}],
 "warnings":[{"key":"warn.raw.labelMismatch","params":{"file":"S01.mzML","label":"profile","data":"centroid","runs_share":0.02}}],"provenance":{"...":"..."}}
```
Errors: unreadable file exit 1 (entry `error`), unknown sample exit 5.

## 2. `raw cache` (AU-12)
`mzlab-cli raw cache FILE... [--cache DIR] [--sample ID] [--check]`

Builds or reuses the `.mzidx` entries (part 1 §5; equal to `file index --cache`). `--check` builds nothing and says which entries are valid. Output `raw-cache/1`: per file `status` built|reused|stale|missing, `key`, `bytes`, `n_scans`, `sections` [{`level`, `polarity`, `n_peaks`}], `ion_quantum`. Without `--cache` and `MZLAB_CACHE`: exit 2 with the hint (nothing is written where it was not asked).

## 3. `raw xic` (AU-13)
`mzlab-cli raw xic FILE... --mz LIST [--rt A,B | --targets PATH] [--da D | --bin nominal|0.1] [--samples PATH] [--set min_sn=3,min_points=3,alpha=0.05,side_min=0.3,gap_min=0.05] [--figure PATH]`

What it answers: "at this mass and this time, is there a peak, in each file?" `--targets` takes the `quant targets` table (id, mz, rt_min, rt_max, polarity) for many masses with their own windows.

Per file x mass row (`rows`): `file`, `sample`, `series`, `time`, `role`, `id`, `mz`, `window` [lo, hi] Da, `rt_window` [a, b], `verdict` peak|trace|nothing|no_data, `rule` (the numbers and thresholds, part 1 §4), `apex_rt`, `height`, `area`, `sn` (pipeline estimator, named in `sn_estimator`), `sn_floor`, `baseline`, `start_rt`, `end_rt`, `points`, `shape`, `counts_gross`, `counts_net`, `counts_sigma`, `l_c`, `l_d`, `z_counts`, `background_per_scan`, `ul95_counts`, `ul95_area` (null unless `nothing`), `flags` (`edge`, `saturated`,
`offset_applied`, `no_quantum`).

`summary`: per mass, `n_files`, counts per verdict, `max_sn_floor` with its file, `median_area` of the `peak` rows. `series`: per series and mass, `n`, counts per verdict, `times_with_peak`, `median_area`, `exclusive` (true when the mass is `peak` in this series and `nothing` in every other series and in the blanks: the "present only in series A" question as one field).

Example row:
```json
{"file":"A3.mzML","sample":"0","series":"A","time":3.0,"role":"sample","id":"P1","mz":250.0,"window":[249.5,250.5],"rt_window":[6.0,7.0],
 "verdict":"trace","rule":"N 4.8 >= L_C 2.0 but < L_D 6.7; sn_floor 1.0 < 3","apex_rt":6.42,"height":200.0,"area":480.0,"sn":17.3,
 "sn_estimator":"smoothed height over MAD of first differences (RMS fallback)","sn_floor":1.0,"counts_gross":6,"counts_net":4.8,"counts_sigma":2.5,
 "l_c":2.0,"l_d":6.7,"z_counts":1.9,"background_per_scan":0.1,"ul95_counts":null,"ul95_area":null,"flags":[]}
```
Same maths: `area`, `height`, `sn`, `start_rt`, `end_rt`, `points`, `shape` are the `quant targets` numbers for that target (test). The verdict never changes them.

`--figure PATH`: an SVG (D4) grid, one row per series, one column per time, each cell the window's trace with the region shaded and the verdict letter; at most 12 x 12 cells, else `warn.raw.figureCut` and the first cells.

## 4. `raw scan` (AU-16)
`mzlab-cli raw scan FILE --rt A,B [--side MIN] [--gap MIN] [--bin nominal|0.1] [--mz-range LO,HI] [--top N] [--channel ID]`

What it answers: "what is in the spectrum here that is not in the background?" Mean spectrum of the scans in [A, B] minus the mean of the two sides (part 1 §4 windows), per bin. Rows (sorted by net intensity, `--top` 20): `bin`, `mz_center`, `mean`, `background`, `net`, `ratio` (mean / background, `null` when background is 0, with `background_zero: true`), `counts_net`, `z_counts`, `verdict` (peak = z >= 3 and net >= 1 ion per scan on average; else trace or nothing).
`summary`: `n_scans`, `n_side_scans`, `tic_mean`, `tic_side`, `bins_with_peak`. A window with no side scans (start or end of the run): one side only, `warn.raw.oneSide`.

## 5. `raw companions` (AU-16)
`mzlab-cli raw companions FILE... --mz M --rt A,B [--mz-range LO,HI] [--min-r 0.8] [--max-shift-scans 1] [--list PATH]`

What it answers: "which other masses rise and fall with this one?" For each bin in the range (default M - 100 to 2M + 50), the trace over [A, B] is correlated with the trace of M inside M's peak region (pointwise Pearson, CAMERA-style); kept when r >= `min_r`, the apex is within `max_shift_scans`, and the companion's net counts pass L_D.
Rows: `mz`, `delta_da` (companion - M, nominal), `r`, `apex_shift_scans`, `ratio_to_m` (net area ratio), `z_counts`, `relation` (from the mass-difference table, or `unknown`), `relation_set` (all relations that fit, never one forced). With several files: `r_across_files` (Pearson of net areas over the files where M is `peak`, at least 3) and `n_files`.

Mass-difference table (nominal, ESI+, editable as a `liste/1` user list, shared with AU-10 `features.companions`): isotopes +1, +2; adducts relative to [M+H]+: +17 (NH4), +22 (Na), +38 (K), +41 (H + acetonitrile), +63 (Na + acetonitrile); dimer 2M - 1 ([2M+H]+ from [M+H]+), [2M+Na]+ 2M + 21; in-source losses -17 (NH3), -18 (H2O), -28 (CO), -42 (C2H2O), -44 (CO2), -46 (HCOOH). ESI- uses its own list (Cl +34 from [M-H]-, formate +46, acetate +60, dimer 2M + 1).

Verdict per row: `companion` (r >= min_r, shift within limits, counts significant), `weak` (r >= 0.6 or counts below L_D), dropped below. An ion at M + 1 with r < 0.5 is never reported as an isotope (part 1 test case). Nothing is merged or removed.

## 6. `raw profile` (AU-17)
`mzlab-cli raw profile FILE... --mz M --rt A,B --samples PATH [--quantity area|height]`

What it answers: "how does this mass change with time in each series?" It runs `raw xic` (same rows, reused) and orders them by `time` per series. Rows: `series`, `time`, `file`, `verdict`, `value`, `sigma` (Poisson, from `counts_sigma` x q x scan time), `ul95` for `nothing`. Per series: `shape` with its evidence:
- `absent`: no `peak`;
- `sporadic`: `peak` in fewer than half of the times and not consecutive;
- `rise`: Spearman rho >= 0.8 and the last value within 20 % of the maximum;
- `decay`: rho <= -0.8 and the first value within 20 % of the maximum (washes and carryover fall here; `fold_per_step` median ratio printed);
- `rise_fall`: the maximum at an inner time and both ends below 50 % of it (an intermediate);
- `flat`: max / min <= 1.5 over the `peak` points;
- `irregular`: none of the above.
The thresholds sit next to the class (`shape_rule`). Blanks and washes are their own series (role from `samples/1`).

## 7. `raw isotopes` (AU-18)
`mzlab-cli raw isotopes FILE --mz M --rt A,B [--formula F --adduct A] [--bin nominal]`

What it answers: "do M+1 and M+2 fit this formula, within counting error?" Net counts of M, M+1, M+2 over M's peak region (sides subtracted). Observed R1 = N1 / N0, R2 = N2 / N0 with σ by the delta method (sqrt(σ1²/N0² + N1² σ0² / N0⁴)). With `--formula`: the expected pattern from `ion pattern` (`ion_pattern/1`) summed to the same bins; `z1`, `z2` = (observed - expected) / σ; verdict per ratio `consistent` (|z| < 2), `inconsistent` (|z| >= 3), `undetermined` (else, or N1 < 10 counts: `too_few_counts`).
Without a formula: `carbon_range` from R1 at 1.1 % per carbon (±2σ), and `m2_hint` (`cl_br_s_possible` when R2 > 0.2 R1 + 3σ) as evidence only. Co-eluting interference is flagged when the M+1 trace correlates below 0.8 with M (`interference_possible`).

## 8. `raw ms2` (AU-15)
`mzlab-cli raw ms2 list FILE [--rt A,B] [--tol D]`
`mzlab-cli raw ms2 spectrum FILE --precursor P [--rt A,B] [--ce E] [--top N]`
`mzlab-cli raw ms2 xic FILE --precursor P --product Q[,Q...] [--rt A,B]`

What it answers: "which precursors were fragmented, what are their product spectra, and does a product trace follow the precursor?"
- `list`: precursors grouped within `--tol` (0.5 Da at unit resolution, 10 ppm at high resolution; from `items/1` `ms2_exps` and `channels/1`): `precursor_mz`, `channel`, `code` (EPI, MS2, dd), `ce` (or spread), `n_scans`, `rt_min`, `rt_max`, `tic_median`, `dd` (data-dependent or targeted).
- `spectrum`: mean product spectrum of the precursor's scans in the window (the scans of its channel only, never another precursor's), binned; rows `mz`, `mean`, `share` (% of the base peak), `loss_da` (P - mz), `loss_name` (from a neutral-loss list shared with `raw companions`: 17, 18, 28, 42, 44, 46, ...), `n_scans_present`, `counts_net`. `summary`: `n_scans`, `base_peak`, `losses_found`.
- `xic`: the product-ion XIC inside the precursor's channel, with the `raw xic` rows and verdicts per product, plus `r_to_precursor` (Pearson with the MS1 trace of P in the same window when MS1 scans exist).
Exit 5 when the precursor matches no channel (hint: the nearest three).

## 9. `raw batch` (AU-14)
`mzlab-cli raw batch QUERIES.jsonl [--cache DIR] [--workers N] [--out PATH]`; `-` reads stdin.

One JSON object per line: `{"id":"q1","cmd":"xic","files":["A1.mzML"],"mz":[250],"rt":[6,7]}` with the same keys as the command's flags (without the dashes). One answer line per query, in input order: `{"id":"q1","exit":0,"answer":{...}}` where `answer` is the command's JSON byte for byte; a failing line gives its `exit` and `error/1` and the next line runs. Files are hashed once and indexes opened once per process (LRU, cap shared with the block cache).
A final line `{"schema":"raw-batch/1","n":N,"exits":{...},"files_opened":K,"seconds":T}`. The process ends at end of input (no daemon, D5). An agent keeps it alive by writing to its stdin and reading its stdout, one line at a time; the order of answers is the order of queries.

## R. Regression plan
- **Golden campaign on synthetic data** (`crates/mzlab-core/tests/common_raw/`, written with `MzmlWriter` as `lr_fullscan` does; no real data): series A (5 times) and series B (5 times) in two placeholder matrices, a blank, 3 washes after the top sample; product P1 only in series A; P2 with companions at +22 and +17 Da (same shape, r > 0.95) and an unrelated ion at P2 + 1 with another shape; sporadic single ions (Poisson 0.05 per scan) at P3; carryover of P2 at 2 %, 0.4 %, 0.08 % in the three washes; quantum 200 cps;
  offset +0.3 Da in one file; one file labelled profile with centroid data.
- **The agent's summary table as the golden**: one JSON (`tests/golden/raw_campaign.json`) with each question, the command and the expected verdict per file and per series. P1: `peak` in A, `nothing` in B with `ul95`, `exclusive` true for A. P2: companions with relations `adduct_na`, `adduct_nh4`; the P2 + 1 ion rejected on r. P3: `trace` or `nothing`, never `peak` (the two-ion case of part 1 §2.1). Washes: `decay` with `fold_per_step` near 5. `raw info`: `labelMismatch` on the mislabelled file, quantum 200. Any verdict change fails the test.
- **Reference implementation**: `tools/raw_reference.py` decodes the synthetic mzML with the standard library (base64 + zlib), sums to the same half-open bins and recomputes the count statistics: the agent's throw-away script written once, tested, used as the parity reference of the maths (Python for tests only, never in the site).
- **Consistency tests**: `raw xic` area/height/sn equal to `quant targets` and to `trace xic | peak auto-lr` for the same target (bytes); binned view equal to index windows; answers identical with and without `--cache`, with 1 and 4 workers, single command and `raw batch`, CLI and `.wasm`.
- **Verdict edges**: unit tests of `ionstat` on hand-made counts on zero background: 0 and 1 ion `nothing`, 2, 3 and 5 ions `trace` (`sn_floor` under 3), 3 ions per scan over 5 scans `peak`; the same at background 0.05 and 2 ions per scan; a continuous (non-integer) trace gives `no_quantum` and the pipeline estimator only.
