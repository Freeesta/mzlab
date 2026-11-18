# «Choose samples» for `.wiff` and other multi-sample files: feasibility study and proposal (no feature code)

Request of the owner: after choosing or dropping a `.wiff` (+ `.wiff.scan`), or several, mzLab shows the samples the file holds and the user ticks which to open (one, several, all), like SCIEX Analyst 1.6.x `File > Open Data File`. Today `shell.js::wiffToMzml` lists the samples with `rb.nativeSamples` and makes one list entry per sample, all of them.
Sources: the code at `origin/main` 0feb211, `wiff.md`, `wiff-writer.md`, `wiff-varianti.md`, `memory-limits.md`, `wasm-tetti.md`, `cli-agenti.md`, `native-reading.md`, the description and file list of PR #3398 (its diff was NOT read), and measurements on synthetic pairs made for this study. No real file, no Analyst, no wasm toolchain here: what could not be done is in the last section. Marks: CERTAIN (repo or measured), MEMORY (public knowledge recalled, not checked), UNKNOWN.

## Verdict
- Feasible, small, and mostly already there. The engine already has a format-generic listing of what a file holds: `readers::samples_json` (`crates/mzlab-core/src/readers/listing.rs`; `id`, `name`, `scans`, `size`, `why`, `refused`, `unindexed`), served by wasm `native_samples` and the app. Only its COST and its CONTENT are not right for a chooser (below).
- The cost problem is real and measured: today's listing reads the block heads of the whole `.wiff.scan`, so it costs time proportional to the `.scan` (2.07 s for a 427 MB pair here; about 1 minute for 12 GB, extrapolated). A listing from the `.wiff` alone (the `Idx` rows already hold count, times, TIC and section sizes) took 4 ms for the same pair and does not depend on the `.scan` size at all.
- Recommendation: a generic engine route `list_samples` (a format-neutral JSON, `samples_list/1`) + `mzlab-cli samples list FILE... --json` + `--sample 1,3,5` on the commands that read a file, then ONE page dialog «Choose samples» shown only when a dataset holds more than one sample. Wiff is the only format that needs it today; the route and the dialog are written generic so that a format with several samples gets it for free.
- PR #3398: close it (red, conflicts, 53 files, one cap raise of +2.5 %) and reuse its ideas in small PRs; details in section 3.

## 1. How Analyst 1.6.x does it, and what we know
CERTAIN (our reader): the sample list of a `.wiff` is `SampleSubtree/SampleIdxTable` (16-byte rows: sample number, position in the `.scan`), the per-sample storages are `SampleSubtree/SampleN/{Idx, SampleDABE/DATA, Log, Devices, DAD}`; the name is the first counted string of `SampleDABE/DATA` (`wiff/names.rs`, `wiff.md`); a table row with no `Idx` is an injection that acquired nothing and stays a sample with 0 scans. The method is per METHOD, shared by samples (`wiff-writer.md` (a)).
MEMORY (not checked here; treat as a description of the idea, not of the exact UI): in Analyst `File > Open Data File` shows a folder browser and the `.wiff` files of the chosen folder; selecting one `.wiff` lists the samples inside it in a second pane, and the user selects one or several (Ctrl/Shift) and presses Open; each opened sample is a separate window/trace in Explore. I recall the sample pane showing the sample name; I do NOT reliably remember other columns (sample ID, acquisition
date/time, vial/plate position, method), whether it has an «open all» button, or whether the list is multi-select with checkboxes or with the usual selection. UNKNOWN: the exact columns and wording; the owner can describe or screenshot the dialog (no real data in the repo: a screenshot goes to the private data repo).
Not the same product: SCIEX OS opens `.wiff2` with its own Explorer and behaviour; our reader refuses `.wiff2`.
What our reader knows per sample WITHOUT the `.scan` (CERTAIN, `idx.rs`, `wiff.md`, `wiff_specifica_completa.md`): scans count (rows of `Idx`; empty rows have points 0), retention times (f64 ms), TIC per scan (so apex time and size), section bytes (sum of block sizes; the 44-byte lead and gaps aside), experiment kinds and polarity (from the method: `ExperimentHeader` +86 polarity, MRM marker, `MassRangeEx` for SIM, scan type codes), method name (`.dam` name in `SampleDABE/DATA` matched to `AcqMethodFileInfoStm`), DAD present or not (`DADSampleHeader`).
What our reader does NOT know (UNKNOWN, no code reads it): per-sample acquisition date and time (the mzML `startTimeStamp` comes from the creation date of the whole file, `wiff.md` line on the header), vial and plate position, batch/set name, operator (`Log`, which may hold a name and must stay out of any listing). Where they live in a real file is open: candidates are `SampleDABE/DATA` (more counted strings after the name), `SampleTable`/`DabsInfo` (strings `s1`, `s2`…, `wiff-writer.md`),
`Log`. Experiment X1 below finds out; until then the dialog has no date and no vial column (an empty column is not shown, never filled with a guess).

