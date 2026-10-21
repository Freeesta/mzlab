# Prompt DEV del tutor sulla Teoria (T1-T5)

Studio: `docs/studi/tutor-teoria/README.md`. Ogni prompt è completo: il Coordinatore lo copia così com'è. Una PR per prompt, in ordine; T2 dopo D2 dello studio LLM (`docs/studi/llm/README.md` §13).

**Intestazione comune** (vale per T1-T5): scrivi SOLO al Coordinatore (`send_message`, sessione indicata da Federico); mai a Federico; UN messaggio a fine PR (max 5 righe: PR, SHA, CI, pages, cosa resta) o su un blocco vero (una riga con le 2 opzioni, poi continua). Ramo `claude/tutor-tN` da `origin/main`, unione con merge commit dopo CI verde e `verifica.py --cambiati`.
Vincoli fissi: calcolo e logica solo in Rust; JS solo colla e interfaccia; Python solo test; codice e commenti in inglese; testi dell'app solo in `lang/it.js` e `lang/en.js`; nessun file di `mzlab/web/teoria/` cambia; nessuna dipendenza esterna nuova; nessuna chiamata di rete vera in CI; nessuna chiave, nessun account, nessun deploy (azioni di Federico, §7 dello studio); nessuna domanda vera di studenti nel repository.
Subagenti Haiku: `cercatore`, `verificatore`, `controllo-github` (mai codice, asserzioni o commit). Documenti: una riga per funzione nuova in `docs/agenti/tutor.md` (nuovo in T1, una riga in AGENTS.md §5 «tutor sulla Teoria»); righe ≤ 600 caratteri.

### T1 — Blocchi della Teoria e ricerca BM25 in Rust (PR 1 di 5)
Prerequisiti: nessuno.
Tocca: `Cargo.toml` (membro), nuovo `crates/mzlab-tutor/` (`Cargo.toml` con dipendenze `mzlab-libro`, `mzlab-core`; `src/lib.rs`, `src/blocchi.rs`, `src/cerca.rs`, `src/bin/teoria-blocchi.rs`, `tests/blocchi.rs`, `tests/cerca.rs`), `crates/mzlab-libro/src/lib.rs` + `scan.rs` + `index.rs` (rendere pubblici `slug` e una funzione `sinonimi()` che espone `SINONIMI`, nessun cambio di comportamento)
Tocca (segue): `.github/workflows/pages.yml` (un passo dopo i `.wasm`: `cargo run --locked --release -p mzlab-tutor --bin teoria-blocchi -- mzlab/web/teoria site/static/tutor/blocchi-1.json`), nuovo `docs/schemi/teoria-blocchi-1.json`, nuovo `tests/test_tutor_blocchi.py`, nuovi `tests/data/tutor/eval/*.json` (solo recupero), `docs/agenti/tutor.md`, AGENTS.md §5 (una riga).
NON tocca: `mzlab/web/teoria/`, `tools/genera_indice_teoria.py`, `indice-ricerca.js`, i `.wasm` del sito.
Leggi: §3.2, §3.6 dello studio; `crates/mzlab-libro/src/index.rs` (lettura di h2/h3, `slug`, `SINONIMI`); `mzlab/web/teoria/LEGGIMI.md` righe 40-70 (riquadri).
PR «Tutor T1: i paragrafi della Teoria in blocchi, con la ricerca in Rust»:
1. `blocchi::build(pages: &[(name, html)]) -> Result<Value>` (`teoria-blocchi/1`): salta le pagine con `http-equiv="refresh"`; blocchi = paragrafi sotto lo stesso h2/h3 fino a ~800 caratteri; glossario `<dt id="g-…">` + `<dd>` = un blocco; `details.figdesc` unito alla figura; `details.q` e `details.poe` → `kind: "esercizio"` con la sola `summary`, e accanto `hidden_sha` + `hidden_shingles` (5-grammi normalizzati della risposta, mai il testo); campi `id` (`file#ancora.k`)
   (segue) `file`, `anchor`, `chapter`, `title`, `kind`, `text`, `sha` (12 esadecimali di `mzlab_core::budget::sha256_hex`); chiavi ordinate, stessi byte a ogni corsa.
