# Data channels: one file = a set of channels
The "experiment type" is a label and a heuristic for the defaults; the tools work on the selected channel. The engine (§2: `channels.rs`, `channels_uv.rs`, `api/channels`, `channel=`, `mzlab-cli file channels`) and the page selector (§5) exist; §1 says where the kind still decides, §4 the open work.

## 1. Where the kind still decides (evidence in the code)
**Classification (Rust).** `item_info` (`crates/mzlab-core/src/iteminfo.rs:459-530`) gives every item a `kind` ∈ `full|ms2|mrm|infusion|empty`: `infusion` if `infusion::infusione` (`infusion.rs:109`) recognizes fixed precursors (B: ≤64 MSn keys, median ≥10 scans) and long blocks (C) or a `.raw` method without LC or a name `infus|fia|direct|syringe`; then `ms2` (MSn), `full` (MS1 only), `mrm` (SRM only). A "mixed" file is split into items (`file_parts`, `iteminfo.rs:92`: level × scan type PREC/NL/NG/EMC/TDF/ER × polarity + "MRM"), each with its own `kind`.

**Where the kind changes the interface (JS).** Single registry `ESP` in `mzlab/web/esperimenti.js:6-16` (`spettri`, `vista`, `nodi`).
- Table "1. Data files" (`shell.js`): order `GORD` (85), groups (87-88), Experiment column `expCell` (99-102), name≠kind warning (103), openable only the 4 kinds (139), concentration only `mrm` (71).
- Panel tabs per kind: `explore.js:162` `tabFiles(t)` = files with `kind===t`; every block of panels belongs to a kind.
- Chromatogram: on the `ms2` tab forced to the TIC of the MS2 of a precursor, without m/z range, blank, background, log (`ms2Chrom` in `pannelli.js`; a `pda`/`press`/`an:*` chromatogram of the tab is ordinary); distinct selectors for the same concept: `kind` (tic/bpc/pda/press/an:*/reg, 660-662, 678), `prec` (663-664, tolerance 0.6 Da or `ptol` of the panel, `grafici.js:85-86`), MS2 `tr` (`api/ms2tr`, 676-677, q3±0.5), MRM `tr` (687-689), `filt` only in the HR Bench (`banco.js:38-71, 252-272`).
- MRM: a separate panel type `"mrm"` (`tabs.js:146-156`, `api/mrm`), excluded from table, profile, QA/QC, map (`tabella.js:61`, `profilo.js:20`, `qaqc.js:30`, `lr-map.js:10`).
- Infusion: tree `api/msn_tree` (`infusione.js:9-64`), tabs "alb"/"gra" only if `INF.any()`/`INF.single()` (`barra.js:105-106`), filters `msn:<id>`, entries of the peak menu (`infusione-menu.js`).
- PDA: `kind=pda` of the chromatogram with nm in `mz0/mz1` (`uv-spettro.js:60-63`), UV spectrum `api/uvspec`; analog `api/analog?i` (`grafici.js:53-61`).
- The XIC lens works in the spectrum's channel (`api/xic?channel`, `xic-lente.js`); whether it is proposed is decided by the type (`ESP.predefinito`, field `lente`).

**Engine: what already exists and is almost a channel model.**
- `Scan::key()` (`idxread.rs:127`) = filter without range for MS1, `ms{n} <nominal masses> @act` for MSn; `/api/filters` (`routes.rs:213`) groups the scans by key `{key, level, n, rt0, rt1, pol}`, computed on request, not stored.
- `/api/chrom?kind=tic|bpc&level&prec&filt&mz0&mz1` (`routes.rs:145`) filters by key (`Filter::Testo` of `msnroute.rs`) or by tree node (`filt=msn:<id>`, `msnroute.rs:55`); `/api/scanlist?filt` likewise.
- Non-scan chromatograms: `mzml::ChromKind{Tic,Bpc,Pda,Pressure,Srm,Sic,Analog,Other}` (`mzml.rs:64`), list on request `mzmlhead::Chrom{index,id,kind,q1,q3,ce,name,dwell,n}` (`mzmlhead.rs:140`), in the `Facts` as `srm`, `pda`, `analog[]`.
- Block reading of SRM chromatograms: `Mzml::chromatograms_each` (times and intensities) and `chromatograms_times` (times only) read neighbouring chromatograms together, in spans of up to 16 MiB (`reading.rs::in_spans`), and decode each from its own slice: arrays bit-identical to `chromatogram`/`chromatogram_times`; one beyond the span or out of order forms a span of its own. Used by `facts_from` (`srm_rt`), `fileroutes::mrm_transitions` (`api/mrm`) and `Sorgente::srm`: on the site every read is a Blob slice, so thousands of transitions cost few reads (`reading_tests.rs`: 4000 SRM, <60).
- `.wiff`: modes in `wiff/modes.rs:42-60` (Q1, Q1MI, Q3, Q3MI, MRM, Prec, PI, NL, EPI, ER, MS3, TDF, EMS, EMC; codes 0, 4, 5, 6, 7, 11, 15 confirmed); MRM → one TIC + one `SRM SIC Q1= Q3= … experiment= transition=` chromatogram per transition (`wiff/run.rs:25-49`); DAD → `DadData` (`wiff/dad.rs:33`).
- Thermo: text filter in `Scan.filter`, `path` from `filter_path` (`scantable.rs:238`); scan SRM/CRM → chromatograms per transition (`mzlab-lettori/src/srm.rs`).

