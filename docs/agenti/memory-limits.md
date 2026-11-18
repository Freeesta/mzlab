# "The file is too large": when it happens (measurements)

Machine: Linux container 4 vCPU, 16 GB, headless Chromium 1243, persistent profile (OPFS full); Automatic budget 1024 MB safe / 3072 high, probe 3072. `--js-flags=--wasm-max-mem-pages` imitates a browser that grants less to each instance. Synthetic files (`tools/dati_sintetici.py` and the `scratchpad` generators, no real data).

| Input | Outcome | Time | Renderer at peak | Engine (linear memory) |
|---|---|---|---|---|
| 1 DDA mzML 1.0 GB, 150,000 scans | loaded | 23 s | 606 MB | 117 MB |
| 1 profile mzML 1.4 GB, 5000 scans | loaded | 33 s | 776 MB | 32 MB |
| 8 mzML of 204 MB, 240,000 scans | loaded | 18 s | 1.36-1.6 GB | 266 MB |
| the same with 1024 / 512 / 256 MB per instance | loaded (budget 768 / 256 / 192) | | | |
| 9 files (8 × 204 MB + 1 GB, 374,000 scans, 2.6 GB), 256 MB per instance | before the fix: the warning "The file is too large…" without a name, the line "index incomplete or damaged", the index deleted | | | |
| the same, 192 MB per instance | before: the engine runs out of memory (trap, "The engine ran out of memory…"), 3 files do not open | | | |
| 9 files, 256 MB per instance, after the fix | all 9 loaded | 32 s | 1.46 GB | 226 MB |
| 9 files, 192 MB per instance, after the fix | 8 loaded; the 1 GB file has "…beside the 8 already open (240000 scans)…", index kept, no trap | 32 s | 1.44 GB | 178 MB |

- Building the index is not the limit: budgeted builder + scratch in OPFS handle 1.4 GB with the engine at 32 MB.
- The limit is the page engine: it holds the scan table of EVERY open file (~1.07 KB per DDA scan, outside the cache budget) plus the cache (`min(5/6 budget, 512 MB)`). When a reservation in `Index::open` fails, it used to be a bare `error.memoryBudget` (no phase, no file): `lrIndexOwn` took it for a broken index and deleted it, `rust-bridge.js` fell back to the whole mzML and the warning said "file too large".
- Each file used to occupy 3 engine slots (list, session item, MS1/MS2 part): three copies of the same table. Now `Index.scans` is an `Arc` and `routes_open::open_slot` opens the other slots with `Index::over` on the same `.mzidx` (site: `Engine.share_index` with the same size+date stamp as `createOpenGate`; app: `idxopen` on the list slot of the same name; test `rpc_idx_tests.rs`): one table per file (estimate for 240,000 DDA scans: from ~770 to ~260 MB; the earlier refusal counted 24 slots and 720,000 scans for 8 files).
- Now (`routes_open::open_beside`, site engine and desktop app): if it fails, the cache of the other files is emptied and the open is retried once; if it fails again, the refusal is `error.memoryBudget` phase `openFiles` with `aperti` and `scansioni` ("Remove some files or load fewer at once, or use the desktop app"). The index stays, is not rebuilt and the whole mzML is not reopened (`openStored` in `rust-carica.js`, `refusedMemory` in `rust-bridge.js`; tests `carica_worker_node.mjs`, `bridge_node.mjs` M2).
- Formats read whole (fixed limit `Limits::sito().byte` 192 MB, `desktop()` 1024 MB, does not follow the budget): `.wiff` container and method reading (`WIFF_MAX`), Shimadzu `.lcd`/`.qgd` (container + stream: above ~96 MB), Agilent `MSScan.bin`/`data.ms` (sum of the members), Waters indexes, Bruker records. Above the limit: `error.memoryBudget` phase `whole` (`Limits::fits`, `fits_in`) with the MB, "Open it with the desktop app or convert it to mzML first"; before it was bare and the index pool halved and retried uselessly.
- A bare `error.memoryBudget` remains only for a reservation that failed elsewhere (`try_vec`): text "The browser gave the calculation engine no more memory. Close other tabs or some open files and try again, or use the desktop app".
- Open decisions (owner, none changed here): make `Limits::sito().byte` follow the chosen budget; count the scan tables of open files in the budget; the engine trap with 192 MB granted.