2. `cerca::Index::new(&Value)` e `search(&self, query, where_: Option<&str>, k) -> Vec<(id, score)>`: BM25 (k1 1,2, b 0,75) su testo normalizzato senza accenti e maiuscole come `cerca.js`, sinonimi di `mzlab_libro::sinonimi()`, +20 % ai blocchi della sezione `where_`; i blocchi `esercizio` concorrono con la sola domanda.
3. Binario `teoria-blocchi DIR OUT [--controlla]`: scrive il JSON; con `--controlla` confronta con OUT e torna 1 se diverso.
4. Casi di recupero (≥ 60, sintetici, scritti da te leggendo la Teoria: `question`, `lang`, `expect_anchors`, `intent`; categorie del §3.6) e `cerca::recall_at(cases, 5)`.
Accettazione: `cargo test -p mzlab-tutor` (stessi byte due volte; nessun testo di `details.q` nei `text`; un blocco ≤ 1 200 caratteri; ogni id unico; ogni `anchor` esiste come `id=` nella pagina; richiamo@5 ≥ 0,85 sui casi italiani e misurato su quelli inglesi, scritto in `tutor.md`); `cargo test -p mzlab-libro` invariato; `tests/test_tutor_blocchi.py` (rigenera in una cartella temporanea, 400-450 blocchi o il numero misurato ± 10 %, ≤ 400 KB, schema valido); `tests/test_indice_teoria.py` verde; nessuna dipendenza nuova (`cargo tree -p mzlab-tutor`).
Tetti: nessun `.wasm` del sito cambia. Conflitti caldi: `pages.yml` (un solo passo, dopo `build_feat_wasm.sh`), `mzlab-libro/src/lib.rs` (solo `pub use`).

