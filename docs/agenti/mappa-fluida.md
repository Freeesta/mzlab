# Fluid map: study of the RT×m/z map transitions

Measurements, diagnoses and open work for zoom, pan, crosshair, file switching, scale and 3D of the 2D map, with synthetic files up to HRMS scale. The map functions are described in `dati-mappa.md`.

## How to repeat the measurements
- `MZLAB_CHROMIUM=<chrome> python3 tools/bench_map.py lr,hrc,hrp --wd DIR [--json]`: creates the synthetic mzML files in `DIR` (not in the repository), builds the site with the WASM engine (`build_site.py` + `cargo build` + `wasm-bindgen`), opens two files per case in headless Chromium with a persistent profile (the quota of an incognito context rejects the 300 MB files) and measures the gestures. `--json` prints one line per case.
- Cases: `lr` 1500 scans × 500 peaks (0.75 M peaks, 14 MB); `hrc` 3000 × 3000 centroids (9 M, 150 MB); `hrc_big` 6000 × 5000; `hrp` 400 × 50 000 in profile (20 M, 320 MB); `hrp_big` 600 × 150 000. `name:SxP` changes the sizes.
- Script hooks (only in the test browser): `fetch` of the `api/map|spectra|xic` routes, `drawMap`, `mapImage`, `MAPPA.zoomImg` (true = sharp region drawn), intervals between `requestAnimationFrame` and long tasks, WASM memory peak from the worker messages, Chromium RSS from `/proc`.
- Reading: `rdStat` of `rust-worker.js` (field `rd` of every response) counts time, bytes and bytes from handle inside the readers; the script reports them (`first_read` cumulative, `read_ms` per gesture). `rdStat.ops` (field `rd.ops`) lists the last 300 requests as [op, path or name, ms until the response, ms in the readers, reads]: requests interleave at every `await`, so it is a profile, not an exact measurement.
- The lab operations (`lr_metodo`, `lr_method`, `lr_analog`, `lr_pda`...) read the headers of hundreds of mzML scans from a `SyncAccessHandle` (`rust-worker-lab.js` `open`), falling back to `FileReaderSync` if the handle is taken.
- Gestures: first full-screen map (route, `JSON.parse`, base64, colour); 12 wheel notches; pan with Shift+drag; zoom in RT only (Shift+wheel); deep zoom (2 min × 2 m/z); cold file switch, return and warm again; 60 crosshair steps; 20 pans (`CACHE` entries and JS heap added).
- Native comparison: a small binary outside the repository that opens the `.mzidx` with `Index::open(File)` and calls `Routes::map_route` with the same queries (one cold round and one warm).

## The data path (what is in memory)
1. **Index** `.mzidx` in OPFS: the MS1 peaks sorted by m/z (mz, intensity, position; 12 B/peak in HR), with a fence every 16384 peaks. The worker reads it from a `SyncAccessHandle` (`archivio.js` `lease`/`leaseRead`, `rust-worker.js` `readerFor`) into a reused buffer, in chunks of 65 536 peaks (`idxread.rs` `each_peak`). The handle, exclusive, closes after 2 s without reads and before writing, moving or deleting the file, and `wake()` reopens it at every request; without a handle it falls back to `FileReaderSync`.
2. **Full map** (`api/map` without a window): the session grid (≤600 RT rows, 1 Da cells even in HR) built by reading ALL the peaks of the file; Rust keeps the last 6 per file (`ion_map`), the page remembers them in `memo`.
3. **Region** (`ionmap_region`, at most 2048 × 1024): reads all the peaks of the m/z band for the whole time and discards those outside RT, so a zoom in RT only reads the whole file.
4. **Transport**: `Vec<f32>` → base64 → JSON text → `postMessage` → `Response` → `r.json()` → `MAPDATI.b64f32` → `Float32Array`: about six copies, 1.33 times the size.
5. **Colour** (`grafici.js` `mapImage`): `Float32Array`, 99.5th percentile from `MAPREND.ref` (`mappa-render.js`), precomputed palette written at 32 bits per pixel; the image lives in a single place per panel (`p._img`), so every file switch recolours everything.
6. **Regions in memory**: byte-based LRU of the regions (`REG`/`regionKeep` in `mappa.js`: 96 MB, 48 MB with `deviceMemory` ≤ 2) that also removes the `CACHE` entry.
7. **Drawing**: `draw(p)` at every input event; the crosshair overlay is redrawn once per frame (`frame`, `FRAME.next`); the sharp region starts after an adaptive silence (`quiet`: 40 ms if the last route is under 30 ms, otherwise `QUIET` 150 ms).
11. **Tile pyramid** (`maptiles.rs`, `api/maptiles`): levels of maxima on a snapped grid, sparse 256 × 256 tiles, equal to `region_map` on aligned grids (`tests/maptiles_parity.rs`); the worker saves it in OPFS (`mzlab/tiles/<file>.L<level>.mzt`, `tilesEnsure` in `rust-worker.js`). Proof `e2e_hr_mappa`. The page does not yet ask for it for zoom or for 3D.
8. **Crosshair**: own LRU of spectra (64 MB, 32 on iPad) with read-ahead: it is adequate.
9. **3D** (`mappa3d.js`): the same region route up to 1024², so it inherits point 3.
10. **Thermo `.raw`** (external reader): the scans read by the `thermo` module (map, XIC outside the index, MS1/MS2 scrolling) reach the engine as a single `Uint8Array` (`lettori-modulo.md`, binary transport), without JSON meta; the `.raw` row in this table is measured on real data with `python3 tools/bench_raw.py FILE.raw --steps scansione` (µs per scan in the module).

