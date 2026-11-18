# Chart panel measurements (Data view)

Cost of drawing chromatograms, the map and the spectrum on dense synthetic data: `python3 tools/bench_charts.py [--rate 1|4] [--only big,many,hr,hr100,map,spec]` (site built with the engine, `tools/prepara_sito.py`; headless Chromium without a GPU, i.e. a software 2D canvas, 1500×1000, 4 CPUs; files in `/tmp/mzlab_bench_grafici`). A measurement, not a test.

- **Sets**: `big` = one SRM file with 4 transitions of 10^6 points (20 min), one shown; `many` = 20 SRM files of 10^5 points, all ticked and overlaid ("Area" fill on, the default of the MRM panels). Noise on the baseline, one Gaussian peak per transition and 6 single-point spikes.
- **Columns**: "draw" = time of `draw()` (median of 8, ms) on the full view and on a baseline-only zoom (9.45-9.62 min); gestures = calls to `draw()`, intervals between frames (median / p95 / max, ms) and long tasks (> 50 ms: count / the longest).
- **Where the time goes**: with the software canvas, a stroke wider than 1 px on a dense, jagged line (noise) costs ~70 times a 1 px line (separate test: 20 lines of 825 vertices, 694 ms against 10 ms); the transparent fill under 20 traces ~45 ms. Reading the canvas for the legend (`legende.js`) moves that work onto the main thread; one read costs as much as four.

## 20 files of 10^5 points, CPU 1x (`main` `0be060b32ecb1ba7c12be9a593b9d741df1af6f0` → 1 px line beyond half a point per pixel, `decima.js` `hairline`)
| action | before | after |
|---|---|---|
| draw, full view, ms | 39.7 | 43.4 |
| draw, baseline zoom, ms | 230.0 | 56.8 |
| Ctrl+wheel ×30: frames p95 / max, ms; long tasks | 50.0 / 300.0; 4 (300) | 50.0 / 50.0; 4 (64) |
| Shift+drag ×60 (zoomed): frames p95 / max, ms; long tasks | 233.3 / 250.0; 60 (253) | 66.7 / 83.3; 60 (88) |
| resize the window ×10: calls to `draw()`; frames median / max, ms | 15; 116.7 / 183.3 | 15; 125.0 / 183.3 |

## Waterfall view, 20 files of 10^5 points, CPU 1x (`main` `622c60ccc384e94cb663950740383717ee8af69e` → `grafici.js` draws every waterfall trace with `DECIMA.rawLine`; test `e2e_cascata_densa`)
| action | before | after |
|---|---|---|
| draw, waterfall view, ms | 2903.3 | 88.4 |
| vertices per trace (canvas of 1184 px, 60 000 points) | 60 000 (fill and line) | 1057 |

Before, every point entered the path twice (fill under the trace and a 1.25 px line); now one pixel column has one vertex (the column maximum, as in the overlaid view) and the dense line is 1 px (`hairline`). The highest peak of each trace stays the top of its line.

## Vertical zoom on the baseline, CPU 1x (`main` `2c4704f975e51dadaae010b537d45b509be23feb` → `grafici.js` passes every line through `DECIMA.clipY`; test `e2e_zoom_y_denso`)
| action | before | after |
|---|---|---|
| draw, zoom y 0-300, 20 files of 10^5 points, ms | 326.5 | 148.6 |
| draw, zoom y 0-300, one file of 10^6 points, ms | 28.0 | 28.8 |
| top of the lines above the canvas (4 files of 60 000 points, zoom y 0-300), px | ~110 000 | ≤ 9 |

