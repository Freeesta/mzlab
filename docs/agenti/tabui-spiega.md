# Dove la tabella di Tabui perde un prodotto atteso (`explain-missing`)

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
- Docs: `flow-copertura-tabui.md`, `flow-pacchetto-tabui.md`, this file, the Tabui parts of `gruppi.md`, `mzfinder*.md`, `livelli.md`, `reattivita.md`, `misure.md`, `sospetti.md`, and the rows of
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

- Comando: `mzlab-cli explain-missing TABLE.mzfeat EXPECTED.csv [--ppm 5] [--rt-tol 0.3] [--out FILE]` (`crates/mzlab-cli/src/flow_explain.rs`, `--help` per le colonne). `TABLE.mzfeat` è la tabella di una corsa (uscita di `tp.link` o successivi); `EXPECTED.csv` ha una riga `id,mz,rt` per prodotto atteso (l'intestazione è saltata).
- Uscita: una riga CSV per prodotto, solo numeri e sigle, mai nomi: righe entro la finestra di massa (`rows_mz`) e anche di RT (`rows_in`), errore in ppm e ΔRT della riga migliore (quella di area massima), celle rilevate e riempite, altezza e area massime, `tp_status` di `tp/1` e `stage`.
- `stage`: `view` = la riga passa tutti i filtri; `F1`..`F4` = primo filtro di `filtri/1` che la nasconde (F5, errore di massa, non è misurabile dalla sola tabella); `rt_off` = esiste una riga con quella massa ma fuori dalla finestra di RT; `absorbed` = l'atteso è membro di una riga-famiglia il cui ione di riferimento sta fuori finestra (colonne `absorbed_*`); `no_row_mz` = nessuna riga a quella massa (perso prima: rilevamento, allineamento o ipotesi). `nearest_ppm` e `nearest_drt` danno la riga più vicina in massa quando non c'è corrispondenza.
- Uso con la serie locale: dopo la corsa del piano, `mzlab-cli explain-missing RUNDIR/<tabella>.mzfeat attesi.csv --out conteggi.csv`; riportare solo i conteggi per `stage`, mai percorsi o nomi dei composti.
- Rango: `kinship` (0-4), `q_shape` e `q_snr` della riga migliore, `rank_now` = posto della riga nella vista (righe con `tp_status` 0 per parentela, poi area massima) su `view_size` righe. È la misura del rango che la locale riporta come mediana e 90° percentile per confrontare gli ordinamenti. `q_reason` = bit di `quality::Q_*` che fanno scattare F1 (1 forma, 2 S/N, 4 punti).
- Ordinamenti a confronto: `rank_k1`..`rank_k3` sono il posto della riga migliore nella stessa vista con altre chiavi (sempre prima la parentela): k1 picchi rilevati poi area, k2 fonti concordi (bit di `via`) poi picchi, k3 punteggio continuo (log10 area + 1,5 × quota di file con picco + 0,5 × fonti + forma). Servono a scegliere con la serie vera, non a imporre un ordine.
- Dimensione della vista: `kin_rank` e `kin_size` sono il posto della riga migliore tra le righe della vista della stessa parentela (nell'ordine della vista) e la dimensione del gruppo; `singles_above` conta le righe sopra di lei viste in un solo file; `view_single_pct` è la quota percentuale di righe della vista viste in un solo file. Servono a decidere un criterio di presenza che riduca la vista senza perdere i prodotti deboli.
- Prodotto debole rilevato in pochi file (`detected` basso, `max_height` sotto `min_height` negli altri): è un limite di segnale, non di ordinamento; l'ordine della vista resta parentela e poi area. Rimedio misurato: `align.mode=ion` con riempimento dei buchi (gapfill) recupera celle senza toccare l'ordine; `rt_warp=piecewise` con `rt_gap=0.3` riduce la vista in modo `ion` ma in modo `family` fa perdere righe (`no_row_mz`), quindi non è il default delle famiglie.
