# FLOW-D — Nomi umani, carte Export e View graph, legame con Data

- **Ramo**: `claude/flow-nomi-viste` da `origin/main`.
- **Dipende da**: l'unione di FLOW-A (i risultati MSn in `S.results`). **In parallelo con**: B, C, E.

## Regole comuni (valgono per tutto il lavoro)
- Flow è solo per l'amministratore e SOLO in inglese: nessuna chiave in `lang/`, `flow/` sta fuori da `tools/controlla_i18n.py`.
- **Regola dei testi di Federico**: pochi testi. Niente paragrafi esplicativi dove l'interfaccia si capisce da sola. I tooltip servono SOLO a dire cosa fa un campo o un pulsante, o cosa inserirvi: mai opinioni, avvisi o cautele. Un testo del tuo perimetro che non dice cosa fa o cosa inserire si toglie (con la sua chiave i18n, se ne ha una).
- Logica in Rust, JS solo colla; file ≤ 500 righe; WCAG 2.2 AA (bersagli ≥ 24 px, 40 al tocco, tastiera, `aria-label`, colori da variabili CSS, Okabe-Ito, `prefers-reduced-motion`); nessuna dipendenza nuova; nessun dato reale nel repository.
- FLOW-INFUSIONE-RAW (session_01Ly8ApkcMYtS51kFeTjDzWe) possiede il caricamento `.raw`, i messaggi d'errore del nodo Raw files e il «null» della riga di stato: non toccarli.
- Contesto completo: `docs/agenti/flow-ux.md` (stato del codice §1, layout §3). Leggilo una volta; non rileggere AGENTS.md.
- Primo messaggio al COO (send_message, session_01NawSmneBkv8sosEHpoC8k4): «FLOW-D: ricevuto». A fine PR un solo messaggio (≤ 5 righe: PR, SHA di 40 caratteri a CI verde, cosa resta). Eventi della PR: skill `steward`.

## 1. Ruolo
DEV Sonnet delle carte del Flow. Leggi `docs/agenti/flow-grafica.md` e `docs/agenti/flow-ux.md` §1 e §3.

## 2. Contesto
`F.nameOf` (`flow/lazy/flow-names.js:84`) mostra l'id quando un passo manca in `F.NAMES`: mancano `spectra.msn_graph`, `msn.library_export`, `msn.table_export`, `chrom.uv_peaks`, `features.table`, `quant.calibrate`, `spectra.ms2`, `table.drift`, `table.kmd`, `table.pca`, `table.stats`, `tp.dig_lr`.
`F.canvasModel` (`flow-fix.js:118`) disegna archi solo dai riferimenti `inst.in`: i passi MSn leggono l'infusione aperta in Data senza `file:` nel piano, quindi Raw files resta scollegato. I gruppi di Simple (`F.BASE`) esistono solo per la base. Il Flow disegna già il grafo con i moduli di Data (`flow-msn-graph.js` carica `infusione-grafo-svg.js`; `msngrafo-vista.js` ha albero e radiale).

Decisioni prese (Federico): nomi in inglese, id invariati: `spectra.msn_graph` «Fragmentation graph», `msn.library_export` «Save as library», `msn.table_export` «Save as tables»; gruppo «Export» con i due salvataggi; carta «View graph» a destra del grafo. La carta View è una VISTA, non un passo del piano: CLI e `mzflow/2` non cambiano.

## 3. Perimetro
Tocchi: `flow/lazy/flow-names.js`, `flow/lazy/flow-fix.js` (`canvasModel`, `layout`), `flow/lazy/flow-canvas.js` (solo la carta di vista), `flow/lazy/flow-msn-graph.js`, i test e2e del Flow nelle righe che leggono i titoli delle carte, righe in `flow-grafica.md`.
Non tocchi: `flow-shell.js`, `flow-shell-css.js`, `flow-run-*.js`, `flow-form.js`, `flow-inspect.js`, Rust.

## 4. Lavoro
1. `F.NAMES` per i 12 passi: i tre MSn come sopra; gli altri dal `title` del descrittore, in inglese semplice, ≤ 4 parole, mai la parola «formula» da sola. Il terzo campo («why») è una frase che dice cosa fa, senza opinioni.
2. Gruppi Simple anche fuori dalla base (`F.GROUPS`): «Export» = `msn.library_export` + `msn.table_export` (Advanced li mostra separati).
3. Arco da Raw files ai passi che leggono l'infusione di Data anche senza `file:` nel piano, così Raw files sta a sinistra di Fragmentation graph.
4. Carta «View graph» accanto a ogni passo con una vista registrata: spenta finché il risultato non c'è, poi apre la vista grande (`F.openMsnGraph`) con il fuoco dentro.
5. Nella vista del grafo: interruttore Tree / Radial e pannello dello ione presi dai moduli di Data (`msngrafo-vista.js`, `infusione-grafo-nodo.js`), più «Open in Data» che apre la scheda Fragmentation graph di Data sullo stesso file con gli stessi parametri. Nessuna copia di codice: se un modulo di Data non si può caricare così com'è, esporta una funzione da lì invece di duplicarla.
6. Disposizione compatta con ≤ 5 carte (es. W 230, H 84).

## 5. Accettazione
- Il modello MSn si disegna Raw files → Fragmentation graph → {Export, View graph}.
- Nessun id visibile nelle carte, nei menu né nella riga di stato.
- Il `mzflow/2` esportato è identico byte per byte a prima per lo stesso piano.

## 6. Prove
- `python3 tests_e2e/e2e_flow.py`, `e2e_flow_msn.py`, `e2e_flow_web.py`; `cargo test -p mzlab-cli --lib flow` invariato; `tsc`; `python3 tools/pre_pr.py`; `python3 tools/verifica.py --cambiati`.

## 7. Consegna
PR da `claude/flow-nomi-viste`; messaggio al COO come nelle regole comuni.
