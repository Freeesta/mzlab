# Prompt DEV dell'assistente dati

Intestazione comune (vale per ogni prompt).

Sei una chat DEV di mzLab; AGENTS.md è nel contesto; leggi `docs/agenti/llm.md` e i file di questo studio citati nel prompt; ramo `claude/assist-<pr>` da `origin/main`; una PR alla volta; `tools/fondi_main.py` prima, `tools/pre_pr.py` completo prima di aprire; nessun cambio a `Cargo.toml`, workflow, `pages.yml`, CSP, tetti `.wasm` senza via; nessuna chiamata a un fornitore vero, nessuna chiave; codice e commenti in inglese, testi visibili solo in `lang/`; a fine PR un messaggio al Coordinatore (≤ 5 righe). Subagenti Haiku solo per compiti meccanici (`cercatore`, `verificatore`).

## A1 — Adattatore Anthropic completo, spesa e prezzi verificati
Leggi: `02-costo.md`, `07-mappa-codice.md` §3. Tocca: `crates/mzlab-llm/src/{provider/anthropic.rs, event.rs, cost.rs, lib.rs}`, nuovo `spend.rs`, `crates/mzlab-llm/data/fornitori-1.json`, `crates/mzlab-tutor/src/quota.rs` (importa da `spend`), test e file attesi in `crates/mzlab-llm/tests/data/`.
1. `Usage` con `cache_write_5m`, `cache_write_1h` letti da `usage.cache_creation` (e `cache_creation_input_tokens` come somma se manca il dettaglio).
2. Richiesta: `cache_control` esplicito con `ttl` sul blocco di sistema e automatico in cima, `output_config.effort`, `thinking: adaptive`, `strict: true` sugli strumenti, `output_config.format` (schema) per una chiamata senza strumenti, messaggi `role: "system"` a metà conversazione; mai `temperature`, `top_p`, `top_k`.
3. `spend`: prezzi in micro-dollari per milione, fasce di Haiku sopra 100 000 token, `admit(stima, max_tokens)` che non lascia mai superare il tetto, `charge(usage)`, soglia morbida 80 %; errori `err.llm.limit.spend`, `err.llm.limit.workspace` (400 «specified workspace API usage limits»), 429 senza `retry-after` mai ritentato.
4. Prezzi verificati in `fornitori-1.json` con `as_of` 2026-10-22; il test che vietava numeri ora vieta prezzi senza `as_of`.
Accettazione: `cargo test -p mzlab-llm`, `cargo test -p mzlab-tutor`, file attesi della richiesta Anthropic con cache e effort, SSE sintetico con `cache_creation` → eventi; nessuna chiave nelle uscite.

## A2 — `ispezione/1` e file sintetici con QC, bianchi, DIA
Leggi: `01-architettura.md` §3, `03-parametri.md` §3, `docs/agenti/metodo.md`, `docs/agenti/campioni.md`. Tocca: nuovo `crates/mzlab-llm/src/inspect.rs`, `docs/schemi/ispezione-1.json`, `tools/dati_sintetici.py` (serie con 3 QC e 2 bianchi, deriva RT lineare, contaminante solo nei bianchi, un file DIA), test.
1. `inspect::build(&sources, budget) -> Result<String>`: fonti = risposte che il motore dà già (file aperti, `metodo/1`, `features.estimate`, `campioni/2`, TIC/BPC); niente lettura di file in questo modulo.
2. Classe di risoluzione, tipi di acquisizione, infusione, tempo di ciclo, finestra di isolamento con le regole di `03-parametri.md` §3; nessuna inferenza dal nome del file.
3. Privacy: tolti `operatore`, utente e computer del driver, seriale, percorsi, `grezzo`, `campi` non mappati; nomi come alias se `hide_names`; stringhe ≤ 64 caratteri, controllo e direzionali Unicode tolti, cifre ≥ 6 mascherate.
4. Sezione `qc/1` dentro `ispezione/1` quando ci sono ≥ 2 QC: segnali comuni, deriva RT e d'area contro l'ordine, proposta di `rt_gap` (R-rt-1), come in `prompt/04`.
5. Stessi byte per lo stesso ingresso; tetto in token come `contesto/1`.
Accettazione: golden su tutti i file di `dati_sintetici.py` (tipo, polarità, risoluzione attesi = tabella G01–G09 di `06-valutazione.md`); test che nessun campo privato esce; `.wasm` `llm` misurato (sotto il tetto o via dichiarata).

## A3 — Regole dei parametri `proposta-param/1`
Leggi: `03-parametri.md`. Tocca: nuovo `crates/mzlab-llm/src/params_rules.rs`, `docs/schemi/proposta-param-1.json`, test.
1. Una funzione per regola (id `R-*` della tabella §5), ingresso `ispezione/1` + risposte dell'utente, uscita proposta con segnali, prova, confidenza, `range_ok` dai limiti `step/1` del registro reale.
2. `missing` con la domanda quando il segnale manca; nessun valore inventato.
Accettazione: un test per regola; casi G15–G22 con valori attesi; ogni nome di parametro esiste nel registro (`flow::steps()`), come `llm_tools_tests.rs`.

