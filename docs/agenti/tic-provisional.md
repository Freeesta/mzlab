# Interactive provisional TIC (zoom and pan of the RT axis)

The provisional TIC (`tic-provvisorio.js`, `map-loading.md` step 6) is drawn while the index is being built and can be zoomed and panned on the RT axis.

- **Controls** on `#ticprov-cv` (RT axis only; the height follows the window): Ctrl/Cmd + wheel (a plain wheel scrolls the page, as on the panels), drag, `+` `-`, arrows, `0` and double click = whole trace. One draw per frame (`FRAME.next`); 4-vertex drawing per column (`decima.js` `polyline`). A `role=status` line states the window once the gesture stops. A click repeats the progress note (no spectrum or XIC before the index). `touch-action: pan-y pinch-zoom` on the canvas only; no animation, so `prefers-reduced-motion` is respected.
- **Handover to the panel**: `TICPROV.handover()` (once) is called by `addPanel` (`pannelli.js`) for the first chromatogram without `zoom`: the panel opens on that window and a notice says so (automatic choice logged); the other panels of the view follow it through the time link.
- **Test**: `tests_e2e/e2e_tic_provvisorio.py` (wheel, drag, double click, keyboard, window of the first panel); the site and the `.wasm` are needed.
- **Desktop**: the app has no provisional TIC (direct native read, panel at once): no twin part.
- **Measurement** (`python3 tools/bench_tic_provvisorio.py`, headless Chromium, `.wasm` engine, synthetic mzML 242 MB = 20 000 scans x 3 000 peaks, cold: new browser and OPFS on every run, `idle`/`gest` runs alternated; a 489 MB file does not fit in the container's browser space, which stops at ~330 MB): one gesture (Ctrl+wheel, arrows) per frame while the box is visible keeps the event handler under 1 ms (p50) and the frame at the display rate (p50 16.7 ms, p95 18.1 ms at 1x; with the CPU slowed 4x the frame p95 is 66 ms);
  the time from release to indexed rises by about 0.5 s (13.5 s to 14.1 s at 1x). Without gestures the module adds no work beyond the `sync` already present.
