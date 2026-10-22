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

## A5 — Grafici `grafico/1` (D6 dello studio LLM, senza sandbox)
Prerequisiti: A1 unita. Il punto 4 di D6 (sandbox) resta fuori: nessuna via.
Leggi: `01-architettura.md` §6, `prompt/08_disegnatore-grafici.md`, `docs/studi/llm/README.md` §7 e D6, `docs/agenti/dati.md` (`TDATI`), `docs/agenti/accessibilita.md`, `docs/agenti/llm.md` §Pannello.
Tocca: nuovo `crates/mzlab-llm/src/chart.rs` (+ `chart_validate` in `crates/mzlab-llm-wasm`), `docs/schemi/grafico-1.json`, nuovo `flow/lazy/flow-llm-chart.js`, `flow/lazy/flow-llm-tools.js` (`chart_make` non più `unavailable`), `lang/it.js`, `lang/en.js`, `tests_e2e/e2e_llm_grafici.py`, `tests_e2e/e2e_axe_grafici.py` (un caso in più), `docs/agenti/llm.md`, riga D6 in `docs/studi/llm/README.md`. NON tocca: `grafici.js` della vista Dati, `rust-worker.js`, `index.html`, `code_export` (resta `unavailable`).
1. Schema `grafico/1`: `{schema, title ≤ 60, data:{ref}, mark: line|point|bar|stick|box|hist|heat, encoding:{x, y, color?, detail?}, facet? (≤ 12), log_y?, why}`; con `data.rows` (≤ 5 000) solo per i grafici predefiniti costruiti da Rust, mai dal modello. Uscita d'errore `{schema, error:"no_view", why}` ammessa.
2. Viste nominate: un registro Rust `chart::views(&sources)` che elenca le viste disponibili ora, ognuna con campi tipizzati e unità: `tic:<alias>` e `tic:*` (rt, intensity, file), `bpc:*`, `features:area` (row, file, area), `features:mzrt` (mz, rt, area), `qc:cv` (row, cv_pct), `drift:rt` (order, file, drift_min), `blank:ratio` (row, ratio). L'elenco è l'ingresso di `prompt/08`.
3. `chart::validate(spec, &views) -> Result<Spec>`: vista esistente, campi esistenti, tipo compatibile con il segno (`box` vuole x nominale e y quantitativo; `hist` un solo campo quantitativo; `heat` due nominali e uno quantitativo), `color` solo nominale con ≤ 8 valori, nessun campo di stile; riduzione oltre 5 000 righe per massimo d'intervallo come il TIC di `contesto/1` (stabile: stessi byte). Errori `err.llm.chart.<motivo>` restituiti al modello una volta.
4. Disegno in `flow-llm-chart.js`: Okabe-Ito dalle variabili CSS più tratteggi o forme per serie, assi con unità, tabella dei dati sotto con gli stessi valori (`TDATI`), zoom e spostamento con pulsanti e tastiera (bersagli ≥ 24×24 px, 40 al tocco), `aria-label` con titolo e `why`, «Download CSV»; `prefers-reduced-motion` rispettato.
5. Grafici predefiniti senza modello (stato «rules only»): TIC sovrapposti, distribuzione delle larghezze, deriva RT dei QC, CV dei QC, rapporto campione/bianco; li costruisce Rust come `grafico/1` con `data.rows`.
Prove: `cargo test -p mzlab-llm chart` (spec valide e invalide per ogni segno, riduzione stabile, `no_view`); `e2e_llm_grafici.py` (registrazione che chiede il TIC → grafico + tabella con gli stessi numeri; un `box` su `features:area`; un errore `no_view` mostrato come testo); `e2e_axe_grafici` verde con il grafico dell'assistente.
Accettazione: le prove sopra; `limit_llm` invariato o misura nuova dichiarata come via al Coordinatore; `tour.js` e `e2e_tour.py` verdi.

