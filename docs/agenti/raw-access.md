# `raw info`, `raw cache`, `raw xic`, `raw scan`: what a raw file holds, whether a peak is there and what is in a window, for an agent (`raw-info/1`, `raw-cache/1`, `raw-xic/1`, `raw-scan/1`)

The first slice of the raw access study (`docs/studies/RAW_ACCESS_STUDY.md`, `RAW_ACCESS_STUDY_2.md` §1-2). Code: `crates/mzlab-core/src/rawinfo.rs` (all logic,
route `/api/rawinfo`), `rawxic.rs`, `rawxic_judge.rs`, `rawxic_json.rs` (route `/api/rawxic`), `crates/mzlab-raw-wasm`, `crates/mzlab-rawxic-wasm`, `crates/mzlab-cli/src/raw_cmd.rs`, `raw_cache_cmd.rs`, `raw_xic_cmd.rs`, `idx_cache.rs` (`peek`). Tests: `rawinfo_tests.rs`,
`rawxic_tests.rs`, `raw_cmd_tests.rs`, `raw_xic_cmd_tests.rs`, `crates/mzlab-cli/tests/raw_xic_cli.rs` (equal to `quant targets`), `tests/test_raw_info_wasm.py`, `tests/test_raw_xic_wasm.py` (the `raw` and `rawxic` `.wasm` vs the CLI, byte for byte),
  `crates/mzlab-desktop/tests/parity_routes.rs` and `parity_routes/raw_pol.rs` (app vs `.wasm`: the shared example is positive-only, `raw_pol.rs` is a file of alternating polarity read on every section, `+` and `-`; no `--sample` step on the app leg: a route answers over the index of one sample already chosen, so the sample choice is pinned CLI against `.wasm`).

## `raw info FILE... [--sample ID] [--probe N] [--offset-ion MZ[@RT]] [--dry-run] [--out PATH]`
- Per file `file`, `container`, `sha256`, `size_bytes`, `samples` (the sample read: `--sample`, else the first, logged in `provenance.choices`); per sample, from the index only:
  `polarity` (`+`, `-`, `unknown`), `rt` [min, max], `n_scans`, `ms_levels` [{`level`, `n_scans`}], `n_chromatograms`, `scan_types` (the scan channels of `file channels`:
  `channel`, `code`, `ms_level`, `polarity` +|-|both, `n_scans`, `rt_min`, `rt_max`), `mass_range` (lowest MS level), `probe`, `grid`, `ion_quantum`, `mass_offset`, `spectrum_type`.
- `probe`: `--probe N` scans (default 64, 1-4096, else `err.raw.probe`, exit 2) equally spaced over the lowest MS level; `grid`, `ion_quantum` and the data check read only them.
- `grid`: the first step of 1, 0.5, 0.25, 0.2, 0.1, 0.05, 0.02, 0.01 Da on which at least 90 % of the m/z spacings inside a scan are whole multiples (±0.02 Da):
  `kind` `nominal` (1 Da), `stepped` (a finer step), `continuous` (none), `unknown` (too few points); `phase_da` is the median offset of the m/z from the step.
- `ion_quantum`: `ionstat::quantum` on the probe intensities (rule `min_positive_or_step`, the one of the S/N; it reads, it changes no S/N). `cps` is set only when
  the quantum is accepted (`status` `ok`); `continuous` (a profile that is not ion counts: `warn.raw.noQuantum`) and `none` give `cps` null with the `candidate`.
- `spectrum_type`: `label` the declared type of the probe scans, `data` the detector of `spectrum_type.rs` (the runs of `Runs`): a centroid label on continuous runs
  is profile data, a profile label on sparse points is centroid data; `agree` false gives `warn.raw.labelMismatch` (file, label, data, `runs_share`).
