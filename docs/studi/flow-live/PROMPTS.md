# Prompt delle chat DEV di Flow-Live (L1-L7)

Parte di `docs/studi/flow-live/README.md` (§4). Ogni blocco si incolla come primo messaggio di una chat DEV nuova (Sonnet) e sta in piedi da solo. Ordine: L1 → L2 → L3; L1 → L4 → L5 (dopo L2) → L6 (dopo L3); L7 dopo L1, in parallelo. Una sola PR Flow-Live alla volta su `crates/mzlab-core/src/flow/`, `crates/mzlab-cli/src/flow.rs`, `lib.rs`.

Vincoli comuni (ripetuti in ogni blocco): niente modifiche a `Cargo.toml`/`Cargo.lock`, workflow `.github/`, CSP di `index.html` o tetti `.wasm` senza autorizzazione scritta di Federico (Cargo è ammesso solo per la velocità e va chiesto al COO); `AGENTS.md` e `docs/agenti/cli.md` restano ≤ 25 600 B (oggi sono pieni: non aggiungere righe, al più riscriverne una a parità di byte); un file d'area ≤ 25 KB, una riga ≤ 600 caratteri; codice e commenti in inglese, Flow solo in inglese senza chiavi `lang/`; nessun calcolo in JS; merge solo a CI verde sul SHA esatto con i sei controlli richiesti, dopo un fold di `origin/main` (`tools/fondi_main.py`) e `tools/pre_pr.py` completo; a fine lavoro UN messaggio al COO (session_01SqVGjScVGPMmoMETWKcX2F) di ≤ 5 righe: PR, SHA a CI verde, cosa resta.

## L1 · Scheduler puro in Rust (`mzlab_core::flow::sched`)