With vertical zoom the peaks (10^5) ended up ~10^5 px above the canvas and the software canvas paid for the whole length of the lines (profile: the legend's `drawImage`, which flushes the pending drawing, 300 ms). Now the part outside the chart runs along an edge 9 px beyond the clip, cut exactly where it crosses it: the visible part is the same (differences only in antialiasing). The rest of the time is the noise band magnified to full height.

One file of 10^6 points (same machine): draw of the full view 11 ms, of the zoom 0.9 ms; wheel, drag and pointer hover at 16.7 ms per frame with no long tasks; resizing ×10 = 15 calls to `draw()` (frames up to 50 ms).

## HR, 120 traces of 50 000 scans, CPU 1x (`tools/bench_charts.py --only hr`; `main` `28c73e8b1aa0932cd786d7b44eaf71d4a8518269` → block maxima and minima in `decima.js`, Ctrl+wheel with `drawSoon`)
A synthetic Orbitrap file of 50 000 scans opened 6 times, all ticked; an XIC panel of 20 ions = 120 traces.
| action | before | after |
|---|---|---|
| draw, full view, ms | 105.5 | 58.4 |
| draw, zoom y 0-300, ms | 244.9 | 101.3 |
| Ctrl+wheel ×30: frames p95 / max, ms; long tasks | 116.7 / 183.3; 15 (192) | 50.0 / 66.7; 14 (77) |
| resize ×10: frames median / max, ms; long tasks | 166.7 / 200.0; 6 (190) | 83.3 / 100.0; 6 (94) |

Before, the y scale of every trace scanned all 50 000 points twice at every draw (`amax`/`amin`, 70 ms of 105) and the triangles of the vertical zoom called a function for every point of every trace even outside the window (55 ms). Now the blocks of 128 points (`rangeExtent`, `runsAbove`) give the same values reading ~1/60 of the points; `clipY` no longer creates two arrays per segment (36 ms); wheel bursts within one frame make one draw.

The rest is `m4` (~27 ms, ~38 points per column: the blocks do not help) and the rasterisation of the 120 lines. The TIC chromatogram, the map and pointer hover were already at 16.7 ms per frame; opening the 20 XICs: next section.

## Opening 20 XICs on 6 HR files of 50 000 scans, CPU 1x (`main` `2499d61b` → `rtArr` in `cache-dati.js`; 4 openings, response cache emptied between them)
| action | before | after |
|---|---|---|
| open the XIC panel of 20 ions (120 traces), ms | 1015-1225 | 558-688 |
| longest task, ms | 445-585 | 103-193 |

The scan times of a file are the same for every ion: `xicBin` (`explore.js`) copied the same 50 000 rt values into an Array for each of the 20 ions, and the garbage collector paid ~250 ms. Now an rt that is bit-for-bit equal to the last one of the same file (comparison of 32-bit words, ~10 times cheaper than a copy) returns the same Array; `isSorted` of `decima.js`, cached per array, scans it once. Smoothing into a `Float64Array` does not change the time: not done.

## Memory over repeated cycles, one synthetic LR file (`main` `1913013` → `corrInfoWire` in `pannelli.js` closes the old listener with `abort(0)`; test `e2e_memoria_zoom`)
| 30 cycles zoom → full zoom after 5 of warm-up, after GC | before | after |
|---|---|---|
| DOM nodes | 5004 → 18 084 (+436 per cycle) | 2410 → 2410 |
| JS listeners | 1213 → 3043 (+61 per cycle) | 845 → 845 |

Every zoom rebuilds the panel header; `abort()` without an argument creates a `DOMException` whose stack retains the header's NodeList, and the aborted signal stays reachable from `AbortSignal.any` on the panel's signal: every old header stayed in memory, without limit. With a reason that has no stack (`0`) nodes and listeners stay flat (probe at 50 cycles: 2797 nodes at cycle 20 and at 40).

Opening, using and closing 50 XICs instead retains ~280 nodes per cycle in the undo entries (`storia.js`), which have a cap (100 entries): it stops at ~50 cycles, JS memory stays ~9 MB; no change.

## HR table of 100 000 rows in "Statistics", CPU 1x (`tools/bench_table_hr.py`; `main` `3f613c149d136538f016a75257d92c341975859a` → dense KMD chart `kmd-nuvola.js`, "Data" table in blocks, `contain:paint` on the table boxes; test `e2e_kmd_denso`)
| action | before | after |
|---|---|---|
| table loaded and panel painted: longest frame, ms | 4211 | 423 |
| Kendrick chart: SVG elements | 500 008 | 178 |
| click on a point (XIC, profile, panel), ms; long tasks | 9164; 2 (6987) | 99; 1 (85) |
| "Data" table of the chart, ms; rows written | 10 595; 100 000 | 173; 1000 |

Where the time went: one focusable element per point (5 nodes each), the panel repainted in full at every click (`PROFILO.show` reopened the tab that was already open) and the `overflow-x:auto` box of the reference table (500 rows), which without `contain:paint` the browser re-layered at every page frame (`PaintArtifactCompositor::Update`, ~125 ms).

## HR, 20 traces of 100 000 scans, CPU 4x (mid-range tablet; `--rate 4 --only hr100,big`; `main` `c13dfa1ea6e760ce54fbc0c4f301adb8ab4d4d00`)
`hr100` = one Orbitrap Full Scan file of 100 000 scans, XIC panel of 20 ions (20 overlaid traces of 10^5 points) plus the TIC; `big` as above. Values of `main`:
| action | hr100 | big |
|---|---|---|
| open the XIC of 20 ions, ms (long tasks n / max) | 1220 (2 / 390) | |
| draw, full view / zoom / zoom y, ms | 92 / 54 / 123 | 35 / 6 / 46 |
| Ctrl+wheel x30, frames median / p95 / max, ms | 33 / 100 / 133 | 17 / 33 / 50 |
| Shift+drag x60, frames, ms | 33 / 100 / 150 | 17 / 50 / 67 |
| pointer hover x120, frames, ms | 17 / 33 / 33 | 17 / 17 / 17 |
| resizing the window x10, frames, ms | 283 / 317 / 317 | 100 / 150 / 150 |
| "Data" table of the XIC, ms (long tasks) | 575 (2 / 350) | 210 (1 / 173) |
- **Where the time goes** (CDP profile): in drawing ~55 % is software-canvas raster, `m4` ~5 ns per point (0.5 ms per 10^5 points in node; fine blocks of 16 points do not beat it at ~70 points per column). Wheel and drag already go through `drawSoon` (one draw per frame): at 4x the frame is the draw of the two linked panels.
- **Header on resize** (`fitHd` in `pannelli.js`): the two size observers (panel and canvas) ask for one fit per width, not one each; nodes that certainly do not fit (width + 32 px below the row overflow) go into the "⋯" without a reflow each. 10 steps: `fitHd` 51 → 30 calls, `hdOver` 447 → 243, time ~980 → ~505 ms (~98 → ~50 ms per step); same result as the node-by-node loop on 141 cases (widths 640-1500 px, every panel).
- **"Data" table**: it opens with 200 rows (`FIRST` in `tdati.js`; laying out 1000 in `showModal` cost ~180 ms) and `pick` in `tdati-pannelli.js` takes the window of a sorted trace by binary search (`windowIdx`): hr100 581-698 → ~195 ms (long task ~110 ms), big 189-236 → 118-178 ms.

## Map and spectrum on the hr100 file, CPU 4x (`--rate 4 --only spec,map`)
`spec` = the summed spectrum of the whole `hr100` run (730 695 centroids), `map` = the RT x m/z map of the same file (10^7 centroids). Before = `main` `8462470cd570df9da01614b1e9e2681a40640b9a`, after = bars per pixel column.
| action | spec before | spec after | map |
|---|---|---|---|
| open the panel, ms (long tasks n / max) | 2978 (4 / 712) | 1826-2186 (2 / 430-562) | 1227-1348 (3 / 145-179) |
| draw, full view / zoom, ms | 394 / 88 | 19-22 / 5-8 | 6 / 5 |
| Ctrl+wheel x30, frames median / p95 / max, ms (max long task) | 33 / 517 / 1117 (1129) | 17 / 67 / 117 (109-124) | 17 / 33 / 33 (0) |
| Shift+drag x60, frames, ms (max long task) | 17 / 167 / 217 (223) | 17 / 33-50 / 83-100 (79-101) | 17 / 33 / 50 (55-73) |
| pointer hover x120, frames, ms | 33 / 50 / 117 | 33 / 50-67 / 67 | 17 / 33 / 50 |
| resizing the window x10, frames, ms (max long task) | 1183 / 1317 / 1317 (1272) | 217 / 250-283 / 283 (256-280) | 183-208 / 217-250 (192-226) |
| "Data" table, ms (long tasks) | 189 (1 / 157) | 100-124 (1 / 77-99) | 93-150 (1 / 65-77) |
- **Dense spectrum** (`drawSpec` in `picchi.js`): with more than 4 peaks per pixel only the tallest of each pixel column is drawn (`tallCols` in `numeri.js`, kept per spectrum, window and width), as rectangles (= a flat-capped line: a path of 1000 lines cost ~40 ms of raster in the commit). The spectrum maximum (`topOf`, `topAbs`) and the window maximum (`winTop`) are kept per array; the screen-reader count (`text` in `tdati-pannelli.js`) per window. Labels take their candidates from the columns.
- **What remains**: opening the summed spectrum is the JSON of 730 000 peaks (`Response` + `json` ~430 ms in one task): it would need a binary answer from `api/spectrum` like the map's (engine + parity). Resizing is ~200 ms even with a third, single-scan spectrum (~190 ms): it is the header of every panel, not the data; the map has no hot spots of its own (colour, base64, scattered `ref`, ~30-60 ms each).