- `mass_offset`: `{"status":"not_measured","how":"--offset-ion MZ[@RT]"}`, or with `--offset-ion` the `mzcal::estimate` evidence (`measured`, `da`, `evidence`).
- `warnings`, then `provenance`: program, version, commit, engine (index format, quantum rule), the command as given, the files with SHA-256, the parameters
  with defaults resolved, the automatic choices, the citation. No cache member: stdout is the same with and without `--cache` (stderr says built or reused).
- `--dry-run`: `raw-plan/1` (the query and each file with `size_bytes` and `cache` none|missing|reused|stale); no peak is read.
- Exit: 0; 1 a file could not be read (its entry is `{"file","error":<error/1>}`, the others answered, the document stays on stdout: a verdict command);
  2 usage; 4 a file is missing (one `error/1`); 5 unknown sample; 6 `--out` not writable.

## `raw cache FILE... [--sample ID] [--check] [--out PATH]`
- The entry `file index --cache` writes (`cli-agenti.md`, index cache), with what it holds: per file `status` built|reused, `key`, `sha256`, `bytes`, `n_scans`,
  `sections` [{`level`, `polarity`, `n_scans`, `n_peaks`}] in index order and `ion_quantum` (probe of the lowest MS level, as `raw info`).
- `--check` builds nothing: `reused` (a sound entry the next call reuses), `stale` (it would be rebuilt), `missing`; exit 1 unless all are reused (the document stays).
- Needs `--cache DIR` or `MZLAB_CACHE` (exit 2 with the hint otherwise: nothing is written where it was not asked); a cache it cannot write: exit 6.