```text
Pacchetto L1 dello studio Flow-Live (docs/studi/flow-live/README.md §2.2-2.7; leggi anche docs/agenti/rust-flow.md e crates/mzlab-core/src/flow/run.rs, order.rs, canon.rs). Ramo claude/flow-live-1 da origin/main, una PR piccola, NON unirla: manda al COO la PR e lo SHA di 40 caratteri a CI verde.

Obiettivo: uno scheduler a stati, puro (niente thread, file, orologio, rete), che dato un piano mzflow/2 e il registro dei passi sa quali lavori esistono, in che stato sono e quali sono pronti, e reagisce a un piano che cambia. Nessun chiamante in questa PR: flow::execute resta com'è.

Cosa costruire, in crates/mzlab-core/src/flow/sched.rs (nuovo; in flow/mod.rs solo la riga `pub mod sched;` e gli use pubblici) e sched_tests.rs:
1. JobId = (instance: String, item: Option<String>), dove item è il NOME dell'ingresso del piano che guida il lavoro (mai un indice). Un passo per_item ha un lavoro per elemento della prima porta many che legge (come jobs() di run.rs:142); se quella porta legge l'uscita many di un altro passo per_item, il lavoro (B,f) dipende solo da (A,f). Un passo non per_item che legge una porta many dipende da TUTTI gli elementi (barriera).
2. plan_key(job) = SHA-256 (mzlab_core::budget::sha256_hex) del testo canonico: step@versione, algoritmo@versione, Resolved::params_sha256(), versione del programma passata dal chiamante, le plan_key dei lavori a monte in ordine di porta e di ingresso del piano, l'impronta dei file del piano (sha256 se c'è, altrimenti il testo `identity` che il chiamante dà per ingresso). In HR il rilevamento dipende anche da index(primo ingresso nell'ordine del piano) (ricetta del primo indice, README §2.3, decisione D2): rendilo un arco esplicito solo per i passi features.detect/align/gapfill.
3. Stati per lavoro: waiting, ready, running{frac: Option<f64>}, done, reused, stale, cancelled, error{key,params}, refused, blocked{by}, skipped, paused. API: Sched::new(plan, registry, program_version, identities) ; update(plan) -> Vec<Command> (Cancel(JobId), e gli eventi) secondo README §2.5; next(limits{slots, mem_mb}, estimate: impl Fn(&JobId)->f64) -> Vec<JobSpec> in ordine deterministico (profondità decrescente per gli elementi già avviati, poi ordine degli ingressi del piano, poi ordine del piano); started(id), progress(id, frac), finished(id, Result<outputs>), cached(id) (il chiamante dice che la plan_key è in cache); pause()/resume(); risorse esclusive dichiarate dal chiamante per lavoro (Vec<String>): due lavori con la stessa risorsa non escono insieme da next.
4. Aggregato per nodo (README §2.4): stato peggiore con l'ordine error > refused > blocked > running > stale > ready > waiting > reused/done, frac media (done = 1, indeterminata se un lavoro in corso non ha frac), parte/di, ms sommati da quelli che il chiamante riporta.
5. Eventi flowlive/1: funzione events_since(seq) -> testo JSON (mzlab_core::json, niente serde) con seq crescente e rev = numero di update; schema finale in docs/schemi/flowlive-1.json partendo da docs/studi/flow-live/flowlive-1.draft.json, e un test python (tests/test_flowlive_schema.py) che valida gli eventi di un caso.
6. Casi condivisi: crates/mzlab-core/tests/data/sched_cases.json (piano, sequenza di chiamate, stati e comandi attesi), letti da sched_tests.rs; serviranno alla parità del .wasm (L4). Rigenerazione con MZLAB_UPDATE_SCHED_CASES=1.

Test minimi (cargo test -p mzlab-core --lib flow::sched): catena di 4 passi per elemento su 3 file: il file 1 può arrivare a detect mentre il file 3 è ancora in convert; align resta waiting finché l'ultimo detect non è done; cambiare un parametro di align rende stale solo align (detect resta done, nessun Cancel per detect); cambiare un parametro di detect con detect(f2) running dà Cancel(detect,f2) e rende stale align; aggiungere un file in coda crea solo i suoi lavori e rende stale le barriere; aggiungere un file IN TESTA invalida i rilevamenti HR (arco del primo ingresso) — documentalo; togliere un nodo cancella i suoi lavori; stessa plan_key dopo andata e ritorno di un parametro; next rispetta slots, mem_mb e risorse; due permutazioni dello stesso piano dello stesso ordine danno le stesse plan_key; nessun indice di item nelle chiavi.

Documenti: crea docs/agenti/flow-live.md (file d'area: funzioni, stati, eventi, trappole; ≤ 25 KB) e aggiungi UNA riga in fondo a docs/agenti/flow-web.md che rimanda a flow-live.md. Non toccare AGENTS.md (è pieno).
Criteri: cargo test -p mzlab-core verde, cargo clippy senza avvisi nuovi, nessun cambiamento di byte in alcun test esistente.
Vincoli: niente Cargo.toml/Cargo.lock, workflow, CSP, tetti; AGENTS.md e cli.md ≤ 25 600 B; righe ≤ 600 caratteri; codice e commenti in inglese; merge a CI verde sul SHA esatto con i sei controlli richiesti dopo fold di main e tools/pre_pr.py completo; un messaggio di ≤ 5 righe al COO (session_01SqVGjScVGPMmoMETWKcX2F).
```

## L2 · `flow::execute` sopra lo scheduler; chiavi e nomi senza indice

