# Stato dell'assistente dati su `main`

Controllato su `main` al commit di questa PR (CI `test.yml` verde). Prova = file presenti e test che girano nella CI (`cargo test` dei crate, `tests/`, `tests_e2e/`). «Fatto» vuol dire codice e test in `main`; «collegato al sito» vuol dire che un visitatore di https://freeesta.github.io/mzlab/ lo usa davvero.

## 1. A1–A8 (`08-prompt-dev.md`)

| PR | Stato | Prove (file e test) | Cosa manca |
|---|---|---|---|
| A1 adattatore Anthropic, spesa | fatto | `crates/mzlab-llm/src/{spend.rs, provider/, event.rs}` (`Options`: cache 5m/1h, `effort`, thinking adattivo, `strict`, `format_schema`); `data/fornitori-1.json` con `as_of`; `tests/anthropic.rs`, `tests/spend.rs` (`Meter::admit/charge/release`, soglia 80 %) | il crate `mzlab-tutor` non esiste in `main`: il tetto vive solo in `spend` |
| A2 `ispezione/1` | fatto in Rust, non nel sito | `inspect.rs`, `inspect_qc.rs`, `inspect_facts.rs`, `inspect_scan.rs`; `docs/schemi/ispezione-1.json`; `tests/ispezione.rs`, `tests/ispezione_serie.rs`, `tests/test_llm_ispezione.py` | `inspect_build` non è esportata dal `.wasm` `llm`: la pagina non costruisce l'ispezione |
| A3 `proposta-param/1` | fatto in Rust, non nel sito | `params_rules.rs` + `_det.rs` + `_tab.rs`; `docs/schemi/proposta-param-1.json`; `llm_params_tests.rs`, `llm_params_g_tests.rs` (G15–G22), `tests/test_llm_proposta_param.py` | non esportata dal `.wasm` |
| A4 `tabcheck/1` | fatto, nel sito e nell'app | `crates/mzlab-feat/src/tabcheck/`; `docs/schemi/tabcheck-1.json`; export in `mzlab-feat-table-wasm`, rotta in `mzlab-desktop/src/feat.rs` | non passato ai lettori dal pannello |
| A5 `grafico/1` | fatto, fonti vuote nel sito | `chart.rs`, `chart_data.rs`, `chart_views.rs`; export `chart_views/validate/presets`; `flow/lazy/flow-llm-chart*.js`; `tests_e2e/e2e_llm_grafici.py`, `tests/test_llm_grafico.py` | `F.chartSources` non è definita dalla pagina: il menu dice «nessun file con dati» |
| A6 subagenti, router, `lettura/1` | fatto in Rust e nel worker; lettori saltati nel pannello vero | `assist.rs`, `reading.rs`, `router.rs`, `subcheck.rs`, `subcall.rs`, `tools_assist.rs`; `data/assist/` (8 prompt + `assist-ruoli-1.json`); classe `Assist` nel `.wasm`; `llm_assist_session_tests.rs` (17 chiamate su `fs_pos`), `e2e_llm_lettori.py`, `e2e_llm_lettori_pannello.py` | `flow/lazy/flow-llm.js` non dà `readerInput`: nel pannello reale ogni lettore è `skipped` (solo l'e2e gli passa fonti finte) |
| A7 `mzlab-cli assist` | non fatto | nessun `assist.rs` in `crates/mzlab-cli/src`, nessun sottocomando | tutta la PR |
| T test con la chiave | nessuna prova nel repository pubblico | – | dipende da A7 |
| A8 carte nel pannello | non fatto | nessun `flow-llm-cards.js`, nessun `e2e_llm_assist.py` | tutta la PR |

`.wasm` `llm`: tetto `limit_llm` 344 064 B, misurato 309 824 B (`docs/agenti/wasm-tetti.md`): restano 34 240 B prima del tetto.

## 2. Cosa gira oggi nel sito

- **Chi lo vede**: solo il profilo «ricerca» (modalità amministratore con 5 clic sul logo, o app desktop, `settings.js` `uipProfile()`); un visitatore normale è «corso» e non scarica nessun file di `llm/`.
- **Trasporto** (`mzlab/web/llm/trasporto.html`): CSP `connect-src 'self' http://localhost:* http://127.0.0.1:*`. Dal sito si raggiunge solo un modello locale (Ollama) o una registrazione `replay` servita da 'self'. **La via V1a (chiave dell'utente verso `api.anthropic.com`) non è mai stata costruita** e con la scelta di Federico (nessuna chiave personale) non si costruisce più.
- **Pannello del Flow** (`flow/lazy/flow-llm.js`, `-tools.js`, `-readers.js`, `-chart*.js`): agente con politica L0–L2, carte «Apply/Discard» del piano, «Allow/Deny», «Undo», carta dei lettori, grafici. `data_summary` e `code_export` rispondono `err.llm.tool.unavailable`.
- **Consenso** (`llm-consenso.js`): classi `plan`, `summary`, `names` per fornitore, in `localStorage` (permanente finché revocato). Manca la classe `inspection`.
- **App desktop**: fornitori cloud raggiungibili con la chiave dell'utente in `llm-chiavi.json` (rotte `llm/*`).
- **Parti deterministiche senza modello nel sito**: solo `tabcheck/1` (vista Tabella) e i grafici predefiniti (senza fonti). Ispezione e proposte dei parametri sono Rust ma non ancora nel `.wasm`: il «rules only» di §5 di `05-sicurezza.md` non esiste ancora nel sito.

## 3. Conseguenze per il piano pubblico

1. Il proxy (`09-worker.md`) da solo non basta: servono prima le fonti nella pagina (ispezione, proposte, `readerInput`, `chartSources`), altrimenti il modello vede solo il piano.
2. Il pannello deve esistere anche per i visitatori anonimi in una modalità «demo» distinta da «corso» (decisione D2 di `14-decision-card.md`).
3. Il test T con la chiave non ha bisogno di A7 se la prova si fa dal pannello dietro il proxy in fase 0 (solo Federico): A7 resta utile per le valutazioni Batch ma non blocca la demo.
