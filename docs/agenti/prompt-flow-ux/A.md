# FLOW-A — Il grafo di frammentazione si apre dopo la corsa

- **Ramo**: `claude/flow-msn-risultati` da `origin/main`.
- **Dipende da**: nessuno. **In parallelo con**: B, C, E. **Dopo di te**: D.

## Regole comuni (valgono per tutto il lavoro)
- Flow è solo per l'amministratore e SOLO in inglese: nessuna chiave in `lang/`, `flow/` sta fuori da `tools/controlla_i18n.py`.
- **Regola dei testi di Federico**: pochi testi. Niente paragrafi esplicativi dove l'interfaccia si capisce da sola. I tooltip servono SOLO a dire cosa fa un campo o un pulsante, o cosa inserirvi: mai opinioni, avvisi o cautele. Un testo del tuo perimetro che non dice cosa fa o cosa inserire si toglie (con la sua chiave i18n, se ne ha una).
- Logica in Rust, JS solo colla; file ≤ 500 righe; WCAG 2.2 AA (bersagli ≥ 24 px, 40 al tocco, tastiera, `aria-label`, colori da variabili CSS, Okabe-Ito, `prefers-reduced-motion`); nessuna dipendenza nuova; nessun dato reale nel repository.
- FLOW-INFUSIONE-RAW (session_01Ly8ApkcMYtS51kFeTjDzWe) possiede il caricamento `.raw`, i messaggi d'errore del nodo Raw files e il «null» della riga di stato: non toccarli.
- Contesto completo: `docs/agenti/flow-ux.md` (stato del codice §1, layout §3). Leggilo una volta; non rileggere AGENTS.md.
- Primo messaggio al COO (send_message, session_01NawSmneBkv8sosEHpoC8k4): «FLOW-A: ricevuto». A fine PR un solo messaggio (≤ 5 righe: PR, SHA di 40 caratteri a CI verde, cosa resta). Eventi della PR: skill `steward`.

## 1. Ruolo
DEV Sonnet del Flow web. Leggi `docs/agenti/flow-web.md` e `docs/agenti/flow-ux.md` §1.

## 2. Contesto
Federico ha eseguito il modello «Fragmentation tree of a standard» su un'infusione MSn: tutti i nodi sono «done», ma il grafo non si vede. La corsa web MSn (`flow/lazy/flow-web-msn.js`, `one()`) scrive in OPFS `<istanza>.json` (msngrafo/1), `.msp` e `.csv` e li restituisce in `res.files`.
`F.webRunUi` (`flow-web-ui.js`) ne fa solo link di scaricamento; la sessione live (`flow-live-jobs.js`, lavoro `msn`) li tiene in `live.files`. `F.webAfterRun` (`flow-run-end.js`) mette in `S.results` solo la tabella `series.mzfeat`. La vista del grafo esiste (`F.openMsnGraph`, `flow-views-msn.js` + `flow-msn-graph.js`) ma niente la apre: `tests_e2e/e2e_flow_msn.py:118` la chiama a mano.

## 3. Perimetro
Tocchi solo: `flow/lazy/flow-run-end.js`, `flow/lazy/flow-web-ui.js`, `flow/lazy/flow-live-jobs.js` (solo il lavoro `msn`), `flow/lazy/flow-views-msn.js`, `tests_e2e/e2e_flow_msn.py` (le righe del grafo), una riga in `docs/agenti/flow-web.md`.
Non tocchi: `flow-shell.js`, `flow-names.js`, `flow-fix.js`, `flow-canvas.js`, `flow-form.js`, il nodo Raw files, Rust.

## 4. Lavoro
1. In `flow-run-end.js`, `F.webMsnAfterRun(S, files)`: ogni testo MSn entra in `S.results[<istanza>]` come `{type, label, text}` (`.json` → la vista del grafo, `.msp` → `library`, `.csv` → `table`) e sostituisce l'elemento con la stessa etichetta; poi `S.redraw()`.
2. Chiamalo da `F.webRunUi` a fine corsa e quando un lavoro `msn` della sessione live finisce (anche se non c'è il riquadro di Run: con Auto-run i passi partono da soli). Togli i link di scaricamento duplicati nella riga del passo se i file sono già nei «Results».
3. A fine corsa il grafo si apre da solo sotto il canvas, come la tabella delle feature, con il fuoco sul titolo della vista.
4. Se la vista `corsa` di `flow-report.js` non basta per il tipo scelto, registra in `flow-views-msn.js` una vista propria per il grafo (stesso `F.openMsnGraph`, nessuna copia).
5. Regola dei testi: nessuna nota nuova; «Finished. The results are kept in this browser (OPFS).» si accorcia a «Done.» se il resto si vede già.

## 5. Accettazione
- Dopo Run e dopo una modifica con Auto-run, il pannello del nodo del grafo ha «Results» con il grafo apribile, e i due file d'esportazione sono scaricabili dai loro nodi.
- Il grafo si apre da solo a fine corsa; `window.MZLAB_LR_MISS` resta vuota; nessun errore in console.

## 6. Prove
- `python3 tests_e2e/e2e_flow_msn.py`: togli l'apertura a mano (riga 118) e verifica `.ig-wrap svg` dopo Run e dopo una modifica di un parametro con Auto-run.
- `tsc` del progetto di `tools/tipi/` (0 errori nuovi), `python3 tools/pre_pr.py`, `python3 tools/verifica.py --cambiati`.

## 7. Consegna
PR da `claude/flow-msn-risultati` (titolo e testo in inglese, sempre aperta, mai unita da te); messaggio al COO come nelle regole comuni.
