Read docs/coda/BRIEF.md.

# PBAR-DDA-TICBG-BAR: The DDA buttons and `ticbg` in the shared bar

**Goal.** `ddaw`, `ms2tri` and `ticbg` are drawn by the shared bar (`#pbar`) and work from it, as `xwc` and `xwcl` do.

**Why.** PBAR-MODULE-BUTTONS-2 made them registry actions (header groups `dd` and `tb`), but `pbar.js` has no group for them, so with the bar on they are in no place at all (the header tool groups are hidden by CSS). PBAR-REMOVE-OLD cannot turn the bar on by default before this.

**Read first.** `docs/agenti/actions.md` (Header buttons, Panel bar), `pbar.js` (`PB`, `GRP`), `dda-elenco.js` (`open`, `close`), `tic-fondo-ui.js` (`open`, `close`), `xwc.js` (`arrows`, `pop`: the pattern for a popover opened from the bar).

**Done means.** 1. `PB` maps `dd` and `tb` to a bar group (Extract is the natural one for `ddaw`, `ms2tri`, `ticbg`: decide and say why in the PR). 2. The list of `ddaw` and the options of `ticbg` anchor under the panel header and close on Esc with the focus back on the bar button, with `aria-expanded` kept on both twins. 3. An e2e in the `MZLAB_PBAR=1` mode opens each from the bar. 4. `e2e_dda_ms2_vie`, `e2e_tic_fondo` pass with `MZLAB_PBAR=1` as well. 5. The start-up scripts cap (`tests/test_peso_avvio.py`) is not raised (it has about 70 bytes of room).

**Out of scope.** Engine, CLI and `.wasm` (interface only; parity unchanged). The default of the setting (PBAR-REMOVE-OLD).

**Follow-ups.** Closing step of the brief: delete your row and this file.