## 2. What the engine can list cheaply
Container (CERTAIN): the `.wiff` is an OLE2 file read whole into memory by `wiff::container` (limit `WIFF_MAX`: `Limits::sito().byte` 192 MB site, `desktop()` 1024 MB, `memory-limits.md`); the `.wiff.scan` is read by position. PR #3398 says a real `.wiff` with one 15 MB DAD stream per sample reached 614 MB while its `.scan` was 13 MB: so «the `.wiff` is small» is false for DAD files and a by-sector OLE open (`Ole::open_at` in #3398) is needed for the listing on the site (limit untouched on the page today: `error.memoryBudget` phase `whole`).
Measurements (throwaway Rust probe over the real `Reader`/`Ole` code, built `--release`, 4 vCPU Linux container, files in the page cache, synthetic pairs from `tools/wiff_grande.py`; the probe was not committed). «apri» = `readers::apri` + `samples()`, what `native_samples` runs today (it reads the block heads to count scans). «cheap» = `wiff::container` + `wiff::samples` + the `Idx` rows of every sample (count, time range, TIC apex, section bytes), the `.scan` never opened.

| Pair | Samples / scans | `.scan` | apri: bytes read, time, peak RSS | cheap: bytes read, time, peak RSS |
|---|---|---|---|---|
| p1 | 1 / 1 300 | 3.5 MB | 3.6 MB in 6 reads, 34 ms, 4.7 MB | `.wiff` 75 KB, 0.2 ms, 3.2 MB |
| p50 | 50 / 15 000 | 24.8 MB | 25.7 MB in 52 reads, 176 ms, 11 MB | `.wiff` 0.9 MB, 0.8 ms, 4.0 MB |
| big | 40 / 80 000 | 427 MB | 432 MB in 442 reads, 2 068 ms, 32 MB | `.wiff` 4.4 MB, 4.1 ms, 7.5 MB |
| huge (big's `.wiff` + a sparse 12 GB `.scan`) | 40 / 80 000 | 12 GB | not run (the sparse file holds no blocks) | 4.0 ms, 7.6 MB: the `.scan` size changes nothing |

- apri reads about 1.0× the whole dataset (the same finding as `crates/mzlab-core/tests/readers_open_counts.rs`: «the container and the heads of every block, each byte about once»). Extrapolated linearly (EXTRAPOLATION, not measured): about 5 ms per MB here, so 12 GB takes about 1 minute native from a warm cache and far more through `File.slice` in a browser or from a cold disk, with 442 read calls per 427 MB. The owner's real files (20-90 MB) would list in 0.1-0.5 s native: acceptable, but the huge case is not.
- cheap is bounded by the `.wiff` size and the number of samples (4 ms for 40 samples and 80 000 rows; the `Idx` streams are 54 B per scan: 0.36 MB per sample in #3398's real pair). Memory: whole `.wiff` today; with a by-sector open only the FAT, the directory and the streams asked.
- Caveats: the fixtures carry no `SampleDABE/DATA` (names are one small read per sample, not measured); the real cost on the page depends on `File.slice` latency and is NOT measured; polarity and type from the method need the `MethodN` streams (a few KB each, not measured).
- Cheap does not touch the `.wiff.scan`, so it cannot see a damaged or cut block: the dialog shows the evidence it has (counts from the index) and the PROBLEM flags (`unindexed`, truncated `.scan` length shorter than the last section end, which is a file-size comparison and free) while the exact `losses` appear after opening, as today (`warn.wiff.damaged`, `Lettore::missing`). A sample is never refused by the listing (rule R1/R2 of `wiff.md`).
- Size class for the warning: section bytes from the `Idx` sizes; OPFS estimate from the figures of `measurements-formats.md` and `native-reading.md` (about 200 MB of OPFS per profile sample), not recomputed here.

## 3. Existing work: #3398 and the `.wiff` writer study
Facts from the PR description (the diff was not read):
- #3398 «Open a huge .wiff pair: read by sector, DAD on demand, choose samples, convert --samples»: open, not draft, `mergeable_state` dirty, 14 commits, 53 files (+1 461 −232), set aside by the owner (red, conflicts). Contents: (a) `Ole::open_at` (read by sector, only FAT/directory/mini stream held) and `wiff::container::open_member`; (b) `Wiff::load_dad` (DAD on demand); (c) the picker `wiff-scelta.js` (only for a pair above 192 MB with several samples, none ticked, «Open selected» / «Open
  all» / «Cancel», `role=status`); (d) opening a chosen subset copies to OPFS only the byte ranges its samples need (`native_ranges`, `putSparse`); (e) CLI `convert --samples ID,ID`; (f) the «partial identity» `partial(sample=ID):HEX` in mzML, plan, `.mzidx` fingerprints, CLI report; (g) `error.memoryBudget.wiff` hint; (h) `lettori-sciex` cap 486 400-ish → +2.5 % (owner's coordinator decision).
- `wiff-writer.md` (study, no code): `mzlab-cli wiff list IN.wiff... [--json]` (`wiff.list/1`: per file container info and SHA-256; per sample `source`, `name`, `method_no`, `scans`, `blocks`, `section` with start/bytes/SHA-256, `streams` with the reader's `read` flag, `status`, `reason`) is a forensic inventory for a writer (hashes of every stream, `unaccounted` bytes): it hashes the whole `.scan` sections, so it is the opposite of cheap.
Overlap and reuse:
- Listing for a chooser (this study) ≠ `wiff list` of the writer study: different jobs. Keep `wiff list` as the heavy inventory (hashes, streams), add `samples list` as the light chooser listing (no hashes); the writer's `wiff.list/1` can embed the chooser's per-sample object under `summary` so one definition feeds both. Say so in the writer PR before either is written.
- #3398 (a)+(b) are the memory fix for big-DAD files and are needed by the cheap listing on the site: revive as its own PR (engine only, no UI).
- #3398 (c) the picker is a good base for the dialog, but its trigger differs: it appears only above 192 MB and starts with nothing ticked; the owner wants it for any multi-sample file. Keep the ideas, rewrite the trigger.
- #3398 (d) OPFS copy by ranges makes «open fewer samples» cheaper on the site, but is the riskiest part (partial `.wiff` in OPFS, cache key per choice): split off, last.
- #3398 (e) `convert --samples` becomes the shared `--sample 1,3,5` (`channels --sample ID` exists on main; accept a list).
- #3398 (f) partial identity is an owner decision (Q4): honest and cheap, but it changes mzML identity and cache keys; not bundled.
- #3398 (h) the +2.5 % cap is above the 1 % rule; see section 5.
Verdict on #3398: CLOSE as superseded by the small PRs of section 6 (the owner closes it; this chat does not), reusing its tests `wiff_by_sector.rs`, its picker e2e test and its lang keys as raw material. Reviving it whole is not advisable (53 files across Rust, wasm, desktop, JS and e2e went red; the trigger must change anyway).

## 4. UI proposal
Entry points, all end in the same dialog: the Files picker, drag and drop of files, drag or pick of a folder (`cartella-carica.js`), and in the desktop app the native open dialog or a file opened from the OS (`opening.rs`).
Trigger (recommendation): after pairing (`WIFF_WAIT`, the `.wiff` + `.wiff.scan` pair is in the list as soon as both are present) and the cheap listing, show the dialog if ANY of the files has more than one sample; a file with ONE sample opens straight away (a dialog with one ticked row is a click for nothing). A setting «Choose samples when a file has several» (Settings, default ON; «Always ask» adds the one-sample case, «Never» restores today's open-all) with the choice remembered per file
(name + size + date, as `NATIVO.cached` keys do): reopening the same file preselects the same samples.
Mockup (monospace; `[x]` checkbox; text fields are lang keys):
```
 Choose samples                                                     [ x ]
 3 files, 87 samples. Tick what to open. Nothing is read until you press Open.
 Filter: [ name contains...        ]    [Select all] [Select none] [Invert]
 +--+------------------------------+-----+----------+--------+---------+-------+
 |  | Sample                       |  #  | Type     | Scans  | TIC apex| Size  |
 +--+------------------------------+-----+----------+--------+---------+-------+
 v run_A.wiff  (42 samples, 61 MB .scan)                  [select this file]
 |[x]| <name from the file>        |   1 | MS, MS²  |  1 320 | 6.4 min | 1.4 MB|
 |[x]| <name from the file>        |   2 | MRM      |   410  | 3.1 min | 0.5 MB|
 |[ ]| <name from the file>        |   3 | SIM      |   —    |   —     |   0   |  empty injection
 ...
 v run_B.wiff  (45 samples, 2.1 GB .scan)
 |[ ]| ...                         |  12 | MS², DAD | 51 800 | 18.2 min| 112 MB|  large: ~1.2 GB in the browser
 ...
 Selected: 3 samples, about 119 MB read (estimate)       [Open selected] [Open all (87)] [Cancel]
```
- Columns (recommended, from what the engine knows today, section 1): checkbox, sample name (as the file writes it, never an invented one; a nameless sample shows the `<stem>_s<N>` name `names.rs` gives), number, type chips (MS, MS², SIM, MRM, DAD; polarity +/− when the method says), scans, TIC apex time and a small size class (bar or «S/M/L» by section bytes), section size. Date/time and vial/plate columns are added only if X1 finds them in the real files; hidden when empty.
- Rows grouped per file with a collapsible header (select this file, select none in this file); the table sorts by number (default, the order in the file), name, size. Filter by name (substring, case-insensitive; filter plus «Select all» ticks only the visible rows, and the counter says so).
- Remember choice: per file as above; «Select the same numbers in all files» for equally structured runs (off by default).
- Evidence, never blocks (rule «the user decides»): a size class warning is a line with numbers («about 1.2 GB decoded; this tab has room for about N MB») and the sample stays tickable; if the selection exceeds the memory estimate the line turns into the same text as `error.memoryBudget` would give BEFORE trying (recommending fewer samples or the desktop app), and «Open selected» stays enabled. Empty injections, `inferred` (unindexed) sections and a truncated `.scan` are marked (`why`) with the existing `warn.wiff.*` texts, not hidden and not blocked.
- Keyboard and screen reader: modal `role=dialog` labelled by its title; a real `<table>` with `scope` headers and native checkboxes; Space toggles, arrows move, Shift extends, Ctrl+A selects the visible rows, Enter opens, Esc cancels; the selection counter is a `role=status` live region; focus returns to the picker button; at 390 px rows become cards.
- Buttons: «Open selected» (primary, disabled with none ticked), «Open all (N)», «Cancel» (loads nothing; a one-line note tells the files were not opened, a drop is not lost silently).
- Several files: one dialog, one section per file, one «Open» for all (not one dialog per file). A `.wiff` without its `.wiff.scan` keeps today's behaviour: the existing wait/error (`load.wiffWait`, `err.file.wiffNoScan`) and is shown in the dialog only as a greyed section «waiting for X.wiff.scan», never listed as openable. A pair read by the readers module (`variante`) uses the module's own list (`probe` → `campioni`).
- Dropping a whole folder: the folder loader (`cartella-carica.js`) finds the datasets first; the chooser opens once at the end and lists multi-sample datasets only; one-sample datasets open without asking.
- Desktop app: the same page dialog (the app runs the same page); not the OS dialog, which cannot show samples. The native file dialog stays as it is (it selects files). The list comes from the native route (same bytes as the wasm one), the OPFS limit does not apply, so the size line is informative only. The desktop already opens samples per file (`opening.rs`, #3398 says the app has no picker UI yet).
- Other formats: a Waters `.raw` folder holds several FUNCTIONS but ONE sample (`waters.md`; functions are channels, listed by `channels`); an Agilent `.D` is one sample (a sequence is a folder of `.D`, a set of datasets); Thermo `.raw`, Bruker `.d`, Shimadzu `.lcd`, mzML and mzXML are one sample each. So `Reader::samples()` has several elements only for a `.wiff`, and a «what to open» level for the datasets inside a dropped folder is a different, file-level feature.
  Recommendation: generic route and dialog (fed by `samples_list/1`, shown when a dataset reports more than one openable item), shipped for `.wiff` only.

## 5. Engine, CLI, parity, wasm cap, lang keys
Route (all logic in Rust, one function, same bytes site = app = CLI):
- `mzlab_core::readers::list_samples(&mut dyn Reader) -> String` extends `samples_json` (kept byte-for-byte for current callers; the new JSON is a superset behind the id `samples_list/1`): per sample `id`, `name`, `scans`, `size` (section bytes), `kinds` (`ms`, `ms2`, `sim`, `mrm`, `dad`), `polarity` (`+`, `-`, `both`, absent), `rt` (`[min, max]` minutes), `tic` (`{apex_rt, apex, sum}`), `method` (name or absent), `why`, `refused`; per dataset `format`, `bytes`, `unindexed`. No hashes, no log
  text, no operator. A `.wiff` reader provides it from the container and `Idx` only (`Reader::sample_summary(c)`, default: derived from `samples()` so other formats work unchanged); the `.scan` is not opened. Needs the by-sector OLE open for `.wiff` above the whole-read limit.
- wasm: `native_samples` keeps its name and returns the new JSON in the same call (no new export, no new module). App: the same function through the existing native listing. CLI: below. A byte-parity test as for the other routes: native, wasm in Node, CLI give the same JSON for the synthetic pairs of `tools/wiff_riferimento.py` (`test_wiff_*_parita.py` style).
CLI (`mzlab-cli`, in `crates/mzlab-cli/src/samples_cmd.rs`, which already holds `samples vars|table|group`: a new verb `list`):
- `mzlab-cli samples list FILE... [--json] [--probe]`: one `samples_list/1` per file on stdout (plain text table without `--json`); progress and warnings on stderr; `--dry-run` is implicit (it reads, never writes). `--probe` adds a check of the `.scan` length against the last section end (the cut-copy test) without decoding.
- Exit codes (the pattern of `cli-agenti.md`): 0 listed; 1 some file or sample refused (listing still printed); 2 usage; 3 not applicable (no listing for a plan problem: unused here, reserved); 4 missing or unreadable input.
- `--sample 1,3,5` on every command that reads a multi-sample file: `convert`, `channels` (today `--sample ID`, one), `features`, the `flow` inputs. A number not in the file is an error (exit 2, the numbers present named on stderr), no silent skip. In `flow run`, the same restriction as an input property in the plan (`mzflow/2` input `samples: [1,3,5]`), recorded in `prov.json` (evidence of what was left out). Output of a restricted run says it is partial (the identity rule, Q4).
- Agent catalogs: `llm tools` entry and a line in `cli-agenti.md`/`cli.md`; `steps --json` unaffected (not a step).
Wasm cap (ESTIMATE; no wasm-bindgen or wasm toolchain here, nothing measured on a `.wasm`; rule: up to +1 % over the old cap accepted, above that or any new module/cap entry: stop and ask):
- Per-sample summary from `Idx` + method experiment kinds: about 150-300 lines of Rust, about +2 to +5 KB optimized, sitting where `samples_json` already is (`lettori-sciex` and `motore`).
- `lettori-sciex`: cap 486 400, measured 485 158, room 1 242 B (`tools/moduli/lettori-sciex.toml`); 1 % of the cap is 4 864 B, so the accepted ceiling is 491 264 B. The estimate (+2 to +5 KB) straddles the ceiling: it fits only if the new code reuses the existing `Idx`/method readers instead of adding parsers. DO NOT assume it fits: measure with `python3 tools/moduli.py --misura` in the first PR, and if it passes 1 % stop and ask the owner (the PR body says so).
- `motore` (2 147 104 B, cap 2 183 168, room 36 064 B): +5 KB is +0.2 %, fits.
- The by-sector OLE open (#3398 a) is a separate cap question: #3398 measured it, with the partial identity and DAD change, at +2.5 % of `lettori-sciex`; this study cannot separate its share. Its PR repeats the measurement and asks if above 1 %.
Lang keys (English, new `pick` file under `mzlab/web/lang/en/` with its contexts, checked by `tools/controlla_i18n.py`): `pick.title`, `pick.intro`, `pick.filter`, `pick.all|none|invert`, `pick.file`, `pick.col.*`, `pick.type.*`, `pick.empty`, `pick.large`, `pick.partial`, `pick.sel` (live region), `pick.open|openAll|cancel|cancelled`, `pick.waiting`, `pick.remember`, `pick.setting.*`, `pick.aria.*`. Sample names come from the file and are never keys.

## 6. Split into small PRs (one task, one PR; each CI green before the next)
1. (DONE, this PR) Engine + CLI, no UI: `list_samples` + `samples_list/1` (cheap `.wiff` listing, cap measured), `mzlab-cli samples list`, parity tests (native/Node-wasm/CLI), docs rows. No behaviour change on the page.
2. By-sector OLE open and DAD on demand for big `.wiff` (from #3398 a, b): memory only, the listing and `Wiff::open` work above `WIFF_MAX`; cap measured; tests `wiff_by_sector.rs` as base.
3. (DONE) `--sample LIST` on `convert` / `channels` / `features` + `samples select`, CLI only; `sample-flag.md`. Left: `flow run` input property.
4. The page dialog «Choose samples» (setting, remember, filter, a11y, i18n), reading `native_samples`; on the site it still copies the whole pair to OPFS; skipped samples are not indexed. e2e its picker e2e test revived for several files, one file, a pair without scan, drop of a folder.
5. OPFS copy by ranges of only the chosen samples (from #3398 d): the part that makes a huge pair openable in the browser at all; own PR, own risks.
6. Optional: desktop-specific polish, a file-level chooser for folders, date/vial columns after X1.
PRs 1-3 can run in parallel with the dialog design review; PR 4 needs PR 1; PR 5 needs 2 and 4.

## 7. Risks
- The cheap listing can disagree with what opening finds (damaged blocks): the dialog says «per the file index» and opening reports losses as today (R1/R2, `losses`).
- Type and polarity from the method are guesses for samples with no method or a borrowed one (`method_no` rules, `wiff-writer.md`): shown with the reader's «borrowed» mark, never asserted.
- A remembered choice keyed by name + size + date could preselect samples of a changed file: the preselection is visible and editable.
- Dialog fatigue: the setting, «Open all» and the one-sample shortcut.
- Cap: PR 1 may pass 1 % on `lettori-sciex` (section 5); mitigation is reuse, measurement, and stopping to ask. `--sample` uses the file's sample ids, not positions, like `channels`.

## 8. Questions for the owner (recommendation first)
- Q1 Trigger: dialog only when a file has more than one sample, setting to always ask or never (recommended), or always for every `.wiff`.
- Q2 Default selection: all ticked when the whole set is small and each sample is small, none ticked when the selection would exceed the memory estimate (recommended), or always none, or always all.
- Q3 Scope: wiff-first with a generic route (recommended) or a generic «what to open» chooser for folders too (a separate, larger feature).
- Q4 Identity of a partial open (mzML `source-identity`, `.mzidx`, cache keys): follow #3398 (`partial(sample=ID):HEX`, honest and cheap, changes cache keys) or hash the whole file and mark the output partial (reads everything, defeats the point). Recommended: #3398's approach, in its own PR.
- Q5 What to do with #3398: close it and reuse (recommended), or revive it whole.
- Q6 Columns: can you tell (or send a private screenshot of) what Analyst 1.6.3 shows in Open Data File, and run `python3 tools/wiff_dump.py FILE.wiff --all` (without `--log`) on a real file in the private data chat to find the date and vial strings (X1)?
- Q7 Wasm cap: if the first PR passes +1 % on `lettori-sciex`, accept a measured cap raise or move the listing to the engine only.

## Experiments not run
- X1 (needs a real file): where per-sample date/time, vial, plate position and batch name live in a real `.wiff` (`wiff_dump.py --all`, private chat). Until then no date/vial column.
- X2 wasm build and size: no wasm-bindgen / wasm toolchain here; all cap figures above are estimates.
- X3 browser timing: the listing through `File.slice` (and from a cold disk or network drive) was not measured; only native numbers above.
- X4 real files: nothing measured on real pairs; the 614 MB `.wiff` / 13 MB `.scan` numbers are #3398's (aggregate, already public).
- X6 Analyst 1.6.3 behaviour is from memory (section 1); the Open Data File dialog was not seen.
- X5 the #3398 diff was not read (description and file list only); names and method-derived type/polarity were not in the probe (synthetic pairs carry no names).