## Measurements (headless Chromium, 4-CPU container, site with Rust/WASM engine; baseline: reading with `FileReaderSync`, colour as a function, unlimited `CACHE`)
| gesture | LR 1500×500 | HRC 3000×3000 |
|---|---|---|
| opening two files | 3.6 s | 48 s |
| first map (route / colour) | 791 ms (505 / 238) | 6332 ms (5830 / 453) |
| warm full map: route / parse / base64 / colour | 45 / 2.7 / 10.6 / 192 ms | 79 / 3.8 / 17.9 / 397 ms |
| cells and JSON of the full map | 600×521, 1628 kB | 600×901, 2816 kB |
| wheel: frame p50 / p95 / max | 16.7 / 50 / 83 ms | 16.7 / 33.4 / — ms |
| wheel: to sharp / route | 176 / 43 ms | 249 / 113 ms |
| pan: to sharp / route | 176 / 41 ms | 217 / 101 ms |
| RT-only zoom: to sharp / route | 295 / 163 ms | 1617 / 1465 ms |
| deep zoom: to sharp / route | 192 / 50 ms | 259 / 92 ms |
| file switch cold / return / warm | 456 / 219 / 211 ms | 1992 / 490 / 441 ms |
| crosshair: `api/spectra` p50 / p95 | 6.1 / 16.4 ms | 9.6 / 34.6 ms |
| RSS Chromium / JS heap / WASM peak | 1314 / 91 / 32 MB | 1931 / 103 / 175 MB |

Profile HR (`hrp` 400 × 50 000, 2 × 320 MB, 20 M points per file): in the browser the opening of the two files did not finish in 18 minutes (round stopped, no gesture measured); native, 228 MB index: full map 506 ms (18 ms from cache), region 100-1000 m/z over the whole time 316-366 ms, deep region 30 ms. The profile is the case where D1 weighs most.

Native, same `hrc` index (108 MB, file in the system cache): full map 254 ms cold and 14 ms from cache; region 100-1000 m/z over the whole time 212-273 ms; deep region 2 min × 2 m/z 29-45 ms. In the browser the same routes take 5830 ms and 1465 ms: the computation is the same code, the rest is reading.

Colour of the HRC full map (600 × 901, Node): per-pixel algorithm with function 314 ms; with sorted `Float32Array`, scale precomputed in a `Uint32Array` and 32-bit write (the one in use) 52 ms.

## Diagnosis (in order of weight)
- **D1 Index reading in the browser**: with `FileReaderSync` on `Blob.slice` every read is a copy through the browser process (HR full map 9 M peaks: 5.8 s against 0.25 s native); the worker reads from the `SyncAccessHandle` in large blocks ("The data path", point 1) and the WASM computation stays at most 2 times native.
- **D2 RT-only zoom**: the region reads the whole m/z band for the whole time (1.5 s in HR); the fence every 16384 peaks serves only m/z. The pyramid (point 11 of the path) removes it when the page uses it.
- **D3 Colour**: 52 ms for the HRC full map, but redone at every file switch (a single `p._img`).
- **D5 Long frames during the gesture**: p95 33-50 ms; one `draw` per event and overlay work (resizes the canvas, legend, table) at every draw.
- **D7 Text transport**: six copies and +33 %; a few ms of time but double memory at the peaks.
- **D8 WASM peak 175 MB in HR** with cap 256 and budget 192 (`memoria.md`): on iPad it is the nearest limit.
- **D9 3D**: like D2 for windows long in RT.
- **D10 Crosshair**: 10-35 ms per spectrum in HR with LRU and read-ahead: not a bottleneck.

