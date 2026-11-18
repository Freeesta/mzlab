# Map of loading a file (site and desktop app)

A snapshot of the code, not a plan: where a file goes from the click to the screen, what it costs, what is missing. Where the area guide disagrees with the code it is written under "Doubts". "Not found" = searched for and absent. Details already written elsewhere are not repeated: `startup.md` (page start-up), `progress.md` (progress bar), `native-reading.md` (`.raw`, `.wiff`, `.D` without mzML), `readers-module.md` (modules per family), `mzidx.md` and `rust-mzidx.md` (index and pool), `rust-desktop.md`
(desktop), `capabilities.md`, `fast-viewer.md` (after opening: XIC, zoom), `measurements-speed-site.md` and `optimizations-tried.md` (numbers and routes already discarded: read them BEFORE optimizing), `measurements-machines.md` (opening on slow machines).

## 1. Site: the common path (mzML)

| # | Step (file:function) | Where it runs | What it does / what it reads |
|---|---|---|---|
| 1 | `shell.js` `#drop`/`#pick` (`onchange`, `drop`) → `upload()` (308) | page, async | Gate `AVVIO.pronto()`: files that arrive before the engine stay queued. `dropDups` removes duplicates. Folders: `cartella-carica.js` (`fromDrop`, `fromInput`) or `cartella-puntata.js` (File System Access). |
| 2 | `upload0` (314) | page | `wiffToMzml` 184 and `rawToMzml` 179 analyse what the engine reads by itself, then `moduloToMzml` 227 converts the readers-module files one at a time. Up to 50 files, each file goes to `upload0` (`fileKind`, `PROGRESSO.begin`, `CONVROWS` row, `one` read) as soon as it is ready: the first sample opens while the following ones convert (`e2e_apertura_pipeline`). Beyond 50 files, first the "Choose" question, then the whole batch: `Promise.all(files.map(one))` in the browser, otherwise one at a time. |
| 3 | `one` → `sendFile` → `fetch POST api/upload` → `browser.js` → `lr.js` `upload` (84) | page | The browser has no network: `browser.js` intercepts `fetch("api/...")` and forwards it to the engine. Rejects files > 4 GB (`err.memory.over4gb`). The `File` goes to the worker by reference (no copy). |
| 4 | `rust-worker.js` `lrPut` → `rust-carica.js` `copyPrints` | engine worker, async | `ARCH.ensureRoom` (space ×1.8 for file + index), removes the old index of the same name (238), copies into OPFS in chunks (`blob.slice().arrayBuffer()`, `putChunks`; "Cancel" stops the copy at the next chunk and the promised space is freed on every error). For a `.wiff` sample the space requested covers only the members not yet in OPFS. In the same pass SHA-256, SHA-1 and `probe` go to `impronta-worker.js` (≤4 chunks waiting). Bar 0-40 %, updated every 100 ms. Reads the WHOLE file once. |
| 5 | `lrIndex` (277) → `prepare_upload` (`mzlab-wasm/src/lib.rs` 624 → `upload::prepare_in`) | engine worker, SYNCHRONOUS (answers nothing else) | Scan table: with a valid `indexListOffset` (`scantable::read_listed`) it reads the header, the last 2 KB, the index and ≥4 KB per spectrum (never the arrays); without an index (`read_walked`) it reads the file in 1 MiB blocks. Fingerprints already known: no re-read. One pass: `measurements-one-pass.md`. |
| 6 | `earlyTicOf` (258) | engine worker | From the table it derives the declared rt and TIC and sends `progress {early_tic}`: while no file is open the workspace draws the PROVISIONAL TIC of the first file being loaded, with its `TDATI` table (`tic-provvisorio.js`, loaded at the first `early_tic` by `area-lavoro.js`, `#ticprov` under the drop zone, which therefore does not move, goes away with the entry page, not before, or when loading ends); `getChrom` (`explore.js`, `provvisorio:true`) uses it for an already-open file. Test: `e2e_tic_provvisorio`. |
| 7 | `idxPool().build` → `idx-worker.js` `build` + `build_mzidx` | pool of short-lived workers (min(cores−1, 4); 2 on iPad/phone) | Reads the OPFS copy with `SyncAccessHandle` (`lettura-sync.js`) in 4 MiB blocks (`BLOCK`, `mzlab-wasm/src/lib.rs` 74), decodes each spectrum ONCE, sorts by m/z into buckets; peaks beyond `budget` (64 MB, 32 on iPad) go to an OPFS spill file `.sp.part` (~20 B/peak). With isolation (SharedArrayBuffer) up to 4 helpers decode base64/zlib (`idx-aiuti.js`). Writes `.mzidx.part`, then `ARCH.promote`. Bar 40-100 %. |
| 8 | `lrOpen`, `lr_entry`, `mergeFiles` | engine worker + page | Opens the index (header, meta, scan table; the peaks stay in the file and are read in windows) and fills the load table. No peaks in memory. |
| 9 | file read → `OPENNOW` (`open-now.js`) | page | Each file enters the session as soon as it is read, hidden; the first non-blank one shows itself (`showFirst`, `bootSession`), the others are shown with a click on a name (side list or sample table). Files waiting are grey rows (`E.pending`). No button to press: `upload()` and opening proceed together. |
| 10 | `bootSession0` (`explore.js` 424) → `getChrom` TIC → `rust-bridge.js` `indexRoute` | page → worker | First chart: TIC from the index (`op:"tic"`). The result is a `Float64Array` TRANSFERRED (not copied) from the worker, returned with `recycle` after drawing (`rust-worker.js` 24, `rust-recycle.js`). Response cache `CACHE` (`cache-dati.js`), byte-based LRU, 256 MB (96 MB if `deviceMemory` ≤ 2). `CHROM1` orders the first requests: the first chromatogram before the rest. |

