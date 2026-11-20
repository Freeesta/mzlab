# "The file is too large": when it happens (measurements)

Measured on a Linux container, 4 vCPU, 16 GB, headless Chromium 1243, persistent profile (OPFS full); Automatic budget 1024 MB safe / 3072 high, probe 3072. `--js-flags=--wasm-max-mem-pages` imitates a browser that grants less to each instance. Synthetic files (`tools/dati_sintetici.py` and ad-hoc generators, no real data).

| Input | Outcome | Time | Renderer at peak | Engine (linear memory) |
|---|---|---|---|---|
| 1 DDA mzML 1.0 GB, 150,000 scans | loaded | 23 s | 606 MB | 117 MB |
| 1 profile mzML 1.4 GB, 5000 scans | loaded | 33 s | 776 MB | 32 MB |
| 8 mzML of 204 MB, 240,000 scans | loaded | 18 s | 1.36-1.6 GB | 266 MB |
| the same with 1024 / 512 / 256 MB per instance | loaded (budget 768 / 256 / 192) | | | |
| 9 files (8 × 204 MB + 1 GB, 374,000 scans, 2.6 GB), 256 MB per instance | all 9 loaded | 32 s | 1.46 GB | 226 MB |
| 9 files, 192 MB per instance | 8 loaded; the 1 GB file is refused with "…beside the 8 already open (240000 scans)…", index kept, no trap | 32 s | 1.44 GB | 178 MB |

- Building the index is not the limit: the budgeted builder with a scratch file in OPFS handles 1.4 GB with the engine at 32 MB.
- The limit is the page engine: it holds the scan table of EVERY open file (~1.07 KB per DDA scan, outside the cache budget) plus the cache (`min(5/6 budget, 512 MB)`).
- Each file has one scan table: `Index.scans` is an `Arc` and `routes_open::open_slot` opens the other engine slots (list, session item, MS1/MS2 part) with `Index::over` on the same `.mzidx` (site: `Engine.share_index` with the same size+date stamp as `createOpenGate`; app: `idxopen` on the list slot of the same name; test `rpc_idx_tests.rs`). The estimate for 240,000 DDA scans is ~260 MB.
- When a reservation in `routes_open::open_beside` (site engine and desktop app) fails, the cache of the other files is emptied and the open is retried once; if it fails again, the refusal is `error.memoryBudget` phase `openFiles` with `aperti` and `scansioni` ("Remove some files or load fewer at once, or use the desktop app"). The index stays, is not rebuilt and the whole mzML is not reopened (`openStored` in `rust-carica.js`, `refusedMemory` in `rust-bridge.js`; tests `carica_worker_node.mjs`, `bridge_node.mjs` M2).
- Formats read whole (fixed limit `Limits::sito().byte` 192 MB, `desktop()` 1024 MB, does not follow the budget): `.wiff` container and method reading (`WIFF_MAX`), Shimadzu `.lcd`/`.qgd` (container + stream: above ~96 MB), Agilent `MSScan.bin`/`data.ms` (sum of the members), Waters indexes, Bruker records. Above the limit the refusal is `error.memoryBudget` phase `whole` (`Limits::fits`, `fits_in`) with the MB and "Open it with the desktop app or convert it to mzML first".
- A bare `error.memoryBudget` (no phase, no file) comes only from a reservation that failed elsewhere (`try_vec`): text "The browser gave the calculation engine no more memory. Close other tabs or some open files and try again, or use the desktop app".
- Open decisions (owner's): make `Limits::sito().byte` follow the chosen budget; count the scan tables of open files in the budget; the engine trap with 192 MB granted per instance.
