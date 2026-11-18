# Where the Tabui table loses an expected product (`explain-missing`)

## Planned removal (owner decision, 17 Nov 2026)

Tabui (nav tab `data-v="tpmine"`, internal name "TP Mine" / mzFinder) is **hidden from the navigation but kept**. It is reachable only by its exact address `#/tpmine`
(typed, pasted, or a saved link). When the owner says so, it must be **DELETED ENTIRELY**. Until then nothing of it is deleted: code, tests, docs and i18n keys stay.

How it is hidden: one constant, `TABUI_VISIBLE = false` in `mzlab/web/first-paint.js`. It only hides the nav button (`display: none`); the tool stays registered and the
route `tpmine` stays in `VIEW_HASH` / `HASH_VIEW` (`mzlab/web/stato.js`). Set it to `true` to show the tab again. The e2e `tests_e2e/e2e_tabui_hidden.py` covers the hiding
(no nav button, no palette entry that opens it, the address opens it); `tests_e2e/lib.py` turns every test click on the Tabui nav button into the exact address (`open_tabui`).
The nav is page-only: there is no CLI route for it (the CLI and the Flow steps are not affected).

What the deletion will have to cover (check each with grep before removing it):
- `TP_Mine/` (about 1.3 MB: `js/`, `lazy/`, `files/`, help and style) and the lazy modules it loads; `mzlab/web/tpmine-loader.js` (the package loader: Flow also loads through it, see below).
- The nav button in `mzlab/web/index.html`, the loader hooks (`data-late`, `MZLAB_LATE_CLICK` in `first-paint.js`, `addTab`, `PLACEHOLDER`), the `tpmine` entries in `stato.js`, `shell.js` titles,
  `tour.js` (`TPMINE.on`).
- Flow views for Tabui (`flow/lazy/flow-views-tabui.js`, `flow-views-tptab.js`, parts of `flow-views-features.js`, `flow-family-proofs.js`, `flow-registry.js`, `flow-palette.js` package «Tabui») and
  the step family that exists only for Tabui:
  `crates/mzlab-feat/src/steps/tabui*.rs` (`tabui.rs`, `tabui_link.rs`, `tabui_draw.rs`), `docs/schemi/steps-tabui-1.json` (regenerate with its tool), the `tabui` rows of the step catalogs and
`flow_*` CLI tests that use them.
- Tests: every `tests_e2e/e2e_tabui_*.py` and `e2e_tpmine*.py`, `tests_e2e/e2e_tabui_hidden.py`, the Tabui clicks in shared tests (`e2e3.py`, `e2e_giro_intestazione.py`, `e2e_lab.py`,
  `e2e_axe_release.py`, `misura_primo_paint.py`), `open_tabui` in `tests_e2e/lib.py`, and the Rust tests `crates/mzlab-feat/tests/steps_tabui_parity.rs`, `crates/mzlab-finder-lr-wasm/tests/`.
- Docs: `flow-coverage-tabui.md`, `flow-package-tabui.md`, this file, the Tabui parts of `gruppi.md`, `mzfinder*.md`, `livelli.md`, `reattivita.md`, `misure.md`, `sospetti.md`, and the rows of
  `docs/agenti/INDICE.md` (regenerate `docs/inventari/TOOLTIP-*` and `docs/azioni.md` with their tools).
- i18n keys that only Tabui uses (`mapfeat.*`, `cmd.mapfeat.*`, `feat.mzf`, `feat.src.mzfinder`, `qaqc.feat.go`, `rep.feat`... check each with `python3 tools/controlla_i18n.py`; the Tabui UI itself is
  English-only literals, no keys).
- wasm modules and caps in `tools/moduli/*.toml`: candidate `finder-lr.toml` (`mzlab-finder-lr-wasm`, cap 892928 B, measured 785462 B), used by the LR search of Tabui; check that no Flow step or other
  tool calls it before removing it. `untargeted`, `hr`, `interp`, `table`, `import`, `grafo`, `chem` are shared with Flow or other tools: do NOT remove them without a grep of their users. Removing a
  module lowers a cap: that is a cap change, so state it in the PR.