## `raw xic FILE... --mz LIST --rt A,B [--da D] [--ms-level N] [--polarity +|-] [--mz-offset DA|auto:MZ[@RT]] [--set NAME=VALUE,...] [--sample ID] [--limit N] [--dry-run] [--out PATH]`
- Per file and mass one row of `rows` (the first sample of each file; `file`, `sample`, then the route's row): `verdict` `peak`, `trace`, `nothing` or `no_data` with its `rule` (the test, its values and thresholds), `flags`
  (`edge`, `no_quantum`, `no_background`, `one_side`, `offset_applied`). `area`, `height`, `sn`, `apex_rt`, `start_rt`, `end_rt`, `points`, `shape` are the cells of `quant targets` for the same file, mass and window (one integrator, `rawxic::tallest`).
- Counts (ion-count data, `ion_quantum.cps` set): `counts_gross`, `counts_net`, `counts_sigma`, `background_per_scan`, `l_c`, `l_d` (Currie levels at `alpha`), `z_counts`, `sn_floor` (S/N with a one-ion floor: sigma per scan = q * max(1, sqrt(background in ions))).
  `ul_counts` and `ul_area` bound a `nothing` row at confidence 1 - `ul_alpha` (the `alpha` of `--set`, so the name never says a level the row did not use). Without a quantum the counts are null and the pipeline `sn` decides (`no_quantum`). `sn_estimator` names the S/N of `sn`.
- Verdict: `peak` when N >= L_D, `sn_floor` >= `min_sn` and a run of `min_points` scans holds an ion; `trace` when only gross >= 2 and N >= L_C hold; else `nothing`; `no_data` when no scan is in the window. Nothing already measured changes: `sn` is the S/N of every other command.
- A question holds at most 1000 masses (`rawxic::MAX_MASSES`): more is `err.raw.xic.many` on the route and in the CLI (exit 2; pinned in the CLI and wasm tests), never a cut list; `--limit` only cuts the printed rows (`truncated`, `n_total`), the summary stays whole.
- `--set`: `min_sn` 3, `min_points` 3, `alpha` 0.05, `side_min` 0.3 min, `gap_min` 0.05 min (the background is read from the same trace beside the window); out of range: `err.raw.xic.param`, exit 2. `provenance.params` holds every value used, `provenance.choices` the sample, the quantum and the background.
- `summary`: per mass the files answered, the rows per verdict, `max_sn_floor` with its file and the median `area` of the `peak` rows; always whole. `--limit` (default 50) caps `rows`: `n_total` and `truncated` say what was cut.
- `--dry-run`: `raw-plan/1` with the query. Exit as `raw info`: 0, 1 a file could not be read or its rows are inconsistent (entry with `error`, key `err.raw.xic.rows`; never exit 7, which is the panic hook's), 2 usage, 4 missing, 5 unknown sample, 6 `--out` not writable.

## `raw scan FILE... --rt A,B [--side MIN] [--gap MIN] [--bin nominal|0.1] [--mz-range LO,HI] [--top N] [--channel ID] [--ms-level N] [--polarity +|-] [--mz-offset DA|auto:MZ[@RT]] [--set NAME=VALUE,...] [--sample ID] [--limit N] [--full DIR] [--dry-run] [--out PATH]` (`raw-scan/1`)
- What is in the spectrum here that is not in the background: the mean spectrum of the scans of the window minus the mean of the scans of its two sides (as `raw xic`: W = the window, at least `--side`, ending `--gap` away; the side scans are pooled), per bin (`nominal` = round(m/z), `0.1` = round(m/z x 10)).
- Per file and bin one row (`file`, `sample`, then the route's row), highest `net` first: `mean`, `background`, `net`, `ratio` (null with `background_zero` true when the background is 0), then ion counts (`counts_net` per window scan, `z_counts`) and `verdict`: `peak` when `z_counts` >= `min_z` and `counts_net` >= `min_net`; `trace` when net > 0 and `z_counts` >= the one-sided z of `alpha`; else `nothing`; null without an ion quantum.
- `--top N` (default 20, 0 = every bin) cuts the rows by net, `--limit` (default 50) the printed ones; `--full DIR` writes every bin to `DIR/raw-scan.csv` (`file,sample,bin,...`; `full` holds path, rows, sha256) and prints no rows; exit 6 when it cannot be written. `summary` per file: scans in the window and each side, mean TIC of both, `n_bins`, `bins_with_peak`, `bins_with_trace`; always whole.
- `--set`: `min_z` 3, `min_net` 1, `alpha` 0.05, `side_min` 0.3 min, `gap_min` 0.05 min (`--side` and `--gap` are the last two); out of range: `err.raw.scan.param`, exit 2. `--channel` takes an MS or DDA channel (`err.raw.scan.channel` otherwise, `err.canali.unknown` exit 5). `provenance.params` holds every value, `provenance.choices` the background and the quantum.
- `warn.raw.oneSide`: a window at the start or end of the run has one side only (`side` left|right|none: none = a background of zero everywhere). `--dry-run`: `raw-plan/1`. Exit as `raw xic`. Route `/api/rawscan`, lazy `.wasm` `rawscan` (`mzlab-rawscan-wasm`).

## Site, app, CLI
- The page asks the route `/api/rawinfo?probe=N&offset_mz=M&offset_rt=T` of a slot: the site's worker sends it to the lazy `.wasm` `raw` (`mzlab-raw-wasm`, `rust-raw.js`,
  over the index in OPFS; the engine `.wasm` has no room), the app answers it natively (`rawinfo::handle`); the CLI prints the route's members inside each sample. `raw xic` is the route `/api/rawxic`, answered by the lazy `.wasm` `rawxic` (`mzlab-rawxic-wasm`; `raw` keeps its size); `raw scan` is the route `/api/rawscan`, answered by the lazy `.wasm` `rawscan` (`mzlab-rawscan-wasm`); `/api/specsum` (`spectrum get --sum`, `data-channels.md`) is answered by the same `rawxic` module (`spec_sum`); `raw cache` is native only: the site keeps its indexes in OPFS, not in a cache folder.
- Choices kept where the study left them open: no change to any S/N (D1), a lazy module of its own (D2, as the study recommends: in the engine it was over the cap), no `.mzbin` (D3), the command family `raw` (D7).