## A4 — Controlli della tabella `tabcheck/1`
Leggi: `04-feature-table.md`, `docs/agenti/mzfeat-tabella.md`, `docs/agenti/stat.md`. Tocca: nuovo `crates/mzlab-feat/src/tabcheck.rs`, rotta del motore e del desktop, `docs/schemi/tabcheck-1.json`, test.
1. I 15 controlli della tabella §2, ognuno con `value`, `threshold`, `flag`, `rows` (≤ 20), `proof`; masse degli addotti come costanti con il loro commento.
2. Stessi byte; tetto 6 000 token stimati.
Accettazione: tabelle dai file sintetici con i flag attesi (contaminante, addotto sodico, isotopi, frammenti, deriva RT della serie QC); misura del modulo `.wasm` che lo porta.

## A5 — Grafici `grafico/1`
Leggi: `01-architettura.md` §6, `docs/studi/llm/README.md` §7 e D6, `docs/agenti/dati.md` (`TDATI`), `docs/agenti/accessibilita.md`. Tocca: come D6 dello studio LLM, più `mark` `box|hist|heat`, `facet`, viste nominate (`tic:*`, `features:*`, `qc:cv`, `drift:rt`). Aggiorna la riga di D6 nello studio LLM. Il punto 4 di D6 (sandbox) resta fuori.
Accettazione: quella di D6; `e2e_axe_grafici` con il grafico dell'assistente.

## A6 — Subagenti, router, `lettura/1`, prompt come dati
Leggi: `01-architettura.md` §7–9, `prompt/01`–`08`. Tocca: nuovi `crates/mzlab-llm/src/{assist.rs, reading.rs}`, `crates/mzlab-llm/data/assist/*.md`, `mzlab-llm-wasm` (classe `Assist`), test.
1. La macchina `agent::State` emette `Output::Sub{role, request}` e riceve `Input::SubResult`; ruoli e modelli da una tabella dati (`assist-ruoli-1.json`), mai scritti nel codice.
2. `reading::validate` per `lettura/1` (azioni `none|ask|set_param|add_step|split_series|chart`) e `param-scelte/1` (`prompt/05`): `evidence` esistenti, `numbers` presenti nell'ingresso, ≤ 8 risultati, azioni valide (`set_param` nei limiti); errori restituiti al subagente una volta.
3. Prompt: copia esatta dei blocchi ```text dei file `prompt/0N_*.md` (test che confronta i byte); SHA-256 nel registro.
4. Strumenti nuovi `data_inspect`, `params_propose`, `table_check`, `table_rows`, `ask_user` nel catalogo (`strumenti-1.json` rigenerato).
Accettazione: registrazioni `llmreg/1` per una sessione intera G01 → piano → tabella → grafico; politica invariata; iniezione in un nome di file senza effetto.

## A7 — `mzlab-cli assist`
Leggi: `docs/agenti/tutor.md` (modo con chiave), `06-valutazione.md`, `prompt/00_esecuzione-test.md`. Tocca: `crates/mzlab-cli/src/assist.rs`, `assist_tests.rs`, `tests/data/assist/{oro,reg}/`, `docs/agenti/cli.md`, nuovo `docs/agenti/assistente-dati.md`.
1. `assist run DIR --replay R | --provider anthropic --key-file F [--yes] [--cap-usd 10] [--copione C]`: una sessione su una cartella di file (utente a copione per il test), stima prima, conferma, contatore, registro livello a.
2. `assist eval CASI (--replay DIR | --provider … --yes)`: una riga JSON per caso, riassunto con costo; uscita 1 se una soglia di `06-valutazione.md` non passa.
3. Trasporto `curl` con chiave e intestazioni su stdin, come `tutor eval`; nessun test lo raggiunge.
Accettazione: `cargo test -p mzlab-cli --lib assist` con le registrazioni; nessuna chiave nelle uscite; il tetto ferma una sessione finta prima di superarlo.

## A8 — Pannello nel desktop
Leggi: `docs/agenti/llm.md` §Pannello, `docs/agenti/flow-grafica.md`, `docs/agenti/lingue.md`. Tocca: `flow/lazy/flow-llm.js`, `flow-llm-tools.js`, nuovi `flow/lazy/flow-llm-cards.js` (domande, letture, proposte con «Apply»/«Discard», contatore di spesa), `lang/it.js`, `lang/en.js`, `tests_e2e/e2e_llm_assist.py`, `tour.js` se cambia un bersaglio.
Accettazione: e2e con `replay`: carte visibili, «Apply» = una voce di `S.undo`, contatore che si ferma al tetto, «rules only» senza rete, axe 0 gravi.