Reopening (same page, next day): `lr_boot` (`rust-worker.js` 309) opens every `.mzidx` already in OPFS (`lrOpen`) without reading spectra; an mzML without an index (interrupted load) is re-indexed (`lr.js` `boot`). The table (`shell.js` `restoreRows`) reopens the files the notebook does not name (still loading when the page went away) (more than 50: unticked, as after "Choose"); a file that is named but not open (removed by the student) stays unticked (`tests_e2e/e2e_ricarica_a_meta.py`). Reloading the same name with `lr_put` instead deletes the index and redoes everything.

A single engine worker serves ALL requests (`rust-worker.js` `handle`): a long call (like `prepare_upload`) delays the other calls of the same worker, not the page. If it dies: `rust-supervisor.js` / `rust-bridge.js` `died` restarts it and reopens the slots; WASM memory never returns to the system, so `rust-recycle.js` closes a bloated worker after 30 s of quiet (threshold ≥192 MB).

## 2. By format

| Format | Extensions | Path on the site | Path on the desktop | What it reads on opening |
|---|---|---|---|---|
| mzML | `.mzml` | §1, `.wasm` engine | `lr_index` → `IdxBuilder::build` (`idxbuild.rs` 131): `mmap`, `prepare_parallelo`, `build_mzml_spill` | header, index, headers (§1 step 5); arrays only in the index |
| mzML with MS-Numpress | `.mzml` | the engine rejects (`err.file.numpress`); the page opens it as a native sample (`numpressToNat` in `shell.js`) and the engine indexes it from the `aperti` module reader (`open/mzml_numpress.rs`), without rewriting the mzML | CLI: `flow_pace::is_numpress_mzml` sends the index to the registry's `mzml-numpress` reader; app: `moduliToNat` (`shell.js`) → native sample (`natsamples`, `lr_put` with `nat`; `tests/numpress_mzml.rs`, "module types" step of `e2e_shim.py`) | table (`scantable::read`) + bytes of each spectrum |
| mzXML | `.mzxml` | `aperti` module only (`soloModulo`, `lettori-famiglie.js`): conversion to mzML in OPFS, then §1 | native sample (`moduliToNat`, registry `mzxml` reader); `pick_files` offers it | `mzxml_index.rs`: one sequential pass in 1 MiB blocks (the final `<index>` is not used) |
| Thermo `.raw` | `.raw` | `thermo` module loaded on demand (`lettore-esterno.js`, `carica-modulo.js`, SHA-256 verified), the engine reads it via `native_external` WITHOUT mzML; v2 index from the reader | reader in the core (feature `thermo`), `build_nat` (`idxbuild_nat.rs`) | `rawreader.rs`: indices and event table, no peaks; packet by offset on demand; 64 KB window (3 blocks of 256 KB in the module) |
| Waters `.raw` (folder) | folder with `_FUNC001.DAT` | `waters` module (conversion to mzML) | native reader | `_FUNCTNS.INF` and indices; `.DAT` by position |
| SCIEX `.wiff` + `.wiff.scan` | pair | engine (3200 QTRAP) as native sample; variants → `sciex` module | `build_nat` | WHOLE `.wiff` in a `Vec` (cap `Limiti`), `.wiff.scan` never whole (in blocks); `apri_solo`: one sample |
| Agilent `.D` | folder | base engine; variants (QqQ MRM, Q-TOF, compressed) → `agilent` module | native | whole `MSScan.bin` (with cap), `MSPeak.bin` by offset. `.D` folders on the desktop: no opening path found (`folder.rs`, `opening.rs` exclude them) |
| Bruker `.d` TDF/TSF | folder | `bruker` module | native | SQLite tables, frames by offset; `.baf` rejected (`err.file.brukerBaf`) |
| Shimadzu | `.qgd`, `.lcd` | `shimadzu` module (conversion) | native (`moduliToNat`) | WHOLE file in memory (`dim*2 > limiti.byte` rejects); not by offset |
| ANDI/netCDF | `.cdf`, `.nc` | `andi` module | native (`moduliToNat`) | whole columns of the scan variables |
| UV/DAD | inside mzML or manufacturer files | `mzml_dad.rs` (`dad_matrix` read from the tail, or `MS:1000804` spectra by steps); vendors: `dad_map` of the `Lettore` trait; `channels_uv.rs` serves `uv:twc`, `uv:<nm>` | same code | `.uv` as a file: not found |
| `.mzidx` | index | only as a derivative in OPFS (`idx/`); as a file to open: not found | only as a cache (`app_cache_dir()/mzidx`) | 64 B header + meta + scan table; peaks in 64K windows (`idxwalk.rs`); version > 2 → `error.newerFormat` |
| `.dam` | SCIEX method | `lr_put` without an index (`rust-worker.js` 231) | same | read on demand |
| mzMLb, imzML | – | not found | not found | – |
| Libraries `.msp`, `.mgf`, `.json`, MassBank, `.mzlib` | – | not raw data: `mzlab-lib` (`parse.rs` `detect`; `.mzlib` read by directory, blocks decoded on demand); `libreria-worker.js` | – | `libraries.md` |
| `.mzfeat`, `.mzflow` | – | derivatives/plans, not raw data (`file-sicuri.js`: 8 MB, depth 32; beyond, `workflow.err.too*`) | – | – |

