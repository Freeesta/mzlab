# Stato dell'assistente dati su `main`

Controllato su `main` al commit di questa PR (CI `test.yml` verde). Prova = file presenti e test che girano nella CI (`cargo test` dei crate, `tests/`, `tests_e2e/`). «Fatto» vuol dire codice e test in `main`; «collegato al sito» vuol dire che un visitatore di https://freeesta.github.io/mzlab/ lo usa davvero.

## 1. A1–A8 (`08-prompt-dev.md`)

| PR | Stato | Prove (file e test) | Cosa manca |
|---|---|---|---|
| A1 adattatore Anthropic, spesa | fatto | `crates/mzlab-llm/src/{spend.rs, provider/, event.rs}` (`Options`: cache 5m/1h, `effort`, thinking adattivo, `strict`, `format_schema`); `data/fornitori-1.json` con `as_of`; `tests/anthropic.rs`, `tests/spend.rs` (`Meter::admit/charge/release`, soglia 80 %) | il crate `mzlab-tutor` non esiste in `main`: il tetto vive solo in `spend` |
| A2 `ispezione/1` | fatto, nel sito e nell'app | `inspect.rs`, `inspect_qc.rs`, `inspect_facts.rs`, `mzlab-core/src/scanstats.rs` (rotta `/api/scan_stats`); export `inspect_build`; `flow/lazy/flow-llm-facts.js`; `mzlab-cli llm inspect`; `tests/ispezione.rs`, `tests_facts.rs`, `e2e_llm_fatti.py` | `features.estimate` non è ancora tra le fonti della pagina |
| A3 `proposta-param/1` | fatto, nel sito e nell'app | `params_rules.rs` + `_det.rs` + `_tab.rs`; export `params_propose`; carta «Files» con «Apply/Discard»; `mzlab-cli llm params`; `llm_params_tests.rs`, `llm_facts_tests.rs`, `e2e_llm_fatti.py` | le risposte dell'utente (`answers`) non sono ancora chieste dalla pagina |
| A4 `tabcheck/1` | fatto, nel sito e nell'app | `crates/mzlab-feat/src/tabcheck/`; `docs/schemi/tabcheck-1.json`; export in `mzlab-feat-table-wasm`, rotta in `mzlab-desktop/src/feat.rs` | non passato ai lettori dal pannello |
| A5 `grafico/1` | fatto, nel sito | `chart.rs`, `chart_data.rs`, `chart_views.rs`; export `chart_views/validate/presets`; `flow/lazy/flow-llm-chart*.js`; `F.chartSources` (TIC e BPC dei file aperti); `tests_e2e/e2e_llm_grafici.py`, `tests/test_llm_grafico.py` | le tabelle (righe di `features.table`) non sono ancora fonti |
| A6 subagenti, router, `lettura/1` | fatto in Rust e nel worker; fonti vere dalla pagina | `assist.rs`, `reading.rs`, `router.rs`, `subcheck.rs`, `subcall.rs`, `tools_assist.rs`; `data/assist/` (8 prompt + `assist-ruoli-1.json`); classe `Assist` nel `.wasm`; `F.llmFacts.readerInput`; `llm_assist_session_tests.rs`, `e2e_llm_lettori.py`, `e2e_llm_fatti.py` | il pannello non chiama `enableAssist`; `interprete` riceve `tabcheck/1` solo da `F.tabcheckSource`, che la vista Tabella non definisce |
| A7 `mzlab-cli assist` | non fatto | nessun `assist.rs` in `crates/mzlab-cli/src`, nessun sottocomando | tutta la PR |
| T test con la chiave | nessuna prova nel repository pubblico | – | dipende da A7 |
| A8 carte nel pannello | non fatto | nessun `flow-llm-cards.js`, nessun `e2e_llm_assist.py` | tutta la PR |

`.wasm` `llm`: tetto `limit_llm` 397 312 B, misurato 358 897 B (`docs/agenti/wasm-tetti.md`): restano 38 415 B prima del tetto.

## 2. Cosa gira oggi nel sito

- **Chi lo vede**: solo il profilo «ricerca» (modalità amministratore con 5 clic sul logo, o app desktop, `settings.js` `uipProfile()`); un visitatore normale è «corso» e non scarica nessun file di `llm/`.
- **Trasporto** (`mzlab/web/llm/trasporto.html`): CSP `connect-src 'self' http://localhost:* http://127.0.0.1:*`. Dal sito si raggiunge solo un modello locale (Ollama) o una registrazione `replay` servita da 'self'. **La via V1a (chiave dell'utente verso `api.anthropic.com`) non è mai stata costruita** e con la scelta di Federico (nessuna chiave personale) non si costruisce più.
- **Pannello del Flow** (`flow/lazy/flow-llm.js`, `-tools.js`, `-readers.js`, `-chart*.js`): agente con politica L0–L2, carte «Apply/Discard» del piano, «Allow/Deny», «Undo», carta dei lettori, grafici. `data_summary`, `data_inspect` e `params_propose` rispondono con i fatti della pagina, `code_export` con `err.llm.tool.unavailable`.
- **Consenso** (`llm-consenso.js`): classi `plan`, `summary`, `names` per fornitore, in `localStorage` (permanente finché revocato). Manca la classe `inspection`.
- **App desktop**: fornitori cloud raggiungibili con la chiave dell'utente in `llm-chiavi.json` (rotte `llm/*`).
- **Parti deterministiche senza modello nel sito**: `tabcheck/1` (vista Tabella), la carta «Files» del Flow (ispezione e proposte dei parametri con «Apply/Discard», profilo «ricerca», senza consenso) e i grafici predefiniti su TIC e BPC dei file aperti: il «rules only» di §5 di `05-sicurezza.md` esiste nel sito.

## 3. Conseguenze per il piano pubblico

1. Il proxy (`09-worker.md`) da solo non basta: servono prima le fonti nella pagina (ispezione, proposte, `readerInput`, `chartSources`), altrimenti il modello vede solo il piano.
2. Il pannello deve esistere anche per i visitatori anonimi in una modalità «demo» distinta da «corso» (decisione D2 di `14-decision-card.md`).
3. Il test T con la chiave non ha bisogno di A7 se la prova si fa dal pannello dietro il proxy in fase 0 (solo Federico): A7 resta utile per le valutazioni Batch ma non blocca la demo.