## A6 — Subagenti, router, `lettura/1`, prompt come dati
Prerequisiti: A3, A4, A5 unite.
Leggi: `01-architettura.md` §7–9, `prompt/01`–`08`, `docs/agenti/llm.md` §Agente.
Tocca: nuovi `crates/mzlab-llm/src/{assist.rs, reading.rs}`, `crates/mzlab-llm/data/assist/` (`01_sistema.md` … `08_grafici.md`, `assist-ruoli-1.json`, `param-scelte-1.json`), `crates/mzlab-llm/src/{agent.rs, tools.rs, lib.rs}`, `crates/mzlab-llm-wasm` (classe `Assist`), `docs/schemi/{lettura-1.json, param-scelte-1.json, assist-ruoli-1.json}`, `docs/schemi/strumenti-1.json` (rigenerato), test.
1. Ruoli in `assist-ruoli-1.json`: `{role, prompt, model, effort, max_tokens, output_schema, input_sources}` per router, metadati, cromatografia, parametri, interprete, grafici; nessun modello o effort scritto nel codice.
2. Macchina: `Output::Sub{role, request}` (la richiesta completa, cache 5 min sul sistema del ruolo) e `Input::SubResult{role, body|error}`; il router gira prima di ogni messaggio dell'utente e decide quali dati allegare (`need_data`); l'orchestratore vede solo l'uscita validata del subagente (≤ 1 500 token), mai il suo ingresso. Istruzione di fase (`prompt/06`) come messaggio `role: "system"` a metà conversazione.
3. `reading::validate(&Lettura, &input) -> Result<(), Vec<Problem>>` per `lettura/1` (azioni `none|ask|set_param|add_step|split_series|chart`) e `param-scelte/1` (`prompt/05`): `evidence` esistenti nell'ingresso, ogni numero di `numbers` e di ogni frase presente nell'ingresso (`text::numbers`), ≤ 8 risultati, ≤ 5 domande, `set_param` nei limiti del descrittore; problemi restituiti al subagente una volta, poi la lettura si scarta e l'orchestratore lo dice.
4. Prompt: i file di `data/assist/` sono copie esatte dei blocchi ```text di `docs/studi/assistente-dati/prompt/0N_*.md`; un test confronta i byte; SHA-256 di ogni prompt nel registro `llmlog/1`.
5. Strumenti nuovi nel catalogo (classe `read` o `propose`): `data_inspect`, `params_propose`, `table_check`, `table_rows`, `ask_user`; `strumenti-1.json` rigenerato con `MZLAB_UPDATE_STEPS=1 cargo test -p mzlab-cli --lib llm_tools` a fine fusione.
Prove: registrazioni `llmreg/1` per una sessione intera su `fs_pos` (`tests/data/assist/insiemi.json`): apertura → router → metadati → domande → parametri → piano → tabella → interprete → grafico; R06 e R13 di `tests/data/assist/oro/R.jsonl` senza effetto; una lettura con un numero inventato rifiutata e ritentata una volta; politica invariata (`llm_agent` verde).
Accettazione: `cargo test -p mzlab-llm`, `cargo test -p mzlab-cli --lib llm_`, `cargo test -p mzlab-llm-wasm`; byte dei prompt uguali ai blocchi; `limit_llm` misurato.

## A7 — `mzlab-cli assist`
Prerequisiti: A6 unita.
Leggi: `docs/agenti/tutor.md` (modo con chiave), `06-valutazione.md`, `prompt/00_esecuzione-test.md`, `tests/data/assist/` (casi d'oro `assist-caso/1`, insiemi `assist-insiemi/1`, copioni `assist-copione/1`: formato in `06-valutazione.md` §6).
Tocca: nuovi `crates/mzlab-cli/src/assist.rs`, `assist_tests.rs`, `tests/data/assist/reg/` (registrazioni), `crates/mzlab-cli/src/main.rs` (una riga di sottocomando), `docs/agenti/cli.md`, nuovo `docs/agenti/assistente-dati.md`.
1. `assist run DIR --copione C (--replay R | --provider anthropic --key-file F [--yes]) [--cap-usd 10] [--hide-names]`: costruisce l'insieme del copione da `tools/dati_sintetici.py` (o da DIR), applica le `patch`, gioca i messaggi e le risposte del copione, conferma secondo `policy` L1 con `confirm` del copione, scrive il registro livello a e un riassunto JSON (costo, turni, esito per `expect`).
2. `assist eval DIR_CASI [--solo G01-G24] [--ruolo R] [--ripeti N] [--modello M --effort E] (--replay R | --provider … --yes) [--batch]`: una riga JSON per caso con i controlli chiusi di `expect` (campi, flag, parametri negli intervalli, rotte, strumenti vietati, testo vietato) e i giudizi aperti da passare a `prompt/09`; riassunto con le metriche di `06-valutazione.md` §2 e il costo; uscita 1 se una soglia non passa. Casi con `needs` non soddisfatto: saltati e contati.
3. `--batch`: i casi a chiamata singola in un lotto della Batch API (V3a), stima e tetto contati prima dell'invio, interrogazioni distanziate ≥ 60 s, risultati per `custom_id`, registrazione del lotto in `llmreg/1` per le prove.
4. Trasporto `curl` con chiave e intestazioni su stdin, come `tutor eval`; stima e conferma prima di ogni corsa con `--provider`; nessun test lo raggiunge.
Prove: `cargo test -p mzlab-cli --lib assist` con le registrazioni (anche del lotto): tutti i casi G con registrazione giusta passano; una registrazione sbagliata apposta fa fallire la soglia giusta; il tetto ferma una sessione finta prima di superarlo; nessuna chiave nelle uscite (chiave finta `sk-ant-test-0000`).
Accettazione: le prove sopra; `docs/agenti/cli.md` ≤ 25 KB (è al limite: le righe nuove vanno in `assistente-dati.md`, in `cli.md` solo il rimando).

## A8 — Pannello dell'assistente nel desktop
Prerequisiti: A7 unita e test T fatto (le carte si disegnano sulle letture vere).
Leggi: `docs/agenti/llm.md` §Pannello, `docs/agenti/flow-grafica.md`, `docs/agenti/lingue.md`, `docs/agenti/accessibilita.md`, `01-architettura.md` §8–9, `05-sicurezza.md` §5.
Tocca: `flow/lazy/flow-llm.js`, `flow/lazy/flow-llm-tools.js`, nuovo `flow/lazy/flow-llm-cards.js`, `mzlab/web/llm/llm-agente.js` (esegue `Sub`), `lang/it.js`, `lang/en.js`, `tests_e2e/e2e_llm_assist.py`, `tour.js` e `tests_e2e/e2e_tour.py` se cambia un bersaglio, `docs/agenti/llm.md`. NON tocca: CSP, `trasporto.html`, `index.html`.
1. Carte: domanda chiusa (pulsanti delle opzioni + «Other» con campo di testo; risposta come `tool_result` di `ask_user`), lettura (risultati con confidenza, prova cliccabile che porta alla vista o al passo), proposta di parametro (valore, `source`, motivo, «Apply»/«Discard»: «Apply» = una voce di `S.undo`), piano (differenza e problemi come oggi).
2. Contatore di spesa sempre visibile (speso / tetto in $, due decimali), avviso una volta all'80 %, arresto al 100 % con «Export conversation»; stima prima di ogni invio come oggi.
3. «Rules only» (senza rete, senza credito o per scelta): ispezione, proposte con «Apply», controlli della tabella e grafici predefiniti di A5, senza chiamate; `err.llm.network` detto una volta.
4. Accessibilità: carte in una regione `aria-live="polite"`, fuoco sulla prima opzione di una domanda nuova solo se il fuoco era nel pannello, tutte le azioni da tastiera, bersagli ≥ 24×24 px.
Prove: `e2e_llm_assist.py` con `replay` (sessione `fs_pos`: carte visibili, «Apply» annullabile, contatore che si ferma al tetto con una registrazione costosa finta, «rules only» senza richieste di rete, nessun host esterno), axe 0 gravi, `e2e_tour.py` verde.
Accettazione: le prove sopra; nessun testo fuori da `lang/` (`tools/controlla_i18n.py`); `tipi` a 0 errori nuovi.

## W1 — Worker portabile (dopo W0 e via del Coordinatore)
Leggi: `09-worker.md`, `02-costo.md` §4, `05-sicurezza.md`. Tocca: nuova cartella del Worker (`assist-proxy.js`, `adapters/cloudflare.js`, `adapters/memoria.js`, `wrangler.toml` senza segreti, prove), `crates/mzlab-cli/src/assist.rs` (sottocomando `invito`), `docs/agenti/assistente-dati.md`.
1. Logica in un modulo con Request/Response standard; l'adattatore dà `secret` e `store` (riserva e saldo atomici, un Durable Object per persona).
2. Regole di `09-worker.md` §2–4, con le chiavi d'errore in `lang/` per il pannello.
3. Costo come deciso in W0.
Accettazione: prove di `09-worker.md` §6 con fornitore finto e store in memoria; misura della CPU per richiesta su un flusso registrato lungo (≤ 10 ms o via dichiarata); nessuna chiave in nessun file; nessun deploy (lo fa Federico).

## W2 — CSP e scelta chiave/invito
Leggi: `09-worker.md` §7, `docs/agenti/llm.md` (trasporto), `docs/agenti/lingue.md`, `docs/agenti/accessibilita.md`. Tocca: `mzlab/web/llm/trasporto.html` (CSP `connect-src` con `https://api.anthropic.com` e l'host del Worker dato da Federico), pannello (scelta «My API key» / «Invitation code»), `lang/it.js`, `lang/en.js`, e2e.
Accettazione: e2e con `replay` e con un Worker finto su `localhost`; CSP senza altri host; chiave e codice solo nell'iframe del trasporto; axe 0 gravi.