```text
Pacchetto L2 dello studio Flow-Live (docs/studi/flow-live/README.md §1.2, §2.3, rischi 1-2, decisione D3; file d'area docs/agenti/flow-live.md fatto da L1). Ramo claude/flow-live-2 da origin/main dopo che L1 è unito; NON unire: PR e SHA a CI verde al COO.

Obiettivo: un solo motore. flow::execute (crates/mzlab-core/src/flow/run.rs:310) usa flow::sched invece delle onde (levels), con lo stesso Runner e lo stesso prov/1; la cache della cartella di corsa non dipende più dall'indice degli item.
1. run.rs: execute crea Sched, chiede next(limits{slots: workers, mem_mb: inf}), esegue i JobSpec con run_all (thread scope nativo, sequenza sul wasm), riporta finished, ripete finché nulla è ready. Le uscite di una porta many si raccolgono nell'ordine degli ingressi del piano (come gather/settle oggi), MAI nell'ordine di fine. Un passo per_item a valle parte appena il suo elemento a monte è fatto (prima di oggi aspettava l'onda).
2. Test di determinismo: un Runner di prova che restituisce i lavori in ordine casuale con seme (10 semi) e 1/4 workers dà lo stesso prov/1 di oggi salvo ms e heap_peak_bytes (crates/mzlab-feat/tests/flow.rs e un test nuovo in run.rs).
3. crates/mzlab-cli/src/flow_cache.rs:56-81: la chiave non contiene più job.item (gli ingressi dell'elemento ne portano già lo SHA); i file per elemento si nominano dalla radice del nome dell'ingresso resa unica, non dall'indice (flow_base.rs:260 convert_file e ogni out_path per item: cerca con grep 'job.item' in crates/mzlab-cli/src). Aggiungere un ingresso in mezzo non deve ricalcolare gli altri: test in flow_cache_tests.rs.
4. Le etichette del prov/1 cambiano una volta: rigenera le costanti di tests/test_flow_determinismo.py e di ogni test che le fissa, e scrivi nella PR l'elenco delle costanti cambiate e perché (D3).
5. FileRunner::keep (crates/mzlab-cli/src/flow.rs:183) calcola lo SHA-256 a pezzi invece di leggere tutto il file (stesso risultato).
Prove: cargo test -p mzlab-core -p mzlab-feat -p mzlab-cli; python3 -m pytest -q tests/test_flow_determinismo.py tests/test_corsa_schema.py; e2e_flow_web.py se cargo e wasm-bindgen ci sono.
Documenti: riscrivi le righe di docs/agenti/rust-flow.md su execute e cache (a parità di dimensione circa: il file è a 23,5 KB) e aggiorna flow-live.md.
Vincoli: niente Cargo.toml/Cargo.lock, workflow, CSP, tetti; AGENTS.md e cli.md ≤ 25 600 B; righe ≤ 600 caratteri; una PR alla volta su flow.rs/lib.rs; merge a CI verde sul SHA esatto con i sei controlli richiesti dopo fold di main e tools/pre_pr.py completo; un messaggio di ≤ 5 righe al COO (session_01SqVGjScVGPMmoMETWKcX2F).
```

## L3 · Esecutore nativo live, `--jsonl` e rotte desktop