### T2 — Logica del tutor: scelta, richiesta, controlli, tetti, eval (PR 2 di 5)
Prerequisiti: T1 unita; D2 dello studio LLM unita (`crates/mzlab-llm`: `Provider`, `Conversation`, `provider::request`, `Stream`, `fake::Replay`). Se D2 manca: fermati e scrivilo al Coordinatore.
Tocca: `crates/mzlab-tutor/Cargo.toml` (dipendenza `mzlab-llm`), nuovi `src/{ask.rs, pick.rs, answer.rs, checks.rs, quota.rs, stat.rs, eval.rs}`, nuovo `crates/mzlab-tutor/data/tutor-sistema-1.md` e `tutor-scelta-1.md` (inglese), `crates/mzlab-cli/src/lib.rs` (una riga `Some("tutor")` in `execute`, una in `HELP`, `mod`), nuovo `crates/mzlab-cli/src/tutor.rs`, nuovi `docs/schemi/{tutor-ask-1.json, tutorstat-1.json}`, `tests/data/tutor/*.sse` e `*.jsonl` (registrazioni sintetiche), `docs/agenti/tutor.md`, `docs/agenti/cli.md` (una riga).
NON tocca: `crates/mzlab-cli/src/flow.rs`, `mzlab/web/`, il Worker.
Leggi: §3.3-3.6 dello studio; `docs/agenti/llm.md` (contratti di D2).
PR «Tutor T2: dalla domanda alla risposta citata, provato senza rete»:
1. `ask::parse(&str) -> Result<Ask>` (`tutor-ask/1`, limiti del §3.4: domanda ≤ 600 caratteri, storia ≤ 2 turni e ≤ 1 500 caratteri) con errori `err.tutor.*`.
2. `pick::request(&Ask, candidates, &Provider, model, effort) -> Request` e `pick::parse(text) -> Pick{intent, keywords_it, ids}` (JSON rigido, id fuori dai candidati scartati, ≤ 5).
3. `answer::request(&Ask, blocks, level, &Provider, model, effort) -> Request`: sistema fisso con SHA-256, blocchi numerati e delimitati come dati, `thinking` adattivo, `output_config.effort` dal parametro, `max_tokens` 1 500; intent `esercizio`/`dati_lab`/`fuori_tema` → istruzione di rimando (§3.5); `answer::passages(blocks)` per il modo `passi`.
4. `checks::run(answer_text, sent_blocks, hidden) -> Checks{cites_ok, bad_sentences, numbers_ok, solution_overlap}` (§3.4 punto 4, soglia 0,15).
5. `quota::{Limits, State, admit(&State, &Limits, now_day) -> Result<()>, charge(&mut State, usage, &Prices)}`: per studente (USD e domande al giorno), per impronta di IP, per giorno globale, globale; tutto in centesimi interi.
6. `stat::{Stat, add(…)}` (`tutorstat/1`, livello a: nessun testo) e `stat::share_text(q) -> Option<String>` (livello b: toglie email, numeri di 6+ cifre, telefoni; ≤ 600 caratteri).
7. `eval::score(case, answer, checks, usage) -> Score` e `mzlab-cli tutor eval CASES --blocchi F (--replay DIR | --solo-recupero | --provider ID --key-file F --pick M --answer M --effort E)`; il modo con chiave è solo manuale (stampa i costi stimati prima e chiede `--yes`).
Accettazione: `cargo test -p mzlab-tutor` (corpi delle due richieste uguali carattere per carattere ai file attesi; `pick::parse` su 10 risposte sintetiche anche malformate; `checks` su risposte con citazione inventata, numero inventato, risposta di un `details.q` copiata; `quota` ai bordi: tetto esatto, cambio di giorno, globale prima dello studente; `share_text` toglie le tre forme); `mzlab-cli tutor eval tests/data/tutor/eval --replay tests/data/tutor` = 100 % sulla macchina del punteggio; `cargo test -p mzlab-cli` verde; guardia delle chiavi verde; nessuna dipendenza nuova.
Tetti: nessuno. Conflitti caldi: `mzlab-cli/src/lib.rs::execute` (una riga), `cli.md` (una riga).