## Goals
- Frames during zoom and pan: p95 ≤ 17 ms (60 fps), also in HR.
- Sharp after the gesture: ≤ 120 ms in LR, ≤ 200 ms in HR; never more than 300 ms, even RT-only.
- First map ≤ 1 s for a typical HR file (~10 M MS1 peaks); warm file switch ≤ 50 ms, cold ≤ 300 ms.
- Crosshair: spectrum perceived < 16 ms from cache, p95 < 35 ms.
- WASM peak ≤ 128 MB in HR; page memory limited by a byte budget, not by the number of gestures.

## Discarded routes
- OffscreenCanvas in the worker: with 32-bit colour the colour costs little.
- Min-max decimation: useful for 1D traces, not for a grid already at its maximum.
- SharedArrayBuffer: requires COOP/COEP, which GitHub Pages does not send.
- Higher `.wasm` cap: forbidden; handle reading and the pyramid add little code to the engine.

## Open work
Each piece of work is delivered with its `bench_map.py` measurement before and after in the PR description.
- **P2b Pyramid in the page** (D2): zoom and pan ask for `api/maptiles` at the level of the view; the exact region stays for mean, background, difference and deeper zoom. Acceptance: HRC RT-only zoom ≤ 0.2 s; `e2e_mappa*` green.
- **P3 Images per file** (D3): LRU of images per (file, scale, level) with a byte budget (e.g. 64 MB, 32 on iPad) in place of the single `p._img`. Files: `grafici.js` (`mapImage`), `mappa.js`. Acceptance: HRC warm file switch ≤ 50 ms, identical pixels.
- **P4 Binary transport** (D7): the worker returns small JSON + a transferred `ArrayBuffer`, the page builds the `Float32Array` without base64; for `api/map`, the region and 3D. Files: `crates/mzlab-wasm`, `rust-worker.js`, `rust-bridge.js`, `browser.js`, `explore.js`, `mappa.js`, `mappa3d.js`; the JSON with base64 stays for the desktop app and the parity tests. Medium risk (contract between worker and page; `MZLAB_LR_MISS` and route e2e). After P3.
- **P6 One draw per frame** (D5): the map's `draw(p)` queued on a single `requestAnimationFrame`; `MAPPA.after` resizes the canvas only if the size changes and rewrites legend and table only if the data change. Files: `mappa.js`, `grafici.js` (`drawMap`). Acceptance: frame p95 ≤ 17 ms in wheel and pan, LR and HRC; `e2e_tour` green.
- **P7 Colour on GPU** (only if after P3-P6 the frames are not at 60 fps on 2048 × 1024 grids): cells in a float texture, scale as a 1D texture, fallback to JS colour without WebGL; medium-high risk (CSP, lost contexts on iPad, same colour rendering).
- **P8 Desktop app**: `memmap2` for the `.mzidx` and `rayon` to build tiles over several files (`crates/mzlab-desktop`, feature of `mzlab-core` that the `.wasm` does not compile). Acceptance: pyramid of an HR file ≤ 0.5 s native.
- **P9 3D on the pyramid** (D9): `mappa3d.js` asks for the tiles at the level of its grid instead of `api/map`. Acceptance: opening 3D on HRC ≤ 0.3 s; `e2e_mappa3d`, `e2e_mappa_nav3d`, `e2e_mappa3d_errori` green.
- **P10 Memory budgets for iPad** (D8): WASM peak ≤ 128 MB; the cache of full maps (`mapcells.rs`, 6 grids per file) enters the budget of `mzlab-core::budget`; measurement on a real iPad (`memoria.md`).

## Limits of the study
- Measurements on a headless 4-CPU Linux container; iPad and Safari are not measured (handle reading and P10 need a real test).

