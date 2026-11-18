# Interactive provisional TIC (zoom and pan of the RT axis)

The provisional TIC (`tic-provvisorio.js`, `map-loading.md` step 6) is drawn while the index is being built. Owner decision requested on the PR: before, it was a still preview by choice.

- **Controls** on `#ticprov-cv` (RT axis only; the height follows the window): Ctrl/Cmd + wheel (a plain wheel scrolls the page, as on the panels), drag, `+` `-`, arrows, `0` and double click = whole trace. One draw per frame (`FRAME.next`); 4-vertex drawing per column (`decima.js` `polyline`). A `role=status` line states the window once the gesture stops. A click repeats the progress note (no spectrum or XIC before the index). `touch-action: pan-y pinch-zoom` on the canvas only; no animation, so `prefers-reduced-motion` is respected.
- **Handover to the panel**: `TICPROV.handover()` (once) is called by `addPanel` (`pannelli.js`) for the first chromatogram without `zoom`: the panel opens on that window and a notice says so (automatic choice logged); the other panels of the view follow it through the time link.
- **Test**: `tests_e2e/e2e_tic_provvisorio.py` (wheel, drag, double click, keyboard, window of the first panel); the site and the `.wasm` are needed.
- **Desktop**: the app has no provisional TIC (direct native read, panel at once): no twin part.
- **Measurement** (`python3 tools/bench_tic_provvisorio.py`, headless Chromium, `.wasm` engine, synthetic mzML 242 MB = 20 000 scans x 3 000 peaks, cold: new browser and OPFS on every run, `idle`/`gest` runs alternated; 489 MB does not fit in the container's browser space, which stops at ~330 MB). One gesture (Ctrl+wheel, arrows) per frame while the box is visible.
  - 1x, 5 runs, 2860 gestures: event p50 0.2 / p98 0.8 ms; event to end of frame p50 16.7 / p98 19.5 ms; rAF interval p50 16.7 / p95 18.1 ms (target 16.7: p95 18.1 is just above); release to indexed 13.52 [13.03-14.41] s without gestures, 14.08 [13.87-14.90] s with the gestures.
  - CPU x4, 3 runs, 1169 gestures: event p50 0.5 / p98 2.8 ms; frame p50 17.2 / p98 77.5 ms; rAF p95 66.2 ms; release to indexed 15.61 [15.20-16.58] s without, 15.93 [15.15-16.34] s with.
  - The comparison with the `main` code (without the interactive TIC) was not made: without gestures the module adds no work beyond the `sync` already present.