### T3 — Worker Cloudflare (codice e prove, senza deploy) (PR 3 di 5)
Prerequisiti: T2 unita.
Tocca: nuovo `crates/mzlab-tutor-wasm/` (wasm-bindgen come `mzlab-libro-wasm`: `admit`, `search`, `pick_request`, `pick_parse`, `answer_request`, `stream_push`, `checks`, `charge`, `stat_add`), `Cargo.toml` (membro), nuovi `servizi/tutor-worker/{worker.js, contatori.js, wrangler.toml, LEGGIMI.md}`, nuovo `tools/build_tutor_worker.sh` (cargo + wasm-bindgen + `tools/wasm_opt.sh` con `limit_tutor`, uscita in `servizi/tutor-worker/dist/`, mai committata, in `.gitignore`)
Tocca (segue): nuovi `tests/tutor_worker_node.mjs` e `tests/test_tutor_worker.py` (Node già usato da `tests/teoria_editor_node.mjs`), `tests/test_llm_chiavi.py` o la guardia equivalente (aggiunge `servizi/`), `docs/agenti/tutor.md`.
NON tocca: `mzlab/web/`, `pages.yml`, CI (il Worker non si costruisce in CI oltre al test).
Leggi: §3.4 e §6 dello studio.
PR «Tutor T3: il Worker che tiene la chiave, con i tetti e senza registri»:
1. `worker.js` (modulo ES, ~200 righe di colla): rotte `/v1/ask`, `/v1/feedback`, `/v1/status`, `/v1/stats` del §3.4; CORS solo per `env.ORIGINS`; legge `blocchi-1.json` da `env.BLOCCHI_URL` con Cache API (1 h); chiama il fornitore con `env.ANTHROPIC_KEY` (segreto) e passa lo stream normalizzato alla pagina; nessun `console.log` di corpi, domande o risposte.
2. `contatori.js`: Durable Object con archivio SQLite: stato di `quota` per corso, studente (HMAC), impronta IP del giorno (sale casuale in memoria, rigenerato al cambio di giorno, mai scritto), `tutorstat/1`; livello b solo se `env.LEVEL_B === "true"` e `share: true`, con allarme che cancella oltre 90 giorni.
3. Parametri come variabili in `wrangler.toml` (senza id di account): `MODEL_PICK`, `MODEL_ANSWER`, `EFFORT_PICK`, `EFFORT_ANSWER`, `CAP_STUDENT_USD`, `DAY_STUDENT`, `DAY_IP`, `CAP_GLOBAL_USD`, `DAY_GLOBAL_USD`, `PRICES`, `ORIGINS`, `BLOCCHI_URL`, `LEVEL_B = "false"`, `COURSES = "{}"` (spento), `observability.enabled = false`; segreti elencati solo per nome nel `LEGGIMI.md` (`ANTHROPIC_KEY`, `HMAC_KEY`, `ADMIN_TOKEN`) con i comandi per Federico.
4. Errore del fornitore, credito finito o tetto → `mode: passi` senza costo.
Accettazione: `tests/test_tutor_worker.py` lancia `node tests/tutor_worker_node.mjs` con `fetch` e Durable Object finti: corso spento → `err.tutor.course`; origine diversa → 403; domanda di 601 caratteri → `too_long`; 26ª domanda del giorno → `rate`; tetto studente → `quota_student` e la domanda dopo esce in `passi`; tetto globale → `passi` per tutti; risposta SSE finta → eventi `meta`, `delta`, `done` con `checks`; nessuna scrittura contiene il testo della domanda con `LEVEL_B` spento; `stats` senza token → 401;
Accettazione (segue): nessun `console.*` chiamato con la domanda (spia). `.wasm` del Worker ≤ `limit_tutor` (misura × 1,10 a 4 KiB, dichiarata). Guardia delle chiavi verde.
Tetti: solo `limit_tutor` nuovo; i tetti del sito invariati. Conflitti caldi: nessuno (cartelle nuove).