**Where information is lost or mixed.**
1. The kind decides the tools: a heuristic error (targeted MS2 of bentazon → "infusion") turns off useful tools and turns on wrong ones.
2. A mixed file becomes several rows and several tabs (DDA = "Full Scan" + "MS2"; MRM+EPI = two items): the file loses its unity, the overall TIC does not exist.
3. An XIC without `channel=` (`routes.rs` `xic_raw`, `api/xicbin`) sums ALL the scans of the level: on level 2 it mixes different precursors and energies.
4. Four mechanisms for "which subset of scans": `prec` (0.6 Da), MS2 `tr` (`ms2tr`), `filt`/`msn:` (only HR and infusion), the `mrm` panel.
5. `.mzidx`: no experiment id, period or cycle (for the `.wiff` they are only in the `native` text, `wiff/run.rs:107-113`); only `Period0` is read (`wiff/mod.rs:120`); scan window, `p/c` and `[a-b]` of the Thermo filter are not fields.
6. Non-indexed mzML chromatograms (only the number in `run.n_chromatograms`): every list rereads the tail of the file.
7. Agilent and Shimadzu PDA not decoded (`agilent_ext/reader_lc.rs`, `mzlab-lettori/src/shimadzu/reader.rs`): the channel is missing, not hidden.

## 2. "File → channels" model
**Principle.** Every file exposes a list of channels; every channel has a trace (time → intensity) and, if it has them, its scans with the spectrum (mass or UV). The tools ask for "channel + file", never "kind".

**Families and stable ids (strings, the same in CLI, wasm and page).**
- `tic` (all MS scans, time order; for an MRM-only file = sum of the transitions, the vendor TIC if there is one), `bpc`.
- `ms:<key>`: a group of scans with the same `Scan::key()` (MS1 full, SIM, EMS, EPI with fixed precursor, MS2/MS3… of a path). Children of `tic`; for MSn the parent is the precursor's channel (`ms:<key>` of level n-1), so the precursor tree IS the channel hierarchy.
- `dd:<level>[:<tag>]`: aggregate of the data-dependent MSn (DDA, IDA EMS→EPI) when the distinct keys of that level exceed 64 (same cap as `infusion.rs`): a single channel with the scans and the precursors as an on-demand sub-view (not 3 000 channels).
- `srm:<q1>><q3>[@ce]` (one MRM/SRM/CRM transition), `sim:<id>` (SIC chromatograms), `uv:twc`, `uv:<nm>[±bw]` (wavelength asked by the user: derived channel), `uv:auto` (CLI only: the proposed band), `an:<i>` (pressure, flow, single UV).
- `group`: key to overlay different files (level + nominal masses of the path + activation + rounded CE; q1/q3 at 0.1; nm). Two files share a channel if they have the same `group`.

