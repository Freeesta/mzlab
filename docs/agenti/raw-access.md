# `raw info`, `raw cache`: what a raw file holds, for an agent (`raw-info/1`, `raw-cache/1`)

The first slice of the raw access study (`docs/studies/RAW_ACCESS_STUDY.md`, `RAW_ACCESS_STUDY_2.md` §1-2). Code: `crates/mzlab-core/src/rawinfo.rs` (all logic,
route `/api/rawinfo`), `crates/mzlab-raw-wasm`, `crates/mzlab-cli/src/raw_cmd.rs`, `raw_cache_cmd.rs`, `idx_cache.rs` (`peek`). Tests: `rawinfo_tests.rs`, `raw_cmd_tests.rs`,
`tests/test_raw_info_wasm.py` (the `raw` `.wasm` vs the CLI, byte for byte), `crates/mzlab-desktop/tests/parity_routes.rs` (app vs `.wasm`).

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

## Site, app, CLI
- The page asks the route `/api/rawinfo?probe=N&offset_mz=M&offset_rt=T` of a slot: the site's worker sends it to the lazy `.wasm` `raw` (`mzlab-raw-wasm`, `rust-raw.js`,
  over the index in OPFS; the engine `.wasm` has no room), the app answers it natively (`rawinfo::handle`); the CLI prints the route's members inside each sample. `raw cache` is native only: the site keeps its indexes in OPFS, not in a cache folder.
- Choices kept where the study left them open: no change to any S/N (D1), a lazy module of its own (D2, as the study recommends: in the engine it was over the cap), no `.mzbin` (D3), the command family `raw` (D7).
