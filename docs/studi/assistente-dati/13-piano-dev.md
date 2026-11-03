# Piano DEV per l'assistente pubblico

Ordine delle PR ancora necessarie, partendo dallo stato di `11-stato.md`. Ogni PR ha un prompt autosufficiente in inglese in `prompt/dev-P*.md` che il COO lancia così com'è. Regole comuni: una PR alla volta per zona, `tools/pre_pr.py` prima di spingere, nessuna chiamata a un fornitore vero, nessuna chiave, nessun deploy; parità sito/app (AGENTS.md §1).

| PR | Titolo | Prerequisiti | Zona | Modello | `.wasm` |
|---|---|---|---|---|---|
| P1 | Fatti senza modello nella pagina: ispezione, proposte, fonti dei lettori e dei grafici | – | `crates/mzlab-llm-wasm`, `mzlab/web/llm/llm-worker.js`, `llm-agente.js`, `flow/lazy/flow-llm*.js`, `lang/` | Opus | `llm` cresce (stima +25–40 KB): sotto 344 064 o nuovo tetto = misurato +10 % dichiarato |
| P2 | Proxy anonimo (logica, ledger, adattatori Cloudflare e memoria) e conteggio in Rust | P1 non serve; via del Coordinatore per la cartella nuova `proxy/` | `proxy/`, `crates/mzlab-llm/src/spend.rs` (`Ledger`), `crates/mzlab-llm-wasm` (export `ledger_*`), `tests/test_proxy.py` | Opus | `llm` +≈ 5 KB |
| P3 | Modalità demo: catalogo ridotto, tetti del client, chip del copione, fornitore `mzlab-demo` | P2 unita | `crates/mzlab-llm/data/{fornitori-1.json, assist/assist-ruoli-1.json}`, `tools.rs` (catalogo demo), `docs/schemi/strumenti-demo-1.json`, `crates/mzlab-desktop/src/llm.rs`, `settings.js` (profilo «demo»), `lang/` | Sonnet | invariato |
| P4 | CSP del trasporto, consenso per visita, frase del proprietario, «Assistant not available» | P3 unita, indirizzo del proxy da Federico | `mzlab/web/llm/{trasporto.html, trasporto.js, llm-consenso.js, llm-impostazioni.js, llm-clienti.js}`, `tests/test_llm_csp.py`, `lang/`, e2e | Opus | invariato |
| P5 | Carte del copione (domande, letture, parametri), registrazione e «Play a recorded demo», e2e della demo | P1, P4 unite | `flow/lazy/flow-llm.js`, nuovo `flow/lazy/flow-llm-cards.js`, `llm-agente.js`, `static/llm/demo/`, `tests_e2e/e2e_llm_demo.py`, `tour.js` se cambia un bersaglio | Sonnet | invariato |

Dopo P5: Federico registra le demo in fase 0 (`12-demo.md` §3), la registrazione rivista entra in `static/llm/demo/` con una PR di soli dati, poi D3–D5 e fase 1.

A7 (`mzlab-cli assist`) e A8 restano nel piano originale ma non bloccano la demo: P5 copre le carte di A8 che servono al copione; A7 serve per le valutazioni Batch.

## Accettazione (comando → atteso)

| PR | Comando | Atteso |
|---|---|---|
| P1 | `cargo test -p mzlab-llm-wasm` | export `inspect_build`, `params_propose` con gli stessi byte dei golden di `mzlab-llm` |
| P1 | `python3 tools/verifica.py --solo e2e_llm_fatti` | carta «Files» e proposte visibili senza alcuna richiesta di rete; lettori non più `skipped` |
| P1 | `python3 tools/controlla.py` | `limit_llm` rispettato |
| P2 | `cargo test -p mzlab-llm --test spend` | `Ledger::admit` non supera mai nessun tetto (sessione, IP, giorno, mese), riserva saldata, flusso tagliato = riserva |
| P2 | `node --test proxy/test/` (o `pytest tests/test_proxy.py`) | fase 0 senza frase → 403; CORS altra origine → 403; prompt fuori lista → 400; `web_search` → 400; 2 richieste parallele non superano il tetto; nessun `console.log` di contenuto; nessuna chiave in uscita |
| P3 | `cargo test -p mzlab-cli --lib llm_tools` | `strumenti-demo-1.json` = esportazione, byte stabili, sottoinsieme del catalogo intero |
| P3 | `cargo test -p mzlab-desktop --lib llm` | il corpo verso `mzlab-demo` dall'app = il corpo del sito (stesso SHA-256) |
| P4 | `pytest tests/test_llm_csp.py` | `connect-src` = 'self', localhost e il solo host del proxy |
| P4 | `python3 tools/verifica.py --solo e2e_llm_consenso` | consenso per visita (`sessionStorage`), «Assistant not available» con proxy finto spento, axe 0 gravi |
| P5 | `python3 tools/verifica.py --solo e2e_llm_demo,e2e_tour` | copione HR e LR in replay stretto, «Apply/Undo», tetto, nessun host esterno |