The site engine has in its registry only base Agilent `.D` and SCIEX `.wiff` (`readers.rs` 270, `Limiti::sito()` 192 MB); every other family is a separate `.wasm` (`crates/mzlab-lettori-wasm`), loaded only when a file of that family arrives (`lettori-modulo.js`, `lettori-worker.js`: ONE worker for the conversions, no internal parallelism found). Module→engine transport: `transport.rs` (`PROTOCOLLO` 2, binary columns).

Native samples (`.raw`, `.wiff`, `.D`): `lr_put` with `nat` copies the members into OPFS (no mzML), then `build_mzidx_native` writes the v2 index (phases copy .2, sha .1, open .1, scans .35, chrom .25: `nativo-fasi.js`); `natsame` recognises an already loaded `.wiff` pair or `.raw` ("from cache"). Details: `native-reading.md`.

## 3. Desktop app

- **Entry**: click on `#drop` or `#dropdam` intercepted by `shim/desktop-shim.js` → Tauri command `pick_files` (dialog in `spawn_blocking`, filter from the readers registry); drag and drop: `register_paths` (`main.rs` 116, `metadata` only); double click from the system: `open.rs` (`single_instance`, argv, macOS `RunEvent::Opened`; `Pending` queue, `opening.rs`; only registry extensions). The page never sees the path: `State::pick` (`engine.rs` 137) assigns a number, `toFile` creates an empty `File` with `type=application/x-mzlab-file;id=N`. No data is read in this step.
- **Transport**: `desktop-worker.js` (JS Worker thread) does `fetch` on `mzlab://` (POST with JSON; JSON or binary answers for `tic`/`xic`/`spectra`, `scanbin`, `idxxicbin`, `natsrm`); a single message queue (`line`), only `lr_index`, `lr_index_cen`, `nat2mzml`, `mzml2cen` bypass the queue. On the Rust side every request has a `std::thread::spawn` (`main.rs` 456). `State::rpc` (`rpc.rs` 340): long operations run WITHOUT the `Mutex<Eng>` lock; `lr_put`, `lr_entry`, `idxopen`, `route`, `scanbin` take it and are serial.
- **mzML**: `lr_put` (`rpc_lr.rs` 65) does not copy (size+mtime stamp in `store.rs`), except for an app temporary file (`fs::copy`); deletes the index of the name. `lr_index` (`rpc.rs` 125) → `IdxBuilder::build`: `Mapped::open` (mmap, `mapped.rs`), `upload::prepare_parallelo` (table + SHA-256 on a second thread), `ram::check_index`, `mzidx::build_mzml_spill` (budget 64 MB, `.sp.part` spill), writes `.part` then `rename`. Rayon pool: min(cores−1, 4), 1 minimum (`idxbuild.rs` 23-28). Decoding with
  `parse_helped` (3 threads, `mzml/help/pool.rs`) and bucket sorting up to 8 threads (`mzidx_order.rs`).