**Structure in Rust (`crates/mzlab-core/src/channels.rs` and `channels/`; field names in English in the code).**
```rust
pub enum Famiglia { Tic, Bpc, Ms, Dd, Srm, Sim, Uv, Analog }
pub struct Channel {
    pub id: String,            // "ms:FTMS + p ESI ms3 317 261 @cid30"
    pub famiglia: Famiglia,
    pub parent: Option<u32>,   // index of the parent channel (tic, precursor)
    pub livello: u8,           // 0 for uv/analog
    pub pol: i8,
    pub sigla: Option<&'static str>, // EMS, EPI, MRM, Prec, NL, Q1MI… (modes.rs) or "SIM", "DDA"
    pub label: String,         // short, for the list: "MS3 317→261 @CID30"
    pub group: String,         // same value in two files = the same channel
    pub n: u32,                // scans or points
    pub rt: (f64, f64),
    pub spettri: bool,         // has scans with a spectrum (ms, dd, uv)
    pub source: Source,        // where the trace lives
}
pub struct Channels { pub v: Vec<Channel>, pub scan_ids: Vec<u32>, pub experiment: ExperimentType }
```
`scan_ids` is a partition of the MS scans ordered by channel (`Source::Scan{da,a}` = slice): 4 B per scan, no spectral data copied. Built in O(n scans) from the table already in memory (`idx.scans`, ~640 B/scan) + the chromatogram list (`Facts`/`mzmlhead::Chrom`); done once per file and kept in the `Routes` (100 000 scans → 400 KB + a few hundred channels). No change to the `.mzidx` format.

**Experiment type as a label (`ExperimentType`).** `ExperimentType { proposed: &'static str, reason: &'static str }` with descriptive values `lc-full`, `lc-ms2-targeted`, `lc-dda`, `lc-mrm`, `lc-sim` (SIM chromatograms only, even with a UV trace), `msn-infusion`, `mixed`, `uv-only`; computed from `infusion.rs` + channels (`experiment_of`); it is a proposal: the user's value, when it exists, wins and lives in the session. It serves ONLY the defaults (`ESP.predefinito` in `esperimenti.js`): initial channel, XIC lens proposed or not, "alb" tab open at startup.

**API (`/api` routes, the same in wasm and desktop; JSON with version).**
- `GET api/channels?k` → `{"schema":"channels/1","kind":{proposed,reason},"channels":[{id,family,generator,ms_level,polarity,code,label,group,n_points,rt_start_min,rt_end_min,has_spectra}]}` (times in minutes; the full key list with units is `docs/schemi/channels-1.json`).
- `GET api/chrom?k&channel=<id>&kind=tic|bpc&mz0&mz1` → channel trace (for `srm:`/`uv:`/`an:` it redirects to the current reader: `api/mrm`, `pda_trace`, `analog`).
- `GET api/xic?k&mz&tol&channel=<id>` → XIC inside the scans of the channel (without `channel=`: per level).
- `GET api/scanlist?k&channel=<id>`, `api/spectrum?k&channel=<id>&rt|scan&mode=sum`.
Adapter: `channel=ms:<k>` ≡ `filt=<k>`, `channel=msn:<node>` stays, `level`/`prec` stay accepted and translated; no old route disappears while a test uses it. In JS a single module `canali.js`: `canali(k)` (cache), `traccia(k, id, opz)`, `byGroup(files, group)`. CLI: `mzlab-cli file channels FILE` (`cli.md`).

**Construction per format.**
- mzML/mzXML and Thermo `.raw`: keys from `Scan::key()`; SIM = MS1 key with "SIM"; `srm`/`pda`/`pressure` chromatograms from the tail (`mzmlhead`).
- `.wiff` (EMS, EPI, MRM; `channels/wiff.rs`): tag from `modes.rs` per level; MRM → `srm:` from the per-transition chromatograms (method name as label); EMS/EPI → `ms:`, or `dd:2:EPI` if IDA; Prec/NL → `ms:` with tag; DAD → `uv:`.
- Shimadzu/Agilent `.D`/Waters: from the readers' chromatograms (`ChromKind`); the PDA enters when the reader decodes it.

## 3. Rules and decisions in force
- Names: "Channels"; "Experiment type" stays as a label. Visible names of the Type: "LC full scan", "LC MS²", "LC DDA", "LC MRM", "MSn infusion", "mixed", "UV only" (internal ids unchanged).
- File TIC = all MS scans (like the vendor TIC), with `MS1` as a child.
- DDA: cap of 64 keys per level before the `dd:` aggregate.
- Defaults per type (initial channel, proposed lens): table `predefiniti` in `esperimenti.js`, editable; a default, never a permission: a type never turns a tool off.
- File overlay: the panel saves the channel as its `group`, not as an index; each file resolves its own; a file that does not have it appears in the legend as "—" (no error). Overlay with CE rounded to the integer, exact CE in the channel `title`.
- The "alb" tab exists for every file with MSn channels (`infusione-albero.js` reads `api/channels`), not only for infusions.
- Texts: no paragraphs where the interface is self-explanatory; tooltips only to say what a field does or what to enter in it.
- No new dependency; no change to the `.mzidx` format.

