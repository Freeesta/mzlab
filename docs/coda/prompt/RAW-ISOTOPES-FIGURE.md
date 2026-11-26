Read docs/coda/BRIEF.md.

# RAW-ISOTOPES-FIGURE: `raw isotopes --figure PATH`

**Goal.** Add `--figure PATH` to `raw isotopes`: an SVG with the traces of the bins M, M+1 and M+2 over the window, the peak region shaded, and below them the observed ratios R1 and R2 with their sigma next to the expected ones (when a formula is given) and the verdict of each.

**Why.** A reader checks a formula proposal by eye in one picture; the numbers of `raw-isotopes/1` stay the evidence, the figure only shows them.

**Read first.** `docs/agenti/raw-access.md` (`raw isotopes`), `docs/studies/RAW_ACCESS_STUDY_2.md` §7 and the `--figure` of §C (SVG, D4), `crates/mzlab-core/src/rawiso.rs` and `rawiso_json.rs`, the figure code of the other commands (`grep -n 'figure' docs/agenti/INDICE.md`).

**Already on main (checked 26 Nov, re-check before coding).** `raw isotopes` gives the counts, the ratios with sigma, the expected pattern, the z scores and the region; it draws nothing. The traces of the three bins come from the sweep of `rawiso::raw_isotopes_at`.

**Done means.** 1. `--figure PATH` writes the SVG and the path with its sha256 is in the output (exit 6 when it cannot be written); the route `/api/rawiso` takes the same switch and answers the SVG text, so the site, the app and the CLI give the same bytes. 2. Colours and fonts as the other figures, text through `lang/` keys. 3. A synthetic test with known ratios and a parity test CLI = app = wasm (the wasm leg fails, not skips, in CI). 4. The `.wasm` module `rawiso` keeps its cap or the PR states the growth.

**Out of scope.** A figure for `raw xic` or `raw scan`; doubly charged ions.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