- **Natives**: no copy of the members (mapped); `build_nat` (`idxbuild_nat.rs` 207) with fingerprints on a second thread; `lr_entry` reads from `Src::Nat` (`natives.rs` 91): at most 4 open readers and 32 MB of shells, LRU (`natives_slots.rs`). After loading, every sample enters the background conversion queue (`conv_queue.rs`: cores−1 threads, waits with 100 ms sleeps while indexing).
- **Index cache**: `app_cache_dir()/mzidx/<name>.mzidx` (`startup.rs` 17-20), validity size+mtime (`store.rs` 107, `lr_boot` `rpc_lr.rs` 51-63), then `Index::open` checks magic and size. On a cold start (new process) the code does `?riparti=1` → `lr_clear` which removes the indices (`rpc_lr.rs` 182): the index does not survive a restart (to be confirmed by hand).
- **Later reads** (`mzmlroute`, `scanbin`, `specexact`): `map_file` (`rpc.rs` 399) redoes `open`+`mmap` on every request, no map cache.
- **Memory**: budget `native_auto_mb` = RAM/4 between 1024 and 16384 MB (`ram.rs`, `budget_auto.rs`); `check_index` rejects first with `error.memoryBudget`. `ram_mb()` reads `/proc/meminfo` or `sysctl`: on Windows probably 0 → 1024 MB (not verified). `Limiti::desktop()`: 1024 MB, 50 M entries. The old `State::open` (`engine.rs` 165) reads the WHOLE file into RAM (`mzml::read`/`raw::read`) and is only a fallback.

## 4. Cancellation, waiting, limits

- **Site, Cancel** (`cancelLoads`, `shell.js` 298): aborts the files not yet started (`LOADS`), stops the conversion queue and sends `lr_cancel` → `CANCELLED` + `idxPool().cancel(name)` which terminates the index worker (`idxpool.js` `cancel`); `lr.js` removes the placeholder (`lr_remove`, `err.load.cancelled`). The copy of a plain mzML checks `CANCELLED` at every chunk (`rust-carica.js` `copyPrints`); `prepare_upload` is synchronous and cannot be interrupted: it is checked before `lr_index`
  (280) and the builder dies midway. The terminated worker still holds the copy and the `.part` files for a moment: the copy (`lr_remove` of a name in `CANCELLED`) and the `.part` files go with `ARCH.removeHeld`, which retries for about 5 s (`tests_e2e/e2e_annulla_orfani.py`). A blocked engine worker stops only with `abort` (`rust-bridge.js` `cancelable`), which restarts it.
