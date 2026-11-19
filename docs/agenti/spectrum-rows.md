# Rows of the MS² spectrum card
`mzlab/web/spettro-righe.js` (`SPECROWS.update`, called by `afterDraw` in `pannelli.js`) puts one row per topic under the card header; CSS `order` in `app.css` fixes the sequence:
1. Fragments and intensities: the header controls (`.ctl`, wrapped onto its own line, `aria-label` «Fragments and intensities»).
2. Notes: `p.note`, saved with the panel (`explore.js`).
3. Retention times: the scan window plus the limits of the integrated peaks of the linked chromatogram that touch it.
4. CE: `ce_mid ± ce_spread` eV for a stepped CE, else `ce`; read from `ms2_exps` of the engine (`crates/mzlab-core/src/cespread.rs`), never typed; no CE, no row.
5. Sample: the file label (MS², no overlay).
Same data in site, app and CLI: `mzlab-cli file items FILE` prints the `Item.info()` JSON (`ms2_exps` has `prec`, `ce`, `ce_mid`, `ce_spread`, `n`; the Python reference is `Item._ms2_exps` in `mzlab/explore.py`, checked by `iteminfo_parity`). Tests: `e2e_spettro_righe`, `items_cmd` and `cespread` unit tests, `iteminfo_parity`.