### T4 — Pagina del tutor, avviso, pulsante nella scheda Teoria (PR 4 di 5)
Prerequisiti: T3 unita (il contratto); il deploy non serve.
Tocca: nuovi `mzlab/web/tutor/{index.html, tutor.js, tutor.css, config.json}` (`// @ts-check`, progetto in `tools/tipi/`), `mzlab/web/barra.js` (un pulsante «Tutor» nel contesto Teoria accanto ai comandi `TCMD`, che apre `tutor/index.html?where=<file#ancora visibile>` in una scheda nuova; visibile solo se `tutor/config.json` ha `enabled: true`), `mzlab/web/icons*.js` (`QICON` per l'icona), `lang/it.js`, `lang/en.js` (`tutor.*`), `tools/build_site.py` (nulla se la copia di `mzlab/web` basta; verifica)
Tocca (segue): nuovi `tests/test_tutor_csp.py`, `tests_e2e/e2e_tutor.py`, `tests_e2e/tutor_finto.py`, `tests/data/tutor/finto/*.sse`, `docs/agenti/tutor.md`.
NON tocca: `mzlab/web/teoria/`, `index.html` (CSP invariata), `tour.js`, `rust-worker.js`.
Leggi: §3.5, §6 dello studio; `docs/agenti/accessibilita.md`, `docs/agenti/lingue.md`, `mzlab/web/libro/editor.html` (CSP propria).
PR «Tutor T4: la pagina delle domande, spenta finché il corso non si accende»:
1. CSP propria: `default-src 'self'; connect-src 'self' <endpoint di config.json>; frame-ancestors 'self'`; con `endpoint` vuoto il tutor mostra «non attivo» e non fa richieste.
2. Primo uso: `<dialog>` dell'avviso (§6, `tutor.avviso.*`, versione in `mzlab.tutor.avviso`), poi il codice del corso (`GET /v1/status`), poi il campo della domanda; scelte «Usa il tutor» e «Solo paragrafi» (in «Solo paragrafi» nessuna richiesta: link alla ricerca della Teoria).
3. Risposta in stream con citazioni `[n]` come link a `../teoria/<file>.html#<ancora>`, frase con «citazione non verificata» se `checks` lo dice, livello base/approfondito, pollice in giù con motivo, casella «Condividi questa domanda» solo se `/v1/status` dice livello b acceso (spenta ogni volta), residuo del tetto.
4. Conversazione solo in memoria; `mzlab.tutor.id` creato con `crypto.getRandomValues`.
Accettazione: `tests/test_tutor_csp.py` (`index.html` e `teoria/*.html` invariati; un solo host esterno, uguale a `config.json`); `e2e_tutor.py` con `tutor_finto.py` servito da `'self'`: `enabled: false` → nessun pulsante; prima dell'avviso nessuna richiesta (ascoltatore che fallisce su ogni host esterno e conta le richieste al finto); avviso → codice → domanda → risposta con 3 citazioni che aprono l'ancora giusta; modo `passi`; errore `rate` mostrato senza bloccare; tastiera sola; axe 0 violazioni gravi; bersagli ≥ 24 px; `prefers-reduced-motion`;
Accettazione (segue): telefono (`?telefono=1`) leggibile; `e2e_tour`, `e2e_menu_coerenti` verdi; `tools/controlla_i18n.py` verde; job `tipi` verde con base invariata.
Tetti: nessuno. Conflitti caldi: `barra.js` (un blocco nel contesto Teoria), `lang/*.js` (solo chiavi `tutor.*`).

### T5 — Contatori per Federico e livello b dietro interruttore (PR 5 di 5)
Prerequisiti: T4 unita.
Tocca: nuovi `mzlab/web/tutor/stat.html` + `stat.js` (pagina amministratore, `mzlab.admin`, token chiesto ogni volta e tenuto in memoria, CSP propria come T4), `crates/mzlab-tutor/src/stat.rs` (`stat::report(&Value) -> Value`: blocchi più scelti con capitolo e titolo, risposte «non coperto» per giorno, voti negativi per motivo e per blocco, costo per giorno), `crates/mzlab-tutor-wasm` (`stat_report`), `lang/it.js`, `lang/en.js` (`tutor.stat.*`), `tests_e2e/e2e_tutor_stat.py`, `docs/agenti/tutor.md`.
NON tocca: il Worker salvo la rotta `stats` se manca un campo; `LEVEL_B` resta `"false"`.
Leggi: §6 dello studio; `docs/agenti/dati.md` (`TDATI`).
PR «Tutor T5: che cosa chiedono gli studenti, senza leggere le loro domande»:
1. Tabelle e un grafico a barre dei blocchi più scelti con la sua tabella dei dati (`TDATI`, Okabe-Ito, bersagli ≥ 24 px).
2. Sezione livello b visibile solo se il Worker la dichiara accesa; elenco delle domande condivise con giorno e blocchi, pulsante «Cancella tutto» (rotta `DELETE /v1/stats/shared` con token, aggiunta al Worker con test in `tutor_worker_node.mjs`).
Accettazione: `cargo test -p mzlab-tutor` (`report` d'oro su un `tutorstat/1` sintetico); `e2e_tutor_stat.py` con il Worker finto: senza token nulla; con token tabelle e grafico, tabella dei dati presente; livello b spento → sezione assente; axe 0 gravi; `e2e_axe_grafici` verde.
Tetti: nessuno. Conflitti caldi: `lang/*.js` (solo `tutor.stat.*`).