- **Queued copies** (site): 2 at a time, in line (`createCopyGate`, `?copie=N`; 0 = all).
- **Desktop, Cancel**: `lr_cancel` (`rpc.rs`, outside the shim queue) → `IdxBuilder::cancel`: the builds of `name` and `name.cen` fail at the next read or write (`Fermabile` on source, index and scratch) with `err.load.cancelled` and no file; `lr.js` calls `lr_remove`, which also removes the indices; `lr_put` restarts (`resume`). A native sample stops at the first write (the reader is not wrapped). `natsamples`, `nat2mzml` have no flag; `conv_cancel` stops only the queued jobs; stopping a `nat2mzml` stops the JS worker but the Rust write finishes.
- **What the user sees**: loading rows with bar and phase (`CONVROWS`, `progresso.js`, event `mzlab-index {name, frac, phase, done, total, wait, early_tic}`), elapsed time every second, "no news for N s" after 5 s, "queued" (`wait`) if the pool is full, no veil over the page (progress is only in the side list); provisional TIC (step 6); map with grey preview (`data-map.md`). Desktop: the worker sends progress only from `rawprogress` (polled every 300 ms) for `nat2mzml`/`mzml2cen`; for
  `lr_put`/`lr_index` it emits no `progress`, while `lr.js` (93-98) waits for them (what the bar draws: not verified).
- **Site memory**: WASM engine up to 4 GiB, working budget by levels 96…3072 MB, default 192 (`budget_auto.rs`, `rust-budget.js`, automatic choice); index: `builderBudget` (at least 64 MB, 32 on iPad); scan table ~1.5 KB/scan (145 MB for 100 000); pool `error.memoryBudget`: `rust-mzidx.md`; OPFS quota reserved first (`ensureRoom`, `error.quota`); file > 4 GB rejected. Without isolation the builder decodes alone and an mzML ≥ 32 MB shows the `indiceParallelo` warning (`capacita.js`). Heavy test: `tests_e2e/e2e_carica_grandi.py`, `e2e_wiff_240.py`.
- **Copies along the path** (site): disk → `Blob` (reference) → OPFS (one JS copy per chunk: `arrayBuffer()` then write) → every `read` of the Rust reader: JS `Uint8Array` + `copy_to` into the WASM buffer (`JsReader::read_at` `lib.rs` 179; `Source::read` of `scantable.rs` 450 allocates a `Vec` on every read) → results to the page: `Float64Array` transferred. In addition `prepare_upload` copies the tables into `Uint8Array` (rt and tic twice, `lib.rs` 624) and `build_mzidx` redoes `to_vec`/`collect`
  (`levels`, polarity, rt, tic). Desktop: `mmap` with no copy towards Rust; useless copies: `f64_bytes` (`protocol.rs` 22), `idxxicbin` (`rpc.rs` 273, two `Vec`), `decode_inputs` (`idxbuild.rs` 55), `Blob → ArrayBuffer → POST` for an untagged `File` (`desktop-worker.js` 34-39, in 32 MB chunks).

## 5. Strategies: present?

| Strategy | Status | Where / note |
|---|---|---|
| Streaming (never all in memory) | present | chunked copy into OPFS; builder reading in 4 MiB blocks; disk spill for peaks. Exceptions: `.wiff`, Shimadzu and netCDF whole in memory; the old desktop `State::open` |
| Lazy reading with an index | present | `.mzidx`: scan table in memory, peaks in windows (`idxwalk.rs`), TIC/base peak inside the index (opening reads no peaks); mzML with an offset index: headers only |
| First approximate picture then refined | partial | provisional TIC from the table (step 6); map preview. No approximate XIC/spectrum; a `.raw` already has one during the copy (`tic-nativo.js`) |
| Workers / threads | present | site: engine worker, index pool, helpers, fingerprint worker, module workers; desktop: rayon, thread per request, conversion queue. The page does no heavy I/O |
| Preloading the next / the likely one | partial | engine and `.wasm` start at `DOMContentLoaded` (`browser.js`), manufacturer scripts cached by the service worker, Ketcher and 3D map at idle, `XICBTN.prefetch`; queued files are indexed by the pool BEFORE opening them. No speculative preload of data TIC/XIC/spectra: not found (routes tried in `optimizations-tried.md`) |
| Cache of decoded data | partial | yes: `.mzidx` on disk (the decoded data sorted by m/z), `CACHE` LRU of responses, map tile pyramid in OPFS (`rust-worker.js` 39-70). No: decoded spectra, mmap maps (desktop), desktop index after a cold restart |
| Parallel decoding | present | SharedArrayBuffer helpers (only with isolation), `parse_helped` (3 threads), sorting up to 8 threads (native), mzML encoding in a pool (`mzmlwrite/pool.rs`); the engine `.wasm` stays sequential where there is no isolation |
| Zero-copy Rust/WASM/JS | partial | numeric results transferred and recycled; `Blob` by reference; `mmap` on the desktop. One copy for every `read` towards WASM and every chunk written to OPFS; tables copied in `prepare_upload` |
| Compact structures | present | column-based `.mzidx` with `fence` for blocks of 16384, HR MS1 in `f32`, binary column transport of the modules (`transport.rs`), `xicbin`; table rows as compact JSON |
| Cancellation | partial | site: queue, copy and builder yes, `prepare_upload` no; desktop: index builds yes (`lr_cancel`), `natsamples` and `nat2mzml` no |
| Memory limits | present | §4; module caps (`tools/build_lettori_wasm.sh`), engine `.wasm` ≤ 2 174 976 B |
| Faithful progress | present on the site, partial on the desktop | `progress.md`; desktop without events for indexing |

