# Choosing samples of a `.wiff` (listing, `--sample`, page dialog)

After a `.wiff` (+ `.wiff.scan`), or several, is chosen or dropped, mzLab lists the samples the file holds and the user ticks which to open (one, several, all), like SCIEX Analyst `File > Open Data File`. Sources of the facts: `wiff.md`, `wiff-full-spec.md`, `wiff-variants.md`, `memory-limits.md`, `wasm-caps.md`, `cli-agenti.md`, `native-reading.md`. The `.wiff` container structure (per sample, file-global, per method) is in `wiff-full-spec.md`.

## What exists
- Engine: `mzlab_core::readers::list_samples(&mut dyn Reader) -> String` (`crates/mzlab-core/src/readers/listing.rs`) builds the format-neutral listing `samples_list/1`, a superset of `readers::samples_json` (`id`, `name`, `scans`, `size`, `why`, `refused`, `unindexed`). A `.wiff` is listed from the container and the `Idx` rows only (count, times, TIC, section bytes); the `.wiff.scan` is not opened, so the cost does not depend on its size. No hashes, no log text, no operator.
- Large `.wiff`: `Ole::open_at` (`ole.rs`) and `wiff::container` open by sector, so a `.wiff` above the whole-read limit lists and opens; DAD streams are loaded on demand (`Wiff::load_dad`). Test `crates/mzlab-core/tests/wiff_by_sector.rs`; listing test `wiff_listing.rs`.
- wasm: `native_samples(.., listing = true)` returns `samples_list/1` (same bytes as the app, `crates/mzlab-wasm/src/natives_samples.rs`; app: `crates/mzlab-desktop/src/natives.rs`).
- CLI: `mzlab-cli sample list FILE... [--json] [--probe]` (`crates/mzlab-cli/src/samples_cmd.rs`). One `samples_list/1` per file on stdout, progress and warnings on stderr, read-only. `--probe` compares the `.wiff.scan` length with where its indexed part ends and adds `probe {scan_bytes, indexed_end, status}`.
  - `--probe` is still accepted: the check is now always on and adds `consistency` (not `probe`) only when the pair disagrees (`wiff::completeness`: `.wiff.scan` shorter than its index = `short`; sections after the indexed part, found by their marker = `unlisted`; plain padding is ok), with the sizes, `samples_listed`, `sections_unlisted`, the age of the newest member and the threshold `recent_s` (`wiff::RECENT_S` = 10 s, a documented constant shared with the growing check, logged in the warning on stderr).
  - Exit codes: 0 listed; 1 a file or sample refused, a cut `.wiff.scan` or sections the index lacks (the listing is still printed); 8 the same with a member modified in the last 10 s (still being written); 2 usage; 4 missing or unreadable input. The site lists the same index bytes and has no mtime to poll; the verdict is a CLI check on files on disk, no wasm route.
- `--sample LIST` and `sample select` (below), page dialog «Choose samples» (below).
- Other formats: a Waters `.raw` folder holds several functions but one sample (`waters.md`); Agilent `.D`, Thermo `.raw`, Bruker `.d`, Shimadzu `.lcd`, mzML and mzXML are one sample each. So only a `.wiff` has several elements in `Reader::samples()`; the route and dialog are generic and take any `samples_list/1` with several samples.

## What the reader knows per sample without the `.scan`
- Scan count (rows of `Idx`; empty rows have 0 points, an injection that acquired nothing stays a sample with 0 scans), retention times, TIC per scan (apex time and size), section bytes, experiment kinds and polarity from the method (`ExperimentHeader` polarity, MRM marker, `MassRangeEx` for SIM, scan type codes), method name, DAD present or not. The sample name is the first counted string of `SampleDABE/DATA` (`wiff/names.rs`).
- Not read by any code: per-sample acquisition date and time (the mzML `startTimeStamp` is the creation date of the whole file), vial and plate position, batch name, operator (`Log`, which stays out of any listing). Candidate places are further strings of `SampleDABE/DATA`, `SampleTable`/`DabsInfo` and `Log`; `python3 tools/wiff_dump.py FILE.wiff --all` on a real file in the private data repo finds out. Until then the dialog has no date or vial column (an empty column is never filled with a guess).
- The listing cannot see a damaged or cut block: it shows the evidence it has (index counts, the `unindexed` and truncated-`.scan` flags) and the exact `losses` appear after opening (`warn.wiff.damaged`, `Lettore::missing`). A sample is never refused by the listing (rules R1/R2 of `wiff.md`).

## Cost
Measured natively on synthetic pairs (`tools/wiff_grande.py`, page cache warm):
- Opening with the block heads of the whole `.scan` reads about 1.0x the dataset (2.07 s for a 427 MB pair, 442 read calls; about 5 ms per MB, so minutes for 12 GB and slower through `File.slice`)
- The listing from the `.wiff` alone took 4 ms for the same pair (40 samples, 80 000 rows) and 4 ms with a 12 GB sparse `.scan`. The `Idx` streams are 54 B per scan.
- Not measured: names (fixtures carry no `SampleDABE/DATA`), method-derived type and polarity, the listing through `File.slice` in a browser or from a cold disk. A real `.wiff` with one 15 MB DAD stream per sample can be far larger than its `.scan`, so the `.wiff` is not always small.