## 3D map viewer (`mappa3d.js`)
Viewer of the Data view: opening errors, controls, preload, drawing cost. The 2D map is in `dati-mappa.md`.

- **If 3D does not open** (`failure3d` in `mappa.js`, `Fail`/`makeContext`/`diagnose` in `mappa3d.js`; proof `e2e_mappa3d_errori`): the window states the true cause — `nowebgl2` (no context, tried viewer options, no options, low-power; WebGL1 only to say so), `shader` (compiler line), `import` (file not downloaded or stale in cache), `error` (JavaScript error, never passed off as WebGL2),
  `lost` (lost context, message in the viewer) — with what to do in Chrome and the copyable technical lines (version, renderer, GPU, attributes, `getError`); the 2D stays intact and the test context is released immediately.
- **3D viewer** (`mappa3d.js`, ES module loaded at the first "3D", hand-written WebGL2, no library): modal dialog above the page (Esc closes, focus returns to the button) with the file and the region of the 2D (absent on phones; without WebGL2 a message).
  Retractable sidebar: Data (file, comparison A − B, numeric RT and m/z, "Use the 2D zoom"), Representation (surface | sticks for HR | spectra waterfall | contour lines on the floor; linear height (default)/root/log, vertical exaggeration, RT:m/z proportion, palette, light, 2D map on the floor, time plane), Background (threshold with histogram, "remove the background", smoothing: 0 in HR, 1 in LR), Views (from above, RT side = chromatogram, m/z side = spectrum, isometric, reset, high-resolution PNG).
  Grid up to 1024×1024 (512 on phone, iPad or `deviceMemory` ≤ 4), HR in cells of equal ppm (`mzscale=log`), LR at nominal masses; sticks from the points of `/api/mappunti3d` (200 000, 50 000 on small devices).
  Drag = rotate, Shift+drag or two fingers = pan, wheel or pinch = zoom, double click = centre (on a marked point: spectrum panel); on hover the ray from the floor to the surface (intersection on the height grid, CPU) gives RT, m/z, intensity and two boxes (XIC at that m/z, spectrum at that RT, from the grid); click = pin the point (as in the 2D it adds it to `NB.mappaPunti` only with "Mark" active); arrows = cursor of one bin (and time plane), +/− zoom, 1-4 views, R reset (also height and region), L locks the cursor, ? gestures;
  gestures and menu: "Map navigation" of `dati-mappa.md`; `prefers-reduced-motion`: no ▶ and no inertia. Viewer choices in `qqq.mappa` (`d3`); the old `t3*` keys are ignored. Measurements only: no automatic label. Tests `e2e_mappa3d`, `e2e_pannelli2`.
- **3D viewer** (same controls, same `menu()`, same `MAPPA.barFor` bar at the top right of `.m3d-main`; test `e2e_mappa_nav3d`): adapter `ad3(hit)` in `mappa3d.js`; the point is the locked cursor (`V.lock`: pointer hover does not move it, the readout says "locked"), otherwise the cell under the pointer at right click (`pick` ray) or the cursor.
  Right click without moving or long press (550 ms) or Menu/Shift+F10 = menu (the dragged right button moves the view); the page's `#ctx` is moved inside the dialog while it is open (Esc closes first the menu, then the viewer) and returns to the body on closing.
  Entries: Extract XIC/spectrum/both (the viewer closes and the extract appears in the page's XIC and spectrum panels; the file is the one chosen in the viewer), Lock/Unlock, Reset zoom = camera (35°, 32°), Top view / RT side / m/z side (keys 1-3), Copy coordinates, Gestures (sheet `mapnav.h.3d.*`). One finger rotates, two fingers pan and pinch, wheel = zoom, + / − zoom. The Ctrl/⌘K palette does not open above the modal dialog: the 3D commands are in the menu and the bar.
- **3D preload**: at the first `init` of a 2D map (`preload3d` in `mappa.js`) `static/mappa3d.js` is downloaded in the background (`modulepreload` in idle, not with `saveData` or 2g/3g), so `open3d` finds it already ready.
- **3D drawing cost**: the waterfall (`fall`) draws all rows with two `drawArraysInstanced` (the row is `uRow + gl_InstanceID * uStep` in `ROW_VS`); the `pal` palette is loaded only when it changes (`V.palTab`); picking (`pick`) with a free mouse runs at most once per frame (`hoverPick`).