```text
Pacchetto L3 dello studio Flow-Live (docs/studi/flow-live/README.md §2.5-2.8, §1.3; docs/agenti/flow-live.md, rust-desktop.md). Ramo claude/flow-live-3 da origin/main dopo L2; NON unire: PR e SHA a CI verde al COO.

Obiettivo: sul computer il piano gira in continuo e accetta modifiche mentre gira.
1. crates/mzlab-cli/src/flow_live.rs (nuovo; in flow.rs solo la riga che lo registra): LiveRun { sched, pool di std::thread (cores-1, limite 4 come crates/mzlab-desktop/src/flow_run.rs:66), Arc<AtomicBool> di annullamento per lavoro passato al Runner, cache della cartella di lavoro per plan_key }. API: start(plan, dir, work), update(plan_text) (debounce lato pagina), pause/resume, cancel_all, events_since(seq). Un lavoro annullato che il passo non interrompe finisce, va in cache e il risultato non si usa se la chiave non è più attesa. Nessuna dipendenza nuova (niente Rayon in mzlab-cli).
2. mzlab-cli flow run --jsonl stampa su stdout gli eventi flowlive/1 (stesso schema docs/schemi/flowlive-1.json), una riga per evento; senza --jsonl l'uscita resta identica. La riga d'uso in flow_run.rs:22 si aggiorna; cli.md è pieno: riscrivi la riga di flow run a parità di byte o metti il dettaglio in flow-live.md.
3. crates/mzlab-desktop/src/flow_live.rs (nuovo; in rpc.rs solo le righe del dispatch): rotte flow/live/start {plan, files}, flow/live/update {plan}, flow/live/events {since} -> {events, seq}, flow/live/pause, flow/live/resume, flow/live/cancel. flow/run e flow/progress restano (la pagina passa a live in L6).
4. La coda di conversione (crates/mzlab-desktop/src/conv_coda.rs) entra nel grafo: un ingresso del produttore il cui mzML è pronto (conv_ready) dà il lavoro import/convert come done con la sua chiave; uno in conversione è running (frac dalle scansioni se conv_list le ha); la coda continua a cedere i core all'indice di Dati (INDEXING).
Prove: cargo test -p mzlab-cli --lib flow_live (piano sintetico di tools/dati_sintetici.py: update a metà corsa che cambia un parametro di align non rifà i rilevamenti; un annullamento non lascia file a metà; --jsonl valida lo schema; stessi file finali di flow run); cargo test -p mzlab-desktop se la CI lo prevede (job desktop).
Vincoli: niente Cargo.toml/Cargo.lock, workflow, CSP, tetti; AGENTS.md e cli.md ≤ 25 600 B; righe ≤ 600 caratteri; codice e commenti in inglese; merge a CI verde sul SHA esatto con i sei controlli richiesti dopo fold di main e tools/pre_pr.py completo; un messaggio di ≤ 5 righe al COO (session_01SqVGjScVGPMmoMETWKcX2F).
```

## L4 · Lo scheduler nel `.wasm` del sito

```text
Pacchetto L4 dello studio Flow-Live (docs/studi/flow-live/README.md §2.8, rischio 3, decisione D4; docs/agenti/flow-live.md, flow-web.md, wasm-tetti.md, mzfeat-wasm.md). Ramo claude/flow-live-4 da origin/main dopo L1; NON unire: PR e SHA a CI verde al COO.

Obiettivo: il sito ha lo stesso scheduler di Rust, in un Worker, senza .wasm nuovi né tetti nuovi.
1. crates/mzlab-feat-wasm/src/sched.rs (nuovo, dietro la feature esistente `detect`, che il modulo hr ha già: NESSUNA feature nuova in Cargo.toml): esporta con wasm-bindgen un oggetto con new(plan_text, registry_text, program_version, identities_json), update(plan_text) -> json comandi, next(limits_json) -> json JobSpec, started/progress/finished/cached, pause/resume, events_since(seq). Testi JSON con mzlab_core::json, niente serde.
2. flow/lazy/worker-flow-web.js ospita lo scheduler (carica già il feat per stima_series): messaggi {op:"sched.*"}; nessuna logica di scelta in JS.
3. Parità: tests/flow_sched_nodo.js esegue crates/mzlab-core/tests/data/sched_cases.json sul .wasm e deve dare gli stessi stati e comandi di sched_tests.rs; lanciato da un test pytest (SKIP senza cargo/wasm-bindgen, come e2e_flow_web.py).
4. Misura: costruisci con tools/build_feat_wasm.sh e confronta con limit_hr (oggi 1 212 416 B, misura 1 099 407 B). Se la misura supera il tetto FERMATI, non toccare il tetto, scrivi al COO la misura e lo scarto (decisione D4 di Federico).
Vincoli: niente Cargo.toml/Cargo.lock, workflow, CSP, tetti; AGENTS.md e cli.md ≤ 25 600 B; mzfeat-wasm.md è pieno: la riga va in flow-live.md; righe ≤ 600 caratteri; merge a CI verde sul SHA esatto con i sei controlli richiesti dopo fold di main e tools/pre_pr.py completo; un messaggio di ≤ 5 righe al COO (session_01SqVGjScVGPMmoMETWKcX2F).
```

## L5 · «This browser» diventa live