## 6. Doubts and discrepancies found (verify before acting)

1. `mzidx.md` says the engine recognises the index from size and `probe`; in the code read, `lrOpen` (`rust-worker.js` 116) uses the NAME and `Index::open` checks the magic and the index `total`; a comparison of `probe`/`sha256` with the mzML on reopening: not found (`infoDi` reads `sorgente_sha256` only as information). A file with the same name but different content is discarded because `lr_put` deletes the index, not because of a comparison.
2. `rust-desktop.md` says the index is `<size>_<name>`; the code writes `<name>.mzidx` (`idxbuild.rs` 84): two mzML files with the same name overwrite each other.
3. Desktop: no `progress` for `lr_put`/`lr_index`; index that does not survive a cold restart; budget on Windows probably 1024 MB because of `ram_mb()`; more threads than the declared cap of 4.
4. Desktop: `.D` folders and `.mzidx` as input: not found; not traced what the page shows when a `.D` folder is dragged in.
5. The site's work plan after `lr_index`: `lr_entry` and `lr_items` re-read the index for every file (sequence per file in `lr.js` `explore`): not measured how much this weighs with many files (see `measurements-machines.md`, opening at 6×).

## 7. Batches for Gemini (priority: what weighs most on opening time)

Use: `python3 tools/batch_gemini.py docs/agenti/map-loading.md --lotto 1 -o lotto1.txt` (warns if the batch exceeds `--max-kb`, 200 by default; `--controlla` prints the sizes). Sizes in KB in parentheses in the `--controlla` list; the batches are ≈150-200 KB.

@lotto 1
crates/mzlab-core/src/upload.rs
crates/mzlab-core/src/upload/one_pass.rs
crates/mzlab-core/src/upload/late.rs
crates/mzlab-core/src/upload/not_mzml.rs
crates/mzlab-core/src/scantable.rs
crates/mzlab-core/src/mzml.rs
crates/mzlab-core/src/mzidx.rs
crates/mzlab-core/src/mzidx_reader.rs
crates/mzlab-core/src/mzidx_spill.rs
crates/mzlab-core/src/mzidx_order.rs
crates/mzlab-core/src/idxread.rs
crates/mzlab-core/src/idxwalk.rs
crates/mzlab-core/src/idxcheck.rs
crates/mzlab-core/src/source.rs
crates/mzlab-core/src/scanrow.rs
crates/mzlab-core/src/window.rs
crates/mzlab-core/src/blocks.rs
crates/mzlab-core/src/budget.rs
crates/mzlab-core/src/estimate.rs
crates/mzlab-core/src/readers.rs

