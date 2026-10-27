# FLOW-B — Etichette corte, descrizioni nei tooltip (tutti i moduli)

- **Ramo**: `claude/flow-tooltip` da `origin/main`.
- **Dipende da**: nessuno. **In parallelo con**: A, C, E.

## Regole comuni (valgono per tutto il lavoro)
- Flow è solo per l'amministratore e SOLO in inglese: nessuna chiave in `lang/`, `flow/` sta fuori da `tools/controlla_i18n.py`.
- **Regola dei testi di Federico**: pochi testi. Niente paragrafi esplicativi dove l'interfaccia si capisce da sola. I tooltip servono SOLO a dire cosa fa un campo o un pulsante, o cosa inserirvi: mai opinioni, avvisi o cautele. Un testo del tuo perimetro che non dice cosa fa o cosa inserire si toglie (con la sua chiave i18n, se ne ha una).
- Logica in Rust, JS solo colla; file ≤ 500 righe; WCAG 2.2 AA (bersagli ≥ 24 px, 40 al tocco, tastiera, `aria-label`, colori da variabili CSS, Okabe-Ito, `prefers-reduced-motion`); nessuna dipendenza nuova; nessun dato reale nel repository.
- FLOW-INFUSIONE-RAW (session_01Ly8ApkcMYtS51kFeTjDzWe) possiede il caricamento `.raw`, i messaggi d'errore del nodo Raw files e il «null» della riga di stato: non toccarli.
- Contesto completo: `docs/agenti/flow-ux.md` (stato del codice §1, layout §3). Leggilo una volta; non rileggere AGENTS.md.
- Primo messaggio al COO (send_message, session_01NawSmneBkv8sosEHpoC8k4): «FLOW-B: ricevuto». A fine PR un solo messaggio (≤ 5 righe: PR, SHA di 40 caratteri a CI verde, cosa resta). Eventi della PR: skill `steward`.

## 1. Ruolo
DEV Sonnet del modulo dei parametri del Flow. Leggi `docs/agenti/flow-grafica.md` (moduli) e `docs/agenti/flow-ux.md` §1 e §3.

## 2. Contesto
Sotto ogni campo di ogni modulo c'è una frase: la `description` del parametro e il `doc` dell'algoritmo dei descrittori `step/1` in Rust, scritti come `span.fl-ds` in `flow/lazy/flow-form.js:46` e `:57`. Federico: «esistono i tooltip apposta». Le stime hanno già un «ⓘ» con `title` (`flow-form.js:42`). I descrittori arrivano alla pagina dai JSON generati (`docs/schemi/steps-*.json`, `mzfinder/steps-*.json`), rigenerati con i comandi elencati in `tools/fondi_main.py`.

## 3. Perimetro
Tocchi: `flow/lazy/flow-form.js`, `flow/lazy/flow-msn-molecule.js` (solo i suoi testi), il CSS `.fl-ds` in `flow/lazy/flow-ui.js`, `crates/mzlab-core/src/step/` (due campi facoltativi del parametro e la loro scrittura in `json.rs`), le definizioni dei passi in Rust SOLO per aggiungere i testi brevi dell'interfaccia, i JSON generati rigenerati con il loro strumento, un test pytest nuovo, i test e2e che cercano `.fl-ds`, una riga in `docs/agenti/flow-grafica.md` e in `docs/agenti/rust-step.md`.
Non tocchi: `flow-inspect.js`, `flow-shell.js`, `flow-names.js`, `flow-fix.js`, il nodo Raw files.

## 4. Lavoro
1. Aiuto `F.tip(field, text)` in `flow-form.js`: pulsante «ⓘ» 24×24 (40 al tocco) con `aria-label` «About <etichetta>» e `aria-expanded`, che apre e chiude una nota `role="note"` sotto il campo, legata al campo con `aria-describedby`; si chiude con Esc, un nuovo clic o il fuoco altrove (WCAG 1.4.13). Mai `title` come unico veicolo.
2. `description` e `doc` passano da `span.fl-ds` al tooltip: di norma sotto il campo resta solo l'etichetta (con l'unità tra parentesi).
3. L'«ⓘ» delle stime usa lo stesso aiuto, con il testo ridotto a cosa fa il campo lasciato vuoto.
4. Decisione (Federico): l'aiuto della CLI NON cambia. `title`, `description` e `doc` restano come sono; aggiungi al parametro (e all'algoritmo) due campi facoltativi scritti nel JSON come `x-label` (etichetta ≤ 8 parole) e `x-tip` (UNA frase ≤ 20 parole che dice cosa fa il campo o cosa inserire; via opinioni, avvisi, cautele, rimandi). La pagina usa `x-label`/`x-tip` quando ci sono, altrimenti `title`/`description`. Scrivili per i passi dei modelli (MSn per primi), rigenera i JSON con i comandi di `tools/fondi_main.py`; `flow_rule_tests.rs` resta verde.
5. Test pytest nuovo: per ogni parametro dei JSON dei passi, l'etichetta mostrata (`x-label` o `title`) ≤ 8 parole e il tooltip (`x-tip` o `description`) una frase ≤ 20 parole, con una lista di eccezioni che può solo scendere; un secondo controllo verifica che `title` e `description` siano invariati rispetto a `origin/main` (l'aiuto della CLI non cambia).

## 5. Accettazione
- Nessuna `.fl-ds` visibile senza un clic; ogni campo con descrizione ha il suo «ⓘ» raggiungibile da tastiera.
- Il modulo «Fragmentation graph» (spectra.msn_graph) a 1280 px sta in una schermata senza scorrere (campi principali).
- axe del Flow ed `e2e_axe_grafici` senza violazioni nuove.

## 6. Prove
- `python3 tests_e2e/e2e_flow.py`, `e2e_flow_liste.py`, `e2e_flow_import.py`, il pytest nuovo, `cargo test -p mzlab-cli --lib flow`, `tsc`, `python3 tools/pre_pr.py`, `python3 tools/verifica.py --cambiati`.

## 7. Consegna
PR da `claude/flow-tooltip`; messaggio al COO come nelle regole comuni.