```text
Pacchetto L5 dello studio Flow-Live (docs/studi/flow-live/README.md §1.4, §2.5-2.9; docs/agenti/flow-live.md, flow-web.md, avvio-rapido.md, mzfeat-wasm.md). Ramo claude/flow-live-5 da origin/main dopo L4 e L2, e dopo che la PR e2e di DEV Flow-MSn su flow-web-run.js/flow-web-msn.js è unita; NON unire: PR e SHA a CI verde al COO.

Obiettivo: nel sito, mentre l'utente costruisce il grafo, i passi che il browser sa fare partono da soli per file e si rifanno solo dove la modifica tocca.
1. flow/lazy/flow-live.js (nuovo, // @ts-check, in tools/tipi/pagina.json, in LATE di flow/js/flow-tool.js: MAI nel primo disegno, regola di avvio-rapido.md): a flow:changed (debounce 300 ms) manda il piano al worker (sched.update); esegue i comandi: start(JobSpec) verso il pool di TP_Mine/files/feat-worker.js (detect per file, align), F.webMsn per i passi MSn, il nodo index.mzidx osserva l'indice di Dati (evento mzlab-index, docs/agenti/progresso.md) e non costruisce mai un secondo indice; cancel(JobId) verso il pool. Riporta started/progress/finished; emette ancora flow:web-step (derivato) per compatibilità.
2. TP_Mine/files/feat-worker.js: cancelJob(id) uccide solo il worker che esegue quel lavoro (oggi cancel() a :321 li uccide tutti); il pool ritorna con id di lavoro in detectAll/align.
3. flow/lazy/flow-web-run.js: run(S) diventa «avvia il live e aspetta che tutti i nodi browser siano done/reused» (stessa firma, stessi eventi, e2e_flow_web.py verde senza modifiche alle asserzioni); le chiavi OPFS passano a plan_key (mzlab/flow/live/<plan_key>.*); le vecchie .key si ignorano.
4. Memoria: next riceve slots = posti liberi del pool e mem_mb = budget di MZLAB_RUST; un align oltre il budget va blocked (permission.memory) e chiede «Use more memory» solo al clic dell'utente, non a metà modifica; resta il Web Lock mzlab-mem-grande oltre 512 MB.
Prova: tests_e2e/e2e_flow_live.py (nuovo; costruisce il sito; file sintetici): aprendo Flow con 3 .mzidx in OPFS i rilevamenti partono senza clic su Run; cambiare un parametro di align mentre i rilevamenti girano non li rifà (conta i lavori del pool); cambiare un parametro di detect annulla solo i rilevamenti in corso; tornare al valore di prima riusa (reused); nessuna pausa fissa, si contano eventi; window.MZLAB_LR_MISS vuoto; MISURA flow_live_primo_detect (clic → primo evento running). Più e2e_flow_web.py, e2e_flow.py, e2e_flow_avvio.py verdi.
Vincoli: niente Cargo.toml/Cargo.lock, workflow, CSP, tetti; AGENTS.md e cli.md ≤ 25 600 B; righe ≤ 600 caratteri; Flow solo in inglese, nessuna chiave lang/; nessun calcolo in JS; merge a CI verde sul SHA esatto con i sei controlli richiesti dopo fold di main e tools/pre_pr.py completo; un messaggio di ≤ 5 righe al COO (session_01SqVGjScVGPMmoMETWKcX2F).
```

## L6 · Stato, % e tempo sulle carte; desktop sullo stesso modello

