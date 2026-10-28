# FLOW-E — Barra compatta e inspector da metà pagina

- **Ramo**: `claude/flow-barra` da `origin/main`.
- **Dipende da**: nessuno. **In parallelo con**: A, B, C, D. **Dopo di te**: F.

## Regole comuni (valgono per tutto il lavoro)
- Flow è solo per l'amministratore e SOLO in inglese: nessuna chiave in `lang/`, `flow/` sta fuori da `tools/controlla_i18n.py`.
- **Regola dei testi di Federico**: pochi testi. Niente paragrafi esplicativi dove l'interfaccia si capisce da sola. I tooltip servono SOLO a dire cosa fa un campo o un pulsante, o cosa inserirvi: mai opinioni, avvisi o cautele. Un testo del tuo perimetro che non dice cosa fa o cosa inserire si toglie (con la sua chiave i18n, se ne ha una).
- Logica in Rust, JS solo colla; file ≤ 500 righe; WCAG 2.2 AA (bersagli ≥ 24 px, 40 al tocco, tastiera, `aria-label`, colori da variabili CSS, Okabe-Ito, `prefers-reduced-motion`); nessuna dipendenza nuova; nessun dato reale nel repository.
- FLOW-INFUSIONE-RAW (session_01Ly8ApkcMYtS51kFeTjDzWe) possiede il caricamento `.raw`, i messaggi d'errore del nodo Raw files e il «null» della riga di stato: non toccarli.
- Contesto completo: `docs/agenti/flow-ux.md` (stato del codice §1, layout §3). Leggilo una volta; non rileggere AGENTS.md.
- Primo messaggio al COO (send_message, session_01NawSmneBkv8sosEHpoC8k4): «FLOW-E: ricevuto». A fine PR un solo messaggio (≤ 5 righe: PR, SHA di 40 caratteri a CI verde, cosa resta). Eventi della PR: skill `steward`.

## 1. Ruolo
DEV Sonnet del guscio del Flow. Leggi `docs/agenti/flow-grafica.md`, `docs/agenti/flow-live.md` e `docs/agenti/flow-ux.md` §1 e §3.

## 2. Contesto
La barra (`flow/lazy/flow-shell.js:124-129`) ha 17 controlli su due righe: Canvas / List view / Plan (JSON) occupano una riga a sé; «Open» apre un piano `mzflow/2` (il file `#fl-file` della List view), non file di dati; «Live» (`flow-live-bar.js`) fa partire da soli i passi quando il piano cambia; «Run on CLI» + ▾ copiano o mostrano lo script. L'inspector ha `--insp` 340 px (240–900).

Decisioni prese (Federico): «Open» tolto dalla barra (il blocco Raw files c'è sempre; il piano si apre ancora dalla List view); «Live» resta come interruttore piccolo «Auto-run» con tooltip «Run again when an input or setting changes»; Canvas/List come interruttore piccolo nella stessa riga; inspector da metà pagina; un solo pulsante «Terminal» al posto di «Run on CLI» e ▾ (il pannello lo fa FLOW-F).

## 3. Perimetro
Tocchi: `flow/lazy/flow-shell.js`, `flow/lazy/flow-shell-css.js`, `flow/lazy/flow-live-bar.js`, lo scheletro della barra in `flow/js/flow-tool.js`, i test e2e nelle righe che usano `data-tab`, `#flx-live-on`, «Open», «Run on CLI», `#flx-cli-show`; righe in `flow-grafica.md` e `flow-live.md`.
Non tocchi: `flow-run-ui.js` (FLOW-F), `flow-fix.js`, `flow-canvas.js`, `flow-names.js`, `flow-form.js`.

## 4. Lavoro
1. Una riga: `Flow` · `Canvas|List` (segmento piccolo) · Templates · «+ Add step» · `Simple|Advanced` · Undo/Redo/Fit solo icone (con `aria-label` e tooltip) · spazio · stato «✓ Ready» / «⚠ N to fix» (il ⚠ apre la Checklist) · «Auto-run» (interruttore piccolo, `aria-pressed`, id `flx-live-on`) · «▶ Run» · «>_ Terminal» · «⋯».
2. Il menu «⋯» contiene «Plan (JSON)» (solo Advanced) e «Download plan». «Open» sparisce dalla barra; l'apertura del piano resta nella List view.
3. «Pause» compare solo quando Auto-run ha lavori attivi; «Run now» resta com'è (solo quando un passo chiede memoria).
4. «>_ Terminal» tiene l'id `flx-cli-show` e chiama `actions().toggle` come la freccia di oggi: il pannello nuovo arriva con FLOW-F senza toccare la barra.
5. Inspector: `--insp` iniziale 50 % della larghezza del corpo, limiti 240 px – 70 %; il separatore resta (trascinamento e frecce); la larghezza scelta vale per la sessione della pagina.
6. Regola dei testi: tooltip solo per cosa fa il pulsante; togli i testi della barra che non lo dicono.

## 5. Accettazione
- A 1280 px la barra è su una riga; a 360 px va a capo senza scorrimento orizzontale.
- Ogni bersaglio ≥ 24 px (40 al tocco); tutto raggiungibile da tastiera; il tour guidato (`tour.js`, `tests_e2e/e2e_tour.py`) resta valido se tocca il Flow.
- Auto-run acceso/spento si comporta come il «Live» di oggi.

## 6. Prove
- `python3 tests_e2e/e2e_flow.py`, `e2e_flow_live.py`, `e2e_flow_annulla.py`, `e2e_flow_web.py`, `e2e_axe_grafici`; `tsc`; `python3 tools/pre_pr.py`; `python3 tools/verifica.py --cambiati`.

## 7. Consegna
PR da `claude/flow-barra`; messaggio al COO come nelle regole comuni.
