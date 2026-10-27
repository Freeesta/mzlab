# FLOW-F — Pannello Terminal (comando modificabile, «Run here»)

- **Ramo**: `claude/flow-terminal` da `origin/main`.
- **Dipende da**: l'unione di FLOW-E (pulsante «>_ Terminal»). **In parallelo con**: C, D.

## Regole comuni (valgono per tutto il lavoro)
- Flow è solo per l'amministratore e SOLO in inglese: nessuna chiave in `lang/`, `flow/` sta fuori da `tools/controlla_i18n.py`.
- **Regola dei testi di Federico**: pochi testi. Niente paragrafi esplicativi dove l'interfaccia si capisce da sola. I tooltip servono SOLO a dire cosa fa un campo o un pulsante, o cosa inserirvi: mai opinioni, avvisi o cautele. Un testo del tuo perimetro che non dice cosa fa o cosa inserire si toglie (con la sua chiave i18n, se ne ha una).
- Logica in Rust, JS solo colla; file ≤ 500 righe; WCAG 2.2 AA (bersagli ≥ 24 px, 40 al tocco, tastiera, `aria-label`, colori da variabili CSS, Okabe-Ito, `prefers-reduced-motion`); nessuna dipendenza nuova; nessun dato reale nel repository.
- FLOW-INFUSIONE-RAW (session_01Ly8ApkcMYtS51kFeTjDzWe) possiede il caricamento `.raw`, i messaggi d'errore del nodo Raw files e il «null» della riga di stato: non toccarli.
- Contesto completo: `docs/agenti/flow-ux.md` (stato del codice §1, layout §3). Leggilo una volta; non rileggere AGENTS.md.
- Primo messaggio al COO (send_message, session_01NawSmneBkv8sosEHpoC8k4): «FLOW-F: ricevuto». A fine PR un solo messaggio (≤ 5 righe: PR, SHA di 40 caratteri a CI verde, cosa resta). Eventi della PR: skill `steward`.

## 1. Ruolo
DEV Sonnet, Rust + colla JS. Leggi `docs/agenti/rust-flow.md`, `docs/agenti/cli.md` e `docs/agenti/flow-ux.md` §1 e §2.

## 2. Contesto
«Run on CLI» copia lo script di `api/flow/command` (Rust `crates/mzlab-core/src/flow/command.rs`: here-document con tutto il piano + `mzlab-cli flow run plan.json --in "$MZLAB_IN" --out …`); `flow/lazy/flow-run-ui.js` mostra una `textarea` con la cartella. La CLI accetta già `--set ISTANZA.NOME=VALORE` (`crates/mzlab-cli/src/flow_run.rs:68`; `apply_set` in `crates/mzlab-cli/src/flow_apply.rs:58`). L'app desktop esegue il piano nel processo (`crates/mzlab-desktop/src/flow_run.rs`, rotta `flow/run`); il browser con `F.runners.web`.

Decisione presa (Federico): pannello fatto in casa, niente xterm.js né pty; copia automatica con «Copied» visibile; comando modificabile nel pannello; «Run here».

## 3. Perimetro
Tocchi: `crates/mzlab-core/src/flow/` (`command.rs` + un modulo nuovo per la riga), `crates/mzlab-cli/src/flow_apply.rs` (diventa un rimando al core), il ponte wasm e la rotta dell'app desktop della sola `flow/line`, `flow/lazy/flow-run-ui.js`, file nuovo `flow/lazy/flow-terminal.js` (in `LATE` di `flow/js/flow-tool.js`), i test; righe in `rust-flow.md`, `cli.md`, `flow-grafica.md`.
Non tocchi: `flow-shell.js` e il resto della barra (FLOW-E), `flow-fix.js`, `flow-names.js`.

## 4. Lavoro
1. Rust: la riga corta `mzlab-cli flow run plan.json --in <cartella> --out <cartella> --set ISTANZA.NOME=VALORE …` (un `--set` per ogni parametro diverso dal default, in ordine stabile) accanto allo script di oggi.
2. Rust: `apply_set` passa in `mzlab-core::flow`; rotta `flow/line` (corpo: riga + piano) → il piano con i `--set` applicati, oppure l'errore `err.cli.*` della CLI. Stesso codice per CLI, wasm e app.
3. Pannello (`flow-terminal.js`): uscita in `<pre aria-live="polite">`; la riga in un `input` monospaziato (non `contenteditable`); all'apertura copia la riga e mostra «Copied ✓» nel pannello; pulsanti «Copy», «Run here», «Reset», «Show script» (lo script completo, copiabile).
4. «Run here»: manda la riga a `flow/line`, poi esegue il piano ottenuto con l'esecutore della pagina (browser `F.runners.web`, app `flow/run`); una riga di uscita per evento di passo, con i nomi umani dei passi. Nel browser `--in`/`--out` non si usano: una riga lo dice («Files: the ones open in Data.»).
5. Regola dei testi: nel pannello nessun paragrafo d'istruzioni; tooltip solo per cosa fa ogni pulsante.

## 5. Accettazione
- Cambiare `--set graph.ppm_formula=5` e premere «Run here» dà lo stesso piano di `mzlab-cli flow run plan.json --set graph.ppm_formula=5`.
- Un `--set` sbagliato mostra lo stesso errore della CLI; tutto si usa da tastiera; nessuna dipendenza nuova.

## 6. Prove
- Test Rust di parità riga ↔ CLI; `cargo test -p mzlab-core -p mzlab-cli`; e2e nuovo `tests_e2e/e2e_flow_terminal.py`; `tsc`; `python3 tools/pre_pr.py`; `python3 tools/verifica.py --cambiati`.

## 7. Consegna
PR da `claude/flow-terminal`; messaggio al COO come nelle regole comuni.