## 4. Open work
- **One row per file** in the table "1. Data files": `file_parts` (`iteminfo.rs`) still splits a mixed file into several rows; the parts become channels, "Type" column as an editable label (the user's field in `ExperimentType` does not exist yet), "Channels" column (n). The `ESP` registry becomes only `default` per type.
- **Tabs per kind** (`tabFiles` in `explore.js`): the direction is a single workspace, with the same tools for every file. Tabs and `file_parts` are turned off last (many e2e tests use them).
- **Duplicate selectors**: `prec` (0.6 Da), MS2 `tr` (`api/ms2tr`), `filt`/`msn:` stay behind the adapter while e2e tests use them, then are turned off first; the `mrm` panel stays a type of its own.
- **Scan list** of the selected channel (`api/scanlist?channel`) in every view, also in LR (only the HR Bench uses it, `banco.js`).
- **`.wiff`**: periods beyond `Period0` and experiment as a field (optional `exp` column of the `.mzidx` only if the `native` text is not enough).
- **PDA** Agilent and Shimadzu (§1, point 7).
- **Flow**: the Raw files node shows the type and the number of channels from `api/channels`; the steps receive a channel (`ms:<MS1 key>` for features, `srm:` for quantification) instead of guessing the level from the kind.

## 5. Channel selector (Data page)
`canali.js` (`window.CANALI`, `canali.css`, keys `canali.*`): the single chromatogram selector (button `data-o=kind`, label "TIC", "TIC + 1") opens a tree of groups (`role=tree`, arrows/Space/Enter/Esc): "Mass spectrometry" (name without the mode tag, already in the "Experiment" column; multiple choice: `p.canali`, saved by `panelRec`; only the families that `traccia()` draws), "UV/Vis" (DAD), "Detectors" (analog) and "Logs", single-choice rows (`data-kind`, they set `p.kind`); the MRM panel has "Transitions" / "Ions" (`data-o=canali`, `p.tr`).
- UV/Vis outside Full Scan: if the files of the tab have a DAD or an analog trace, the MRM/SIM selector and the MS2 chromatogram (fixed "MS2" row in place of the disabled `select`) have the groups "UV/Vis" and "Detectors": the row opens (or shows, if already there) a `kind` chromatogram of that detector in the same tab, below the panel, with the time axis linked (`CANALIUV` in `canali-uv.js`, loaded on request, cancellable); `addPanel` from the `mrm` tab leaves a DAD/analog chromatogram there. Without a signal the groups are missing. Test: `tests_e2e/e2e_dad_mrm.py`.
MRM/SIM selector (`ESP.mrm`): click on the name = only that channel; box `.can-hit` (24 px), Ctrl/⌘+click or Space = add/remove (the last one stays); Enter = only that one; "All" (`.can-all`) = empty `p.tr`; "None" (`.can-none`) = `p.tr` `[]`, empty plot with `canali.pick.none.msg`. Help `.can-hint` `sr-only` (`aria-describedby`) and `title` of the tree.
Choice memory (`MRM_MEM` in `tabs.js`, session only): every drawing of an MRM/SIM panel remembers `p.tr` per type (SIM ions / transitions) if it is not the layout's automatic choice (`p._trAuto`); at the next layout (sample removed and put back, removed, another sample) `mrmRecall` reopens the remembered channels that the shown files have, the same for all overlaid samples; if there are none, the quantifier/qualifier as usual. Test: `e2e_sim_memoria.py`.
SIM ions only (`can-sim`): one column, the m/z (or "ion N"), the polarity only if the files have both; engine name, dwell, points and time in the `title`. Test: `e2e_sim_ui.py`.
One trace per file × channel, "—" in the legend if the file does not have the channel, XIC inside the channel with `channel=`. The engine serves `api/channels` (`Routes::canali`, wasm method `canali`) and `channel=` on chrom/xic/scanlist/spectrum/nearest_scan; transitions and UV go through `api/mrm`. The initial channel per type is in `ESP.predefinito` (`esperimenti.js`).

## Extraction commands (`file info`, `trace`, `spectrum`)
- `file info FILE... [--sample LIST]` → `file_info/1`: per file the container, size and every sample (id, name, scans); the samples read (the first, or `--sample`) also hold `items` (= `file items`), `channels` (= `file channels`) and `kind`. It replaces `items` + `channels` in the common case. `file channels` no longer takes `--channel`.
- `trace chrom FILE --id ID [--rt A,B] [--sample ID] [--format json|csv]`: any id `channels` lists. `uv:auto` answers the band `api/uvpropose` proposes (`uvband/1`); `uv:twc` spectra are `dad spectrum`. Points are the route's own number tokens (`api/chrom`, `uvxic`, `analog`, MRM rounding 4/1 decimals). CSV header `rt_min,intensity`, readable by `peak auto-lr -`.
- `trace xic FILE (--mz M[,M...] | --sweep LO:HI:STEP) (--ppm P | --da D | --tol P[ppm]|D[da]) [--smooth 3|5] [--rt A,B] [--ms-level N] [--polarity +|-] [--mz-offset DA|auto:MZ[@RT]] [--dry-run]`: the bytes of `api/xic` per ion (the site draws several ions as repeated `api/xic`, so a list is the same bytes); evidence holds the tolerance in Da and ppm and the scans in the section.
  One ion is `trace/1`, a list `trace-set/1` (a `trace/1` per ion, in order) or, with `--format csv`, `rt_min,xic:<mz>,...` over the shared scans.
  `--sweep LO:HI:STEP` is the whole RT x m/z matrix in one call (`routes_sweep.rs`, `Routes::xic_sweep`): centres `LO + i*STEP` written with the decimals of STEP (of LO if it has more), at most 20,000. The output is EXACTLY the bytes of `--mz` with those centres as a list (`trace-set/1`, or with `--format csv` the wide `rt_min,xic:<mz>,...`: use it for sweeps, JSON repeats `rt` per ion).
  The sweep is NOT faster per ion than extracting the ions one after the other in one process (`--mz` is that loop and does not use the sweep: 500 centres, warm cache, 1,500 scans, 0.34 s sweep vs 0.32 s list); its gain is one call and one index load for the whole matrix: 901 centres took 0.8 s as one `--sweep` against 77 s as 901 `--mz` processes without `--cache` (18 s with a warm cache).
  Without `--tol/--ppm/--da` the half-window is STEP/2 in Da (a stderr note and `sweep.tol_default` in `xicplan/1`).
  The engine finds the index range of every window with the same `search(lo,false)`/`search(hi,true)` as a single extraction, then walks each run of overlapping ranges once, adding a peak to every live window in the order `sum_window` uses: every column is bit for bit one `xic_into`. `--mz` lists keep one extraction per ion (at most 500 ions).
  The matrix `centres x scans x 8 B` must fit in half of the memory budget (`error.memoryBudget`, phase `xicSweep`, exit 1, with a hint; `--rt` cuts the output, not the matrix, so narrow `LO:HI` or raise STEP).
  The last matrix is kept in `Routes::sweep` and dropped by `trim_cache` with the scan cells; it is not `mapscan::ScanCells` (a per-scan max in 1 Da cells, not the sum in `[mz-tol, mz+tol]`).
  `--dry-run` adds `sweep{lo,hi,step,n,tol_default}` and `matrix_bytes_per_scan` (the scans are not read).
  Route: `/api/xic?sweep=LO:HI:STEP[&tol][&level][&smooth]` answers the engine's `trace-set/1` (`{"id","mz","tolerance_da","rt","y"}` per centre, no file head), through the wasm `route` and the app's `Routes::handle`.
  `--smooth` is `api/xic?smooth=3|5` of the engine (`peak1d::smooth` after the scan filter, before the `--rt` cut; the site reaches it through the wasm `route`, the app natively) and is named in the evidence. An ion with no signal is exit 0 with `empty:true` and `empty_reason` in its evidence and a note on the standard error. `--dry-run` writes `xicplan/1`, the ions with their window in Da, and reads no data (except the estimate of `--mz-offset auto:`, whose evidence it then holds).
  `--mz-offset DA|auto:MZ[@RT]` (|DA| <= 0.6; list, sweep and single): see `mzfeat-lr.md` (m/z offset). Windows centre on M + offset, the trace reports M and its evidence carries `mz_offset_da` and `mz_offset`; route `api/xic?mzoff=` and `api/mzcal?mz&rt` give the same bytes on the site and in the app.
- `spectrum get FILE (--rt A[,B] | --scan N) [--ms-level N]`: `spectrum/1`, the bytes of `api/spectrum` (an RT picks the nearest scan of the level; a scan keeps its own level and precursor).
  - Without `--ms-level`, a file with ONE level other than 1 (an enhanced product ion file) uses it, logged on stderr and in `evidence.level_auto` {ms_level, levels, why}; a file with several levels and no level 1 is `err.scan.level` (exit 5, `params` levels and `hint` `--ms-level N`); level 1 stays the default whenever the file has it.
- `spectrum list` → `scanlist/1` (CSV: `scan_id,scan_no,ms_level,rt_min,tic,precursor_mz,base_peak_mz,base_peak_intensity`).
  - `spectrum get ... --minus FILE [--minus-sample ID] [--minus-rt A,B]`: the mean spectrum minus a blank, the engine's own `specdiff::spectrum_diff` (the page's `/api/spectrum` with `bgk`: `diff:true`, lines where the difference is above zero, factor 1, 5 ppm at high resolution). The blank's window is `--minus-rt`, else the window of `--rt`, else (one time or scan) the blank's nearest scan; `--minus-sample` alone takes another sample of FILE.
  - `spectrum get FILE --rt A,B --sum [--fill-ms T]`: the scans added (the engine's `mode=sum`), not averaged; needs a window, not for `--minus` or `--id` (size of the lazy `.wasm`). `evidence.sum` is `/api/specsum` (`specsum.rs`, index only): `n_scans`, `counts_cps` (cps summed), `by_scan_type`, `ions` = cps x fill time (null with `fill_time.note` when the file's index has no fill time; `--fill-ms` states it, never guessed), `ion_quantum`, `ions_by_quantum` (counts / quantum), `background.applied` `none`. The mean of a window of 2+ scans carries `evidence.hint` (`note.spec.mean.sumHint`).
    Site: lazy `rawxic` `.wasm` `spec_sum`; test `tests/test_spec_sum_wasm.py` (fails, not skips, in CI without tools) and `route_steps.rs` (app against `.wasm`). Exit 2 for `--sum` without a window, with `--minus` or `--id`, `--fill-ms` alone or not above zero (`err.spec.fill`). Tests `spectrum_sum_tests.rs`, `specsum_tests.rs`.
  - `spectrum get FILE --rt A,B --peak auto [--set factor=2,side_min=0.3,gap_min=0.05,min_sn=3,min_points=3] [--precursor MZ] [--fill-ms T]`: the engine's `specpeak.rs` (`/api/specpeak`, index only). A,B is where the apex is searched in the chromatogram of the level (summed intensity per scan; at a product-ion level the scans of `--precursor`, else all).
    - Window: `factor` x FWHM centred on the apex (FWHM by interpolation at half height on the raw trace, from the baseline the integrator drew under the apex). Sides: `side_min` wide, starting `gap_min` outside the window and the integrator's peak. Spectrum: the window's sum minus the mean spectrum of both sides scaled to the window's scans, per nominal mass, lines above zero.
    - `evidence.peak` is the route's answer without its lines. `verdict`: `peak`; `no_apex` (none found, or `min_sn` / `min_points` missed); `no_data`; `window_only` (a profile or high-resolution level: window and sides, no spectrum). Also `rule` (every number next to its threshold), `flags`, `params`, `apex`, `fwhm_min`, `window_min`, `sides`, `background` (`per_scan_cps`, scans), `n_scans`, `total_ions`, `sum` (the `--sum` evidence of the window, `background.applied` `sides`).
    - Flags: `edge` (apex on the first or last scan of the range), `fwhm_cut` (half height not reached inside the trace), `no_background`, `one_side`. `total_ions` is the sum's `ions` (null without a fill time).
    - Without an apex no spectrum is made: no points, exit 0, the verdict and its rule in the document and on stderr. Not with `--sum`, `--minus`, `--id` or a single time; `--set` and `--precursor` need `--peak`; a bad `--set` value is `err.spec.peak` (exit 2).
    Site: lazy `specpeak` `.wasm` (`mzlab-specpeak-wasm`, a module of its own so that `rawxic` keeps its cap) `spec_peak`; test `tests/test_spec_peak_wasm.py` (fails, not skips, in CI without tools) and `route_steps.rs` (app against `.wasm`). Tests `spectrum_peak_tests.rs`, `specpeak_tests.rs`.
  - `evidence.minus` names the file, sample, window, scan, factor, clip and tolerance; `--minus` is not for `--id`; exit 2 usage, 1 `err.spectrum.minus` (the engine cannot answer), 4 a missing blank. Test `spectrum_tests.rs` (equals `spectrum_diff` on the same files).
- An unknown channel id or scan: exit 5 (`err.canali.unknown`, `err.scan.unknown`). The `.wiff` inventory command is `file inventory [--full]`. Schemas: `docs/schemi/{trace,trace-set,xicplan,spectrum,scanlist,uvband,file_info}-1.json`.