Shared code that must NOT disappear: `QTOOLS` and `tpmine-loader.js` also register and load Flow (`flow/js/flow-tool.js`) and the Attributio package; `window.QTOOLS.ctx` (files, base) is used by
`flow/lazy/*.js`; `draw.js` (the Tabui «Draw» hook is only an API of the drawing view); `rapporto-flow.js`, `avanzamento.js`, `progresso.js`, `diagnosi.js` (shared progress and diagnosis code that
only mention Tabui in comments); `mzfinder`-named engine code in `crates/mzlab-feat` (`finder_base`, `legacy`, `untargeted`, `evid`) used by Flow steps and the CLI. Before deleting, run `python3
tools/indice.py` and grep each symbol you remove across `flow/`, `mzlab/web/`, `crates/` and `tests*/`; then `python3 tools/pre_pr.py --completo`.

- Command: `mzlab-cli explain-missing TABLE.mzfeat EXPECTED.csv [--ppm 5] [--rt-tol 0.3] [--out FILE]` (`crates/mzlab-cli/src/flow_explain.rs`, `--help` for the columns). `TABLE.mzfeat` is the table of one run (output of `tp.link` or later steps); `EXPECTED.csv` has one `id,mz,rt` row per expected product (the header is skipped).
- Output: one CSV row per product, only numbers and codes, never names: rows within the mass window (`rows_mz`) and also the RT window (`rows_in`), error in ppm and ΔRT of the best row (the one with the largest area), detected and filled cells, maximum height and area, `tp_status` of `tp/1` and `stage`.
- `stage`: `view` = the row passes all filters; `F1`..`F4` = first filter of `filtri/1` that hides it (F5, mass error, cannot be measured from the table alone); `rt_off` = a row with that mass exists but outside the RT window; `absorbed` = the expected product is a member of a family row whose reference ion lies outside the window (columns `absorbed_*`); `no_row_mz` = no row at that mass (lost earlier: detection, alignment or hypothesis). `nearest_ppm` and `nearest_drt` give the closest row in mass when there is no match.
- Use with the local series: after the plan run, `mzlab-cli explain-missing RUNDIR/<table>.mzfeat expected.csv --out counts.csv`; report only the counts per `stage`, never paths or compound names.
- Rank: `kinship` (0-4), `q_shape` and `q_snr` of the best row, `rank_now` = position of the row in the view (rows with `tp_status` 0 by kinship, then largest area) out of `view_size` rows. This is the rank measure the local run reports as median and 90th percentile to compare orderings. `q_reason` = bits of `quality::Q_*` that trigger F1 (1 shape, 2 S/N, 4 points).
- Orderings compared: `rank_k1`..`rank_k3` are the position of the best row in the same view with other keys (kinship always first): k1 detected peaks then area, k2 agreeing sources (bits of `via`) then peaks, k3 continuous score (log10 area + 1.5 × share of files with a peak + 0.5 × sources + shape). They serve to choose with the real series, not to impose an order.
- View size: `kin_rank` and `kin_size` are the position of the best row among the view rows of the same kinship (in view order) and the size of the group; `singles_above` counts the rows above it seen in a single file; `view_single_pct` is the percentage of view rows seen in a single file. They serve to decide a presence criterion that shrinks the view without losing weak products.
- Weak product detected in few files (`detected` low, `max_height` below `min_height` in the others): it is a signal limit, not an ordering one; the view order stays kinship and then area. Measured remedy: `align.mode=ion` with gap filling (gapfill) recovers cells without touching the order; `rt_warp=piecewise` with `rt_gap=0.3` shrinks the view in `ion` mode but in `family` mode loses rows (`no_row_mz`), so it is not the default for families.