```text
Pacchetto L6 dello studio Flow-Live (docs/studi/flow-live/README.md §2.4, §2.9; docs/agenti/flow-live.md, flow-grafica.md, accessibilita.md). Ramo claude/flow-live-6 da origin/main dopo L5 e L3; NON unire: PR e SHA a CI verde al COO.

Obiettivo: ogni carta della tela dice cosa sta facendo il suo nodo, e la pagina resta modificabile.
1. flow/lazy/flow-canvas.js e flow-shell-css.js: riga di stato su ogni carta (icona + parola: waiting, k of n files, NN %, done in X s, reused, out of date, error: motivo, needs a computer, needs your OK for memory), barra role=progressbar con aria-valuenow (senza valore se indeterminata), contorno tratteggiato per stale; colori solo da variabili CSS (Okabe-Ito), forma oltre al colore; prefers-reduced-motion e data-a11y-reduce spengono ogni animazione; bersagli ≥ 24 px (40 al tocco). Lo stato viene SOLO dagli eventi flowlive/1 kind node (nessun calcolo in JS). Mai un tempo rimanente.
2. Pannello del passo (flow-inspect.js): tabella «Files» (file, stato, %, tempo, memoria) dagli eventi kind job.
3. Barra: Live/Pause/Resume (Live acceso per difetto, decisione D1), «Run now» solo dove un nodo è blocked da un permesso; #flx-live annuncia solo i cambi di stato dei nodi, al più uno ogni 2 s.
4. flow/lazy/flow-desktop.js: «This computer» usa flow/live/* (L3) invece di flow/run + polling di flow/progress; stessa tela e stessi testi del sito.
5. Tour: tour.js e tests_e2e/e2e_tour.py restano validi (ogni passo trova il bersaglio).
Prove: tests_e2e/e2e_flow_live.py esteso (stati e % sulle carte, out of date dopo una modifica, axe senza violazioni con e2e_axe_grafici, tastiera); e2e_flow.py, e2e_flow_avvio.py (nessun modulo live nel primo disegno), e2e_tour.py verdi; tsc 0 errori nuovi (base_errori.json non sale).
Vincoli: niente Cargo.toml/Cargo.lock, workflow, CSP, tetti; AGENTS.md e cli.md ≤ 25 600 B; flow-grafica.md è a 24,5 KB: le righe nuove vanno in flow-live.md e in flow-grafica.md si riscrive solo la riga «Avanzamento della corsa»; Flow solo in inglese; merge a CI verde sul SHA esatto con i sei controlli richiesti dopo fold di main e tools/pre_pr.py completo; un messaggio di ≤ 5 righe al COO (session_01SqVGjScVGPMmoMETWKcX2F).
```

## L7 · Avanzamento dentro i lavori lunghi

```text
Pacchetto L7 dello studio Flow-Live (docs/studi/flow-live/README.md §2.4, decisione D5; docs/agenti/flow-live.md, progresso.md, lettura-nativa.md, mzfeat.md). Ramo claude/flow-live-7 da origin/main dopo L1 (in parallelo a L2-L6, ma senza toccare run.rs, flow.rs, flow-web-run.js); NON unire: PR e SHA a CI verde al COO.

Obiettivo: le carte mostrano il % anche DENTRO un file per i passi lunghi.
1. Una callback di avanzamento opzionale step(done, total) (stesso stile di quella dei lettori in crates/mzlab-wasm/src/nativi.rs, Contato) in: conversione/centroidi (crates/mzlab-cli/src/flow_centroid.rs, convert.mzml), costruzione dell'indice, rilevamento delle feature (crates/mzlab-feat, per scansione o per finestra di m/z), allineamento (per file letto). Mai più di un richiamo ogni 1 % circa; nessun effetto sui byte prodotti (test: stessi SHA con e senza callback).
2. Il Runner nativo e il pool di feat-worker.js la inoltrano come progress(id, frac) allo scheduler (solo il trasporto; se L5 non è ancora unito, solo la parte Rust e il messaggio del worker).
Prove: cargo test dei crate toccati (frazioni monotone, ultima = 1, stessi byte), tests/test_nativo.py e i test di monotonia di progresso.md verdi.
Vincoli: niente Cargo.toml/Cargo.lock, workflow, CSP, tetti (misura i .wasm: lib.wasm_cap; se un tetto si supera FERMATI e scrivi al COO); AGENTS.md e cli.md ≤ 25 600 B; righe ≤ 600 caratteri; merge a CI verde sul SHA esatto con i sei controlli richiesti dopo fold di main e tools/pre_pr.py completo; un messaggio di ≤ 5 righe al COO (session_01SqVGjScVGPMmoMETWKcX2F).
```
