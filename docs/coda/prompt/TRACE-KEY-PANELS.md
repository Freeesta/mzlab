Read docs/coda/BRIEF.md.

# TRACE-KEY-PANELS: Colour key in the lists that the SIM/MRM key does not cover

**Goal.** Every list of traces that has a checkbox or a select shows, per row, a swatch with exactly the colour and dash of its trace, from the same computed source as the drawing, with the hollow and dimmed off state and the hover or focus highlight.

**Why.** The SIM/MRM ion picker and the «Extract ion ›» submenus carry the key (`SIMUI.style`, `p._look`, `SIMUI.swatch`, `SIMUI.lit`). Three lists do not: the MS2 transition `<select>`, the legend of the XIC list (it already has swatches, but they read `p.traces` by index), and the DAD/MRM overlay.

**Read first.** `docs/agenti/transitions.md` (colour-key bullets), `mzlab/web/sim-ui.js`, `mzlab/web/canali.js`, `mzlab/web/menu-sotto.js`, `tests_e2e/e2e_trace_key.py`. Start with `grep -n 'transitions' docs/agenti/INDICE.md`, then `grep -n 'SIMUI.swatch' mzlab/web/*.js`.

**Also.** After a file is moved in the list (`reorderFiles`), `SIMUI.ions(p)` is empty because its cache key `k|file` uses the position `k`; the «Extract ion ›» entry disappears until a reload. Fix the cache key or refresh `SIMUI.mark` after the move, and test it (the move step of `e2e_trace_key.py` runs last for this reason).

**Done means.** 1. The three lists show the swatch from one source, never from a second palette call. 2. E2E on synthetic data (`tools/wiff_sim.py`): swatch colour equals trace colour, off state and hover. 3. A move of a file keeps the ion list and its entry in the menu. 4. `tools/safari_api.py`, `tools/controlla.py` and the tour stay green.

**Out of scope.** The manual-integration square (`manint-sw`) and the area labels, which still use the file colour.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