@lotto 2
mzlab/web/rust-worker.js
mzlab/web/rust-bridge.js
mzlab/web/rust-nativo.js
mzlab/web/rust-carica.js
mzlab/web/rust-nativo-pagina.js
mzlab/web/idx-worker.js
mzlab/web/idxpool.js
mzlab/web/idx-spill.js
mzlab/web/idx-aiuti.js
mzlab/web/idx-aiuto.js
mzlab/web/archivio.js
mzlab/web/impronta.js
mzlab/web/impronta-worker.js
mzlab/web/lettura-sync.js
mzlab/web/rust-opfs.js
crates/mzlab-wasm/src/lib.rs
crates/mzlab-wasm/src/natives.rs
crates/mzlab-wasm/src/natives_engine.rs
crates/mzlab-wasm/src/counted.rs
crates/mzlab-wasm/src/courier.rs
crates/mzlab-wasm/src/cache.rs

@lotto 3
mzlab/web/shell.js
mzlab/web/lr.js
mzlab/web/browser.js
mzlab/web/carica.js
mzlab/web/progresso.js
mzlab/web/avvio-progressivo.js
mzlab/web/nativo.js
mzlab/web/nativo-fasi.js
mzlab/web/conversione.js
mzlab/web/cartella-carica.js
mzlab/web/tipo-file.js
mzlab/web/file-sicuri.js
mzlab/web/chrom-primo.js
mzlab/web/cache-dati.js
mzlab/web/first-paint.js
mzlab/web/telefono.js

@lotto 4
mzlab/web/lettori-modulo.js
mzlab/web/lettori-worker.js
mzlab/web/lettori-famiglie.js
mzlab/web/lettore-esterno.js
mzlab/web/carica-modulo.js
mzlab/web/cartella-puntata.js
crates/mzlab-wasm/src/external.rs
crates/mzlab-lettori/src/lib.rs
crates/mzlab-lettori-wasm/src/lib.rs
crates/mzlab-lettori-wasm/src/reading/session.rs
crates/mzlab-core/src/transport.rs
crates/mzlab-core/src/rawreader.rs
crates/mzlab-core/src/wiff/reader.rs
crates/mzlab-core/src/agilent/reader.rs
crates/mzlab-lettori/src/waters/reader.rs
crates/mzlab-lettori/src/shimadzu/mod.rs
crates/mzlab-lettori/src/andi/ms.rs
crates/mzlab-lettori/src/open/mzxml.rs
crates/mzlab-lettori/src/open/mzxml_index.rs
crates/mzlab-lettori/src/open/mzml_numpress.rs

@lotto 5
crates/mzlab-desktop/shim/desktop-worker.js
crates/mzlab-desktop/shim/desktop-shim.js
crates/mzlab-desktop/src/main.rs
crates/mzlab-desktop/src/engine.rs
crates/mzlab-desktop/src/rpc.rs
crates/mzlab-desktop/src/rpc_lr.rs
crates/mzlab-desktop/src/protocol.rs
crates/mzlab-desktop/src/store.rs
crates/mzlab-desktop/src/idxbuild.rs
crates/mzlab-desktop/src/idxbuild_nat.rs
crates/mzlab-desktop/src/mapped.rs
crates/mzlab-desktop/src/ram.rs
crates/mzlab-desktop/src/natives.rs
crates/mzlab-desktop/src/natives_slots.rs
crates/mzlab-desktop/src/conv_queue.rs
crates/mzlab-desktop/src/open.rs
crates/mzlab-desktop/src/opening.rs
crates/mzlab-desktop/src/startup.rs
crates/mzlab-desktop/src/folder.rs

@lotto 6
mzlab/web/explore.js
mzlab/web/rust-supervisor.js
mzlab/web/rust-recycle.js
mzlab/web/rust-budget.js
mzlab/web/capacita.js
mzlab/web/memoria-avviso.js
mzlab/web/avanzamento.js
mzlab/web/sw.js
mzlab/web/rust-scans.js
mzlab/web/rust-xicbin.js
mzlab/web/rust-centroidi.js
mzlab/web/peaks-worker.js
crates/mzlab-core/src/routes.rs
crates/mzlab-core/src/hrscans.rs
crates/mzlab-core/src/fileroutes.rs
crates/mzlab-core/src/lr.rs
crates/mzlab-wasm/src/items.rs
crates/mzlab-wasm/src/scans.rs
crates/mzlab-wasm/src/info.rs
crates/mzlab-wasm/src/xicbin.rs