## `--sample LIST`
- `LIST` is sample ids and ranges: `1,3,5`, `2-4`, `5, 2-4,3`. The ids are those of `sample list` (not positions), each taken once, the answer in the order of the file. One code for all routes: `mzlab_core::readers::selection` (`parse`, `resolve`, `selection_json`), also exported by the engine .wasm as `sample_selection(present_ids, text)` (same bytes; `tests/test_sample_selection_parita.py`).
- Only the container and the index are read to check the list; samples not chosen are never read.
- An id the file does not have stops the command before anything is written: `err.sample.unknown` names the missing ids and the ids present, exit 5. A text that is not a list (`1,,2`, `3-1`, `x`): `err.sample.list`, exit 2. A dataset with one sample ignores the list; when no input has several samples the command stops with `err.sample.single` (exit 2), so a typo is never silent.
- `sample select FILE --sample LIST [--json]`: the dry run; `{"schema":"sample-select/1","samples":[1,3]}` (ids on one line without `--json`); `error/1` on stdout with `--json` on failure; exit 0, 2, 4, 5.
- `sample select FILE --sample LIST --ranges [--json]`: the byte ranges of the `.wiff`/`.wiff.scan` to copy for those samples (same plan as the site's `sample_ranges`; `wiff-range-copy.md`).
- `sample select FILE --default [--budget-mb N] [--json]`: the samples the page dialog ticks first (`readers::selection::suggest`, the site's `sample_default`): all when their section bytes are at most a quarter of the budget (`SMALL_SHARE`, default 1024 MB), none otherwise. Exactly one of `--sample` and `--default`.
- `convert INPUT... --out DIR --sample LIST`: one `NAME-ID.mzML` per chosen sample, byte for byte the file a run without `--sample` writes for that sample (a single chosen sample keeps its `NAME-ID` name).
- `channels FILE --sample LIST`: one `channels/1` line per sample, in file order, each the bytes of its own single-sample run; the whole list is checked first. `trace chrom|xic|spectrum`, `dad` and `file info` read one sample (`--sample ID`; `file info` takes a list).
- `features FILE... --sample LIST`: a file with several samples gives one series member per chosen sample (progress lines name it `FILE@ID`); single-sample files stay one member; without `--sample` a multi-sample file is its first sample.
- The mzML of a chosen sample is the same bytes as in a full run, so no identity changes. Not built: a `samples` property of a `flow run` input (plan inputs already name a sample as `NAME@ID`) and a partial-identity mark for partial opens.
- Tests: `crates/mzlab-cli/src/sample_select_tests.rs` (synthetic `tools/wiff_grande.py` pair; python3 is part of the test), `readers/selection.rs` unit tests.

## Page dialog «Choose samples»
- `mzlab/web/wiff-choose.js` (lazy, `loadScript`), called by `wiffToMzml` (`shell.js`) once per loading, before any sample is read: the pairs of the loading are listed with `native_samples(.., listing = true)`, ONE modal `<dialog>` shows one table per file (tick, name, number, type, scans, size), then only the ticked samples become files of the list. Skipped samples are never indexed. Lang keys: `mzlab/web/lang/en/pick.js` (`pick.*`); styles `.wiffchoose` in `app.css`.
- No sample logic in JS: the list and sizes come from the engine, the first ticks from `sample_default`, the «numbers» field (`1,3,5`, `2-4`) from `sample_selection`. Worker ops `natdefault`, `natselect` (read-free), page method `nativeAsk`.
- Trigger: the dialog shows only when some file has several samples; a file with one sample opens straight away. Settings > Data > «Choose samples of a file»: «When a file has several» (default), «Always ask» (also a one-sample file), «Never, open all»; key `mzlab.pick` in localStorage (absent = default).
- Default ticks: all when the sections add up to at most a quarter of the engine's memory budget, none otherwise; Open stays disabled until something is ticked, nothing is blocked (the user decides).
- One dialog for all pairs of a loading; Cancel opens nothing from the loading and says so in a note; the choice is not remembered per file; no name filter, no per-file select buttons (global «Select all», «Select none», numbers field). A reopened file with a subset chosen skips the OPFS cache check (it would answer with every sample). A pair the readers module takes (`variante`) is filtered by the ids of its probe. A `.wiff` without its `.wiff.scan` keeps the existing wait and error (`load.wiffWait`, `err.file.wiffNoScan`).
- The desktop app runs the same page dialog, fed by the native route (same bytes as the wasm one). On the site a chosen subset is copied to OPFS by byte ranges only (`wiff-range-copy.md`); the memory limit applies to what is opened.
- Entry points all end in the same dialog: the Files picker, drag and drop, a dropped or picked folder (`cartella-carica.js`), and in the app the native open dialog or a file opened from the OS (`opening.rs`).
- Tests: `tests_e2e/e2e_wiff_choose.py`.

## Not built
- Per-file remembered choice, name filter, date, vial and plate columns (needs the strings above), a file-level chooser for the datasets of a dropped folder.
- Partial identity of a partial open (mzML `source-identity`, `.mzidx`, cache keys): decided by the owner if ever needed.

## Wasm cap
The per-sample summary sits where `samples_json` already is (`lettori-sciex` and `motore`); caps and measured sizes are in `tools/moduli/*.toml` and `wasm-caps.md`. New code reuses the existing `Idx` and method readers instead of adding parsers.

## Risks
- The listing can disagree with what opening finds (damaged blocks): the dialog says the numbers are per the file index and opening reports losses.
- Type and polarity from the method are guesses for samples with no method or a borrowed one (`method_no` rules, `wiff-full-spec.md`): shown with the reader's «borrowed» mark, never asserted.
- `file inventory` (`wiff.list/1`) is the heavy forensic inventory (hashes of every stream); `sample list` is the light listing for a chooser; they are separate commands (`wiff-full-spec.md`).
