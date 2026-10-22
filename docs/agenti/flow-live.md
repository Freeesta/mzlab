# Flow «live»: lo scheduler a stati (`mzlab_core::flow::sched`)

Studio e piano a PR: `docs/studi/flow-live/README.md`. Questo file descrive il codice che c'è. Lo usa `flow::execute` (`run_sched.rs`); gli esecutori live (CLI, `.wasm`, pagina) arrivano dopo.

## Cosa fa
Uno scheduler puro (niente thread, file, orologio, rete) che dato un piano `mzflow/2` e il registro dei passi sa quali lavori esistono, in che stato sono e quali sono pronti, e reagisce a un piano che cambia. L'esecutore (thread o Worker) chiede lavoro con `next`, riporta `started`, `progress`, `finished`, `cached`, legge gli eventi con `events_since`. File:
`flow/sched.rs` (macchina a stati, `update`, `next`), `sched_graph.rs` (lavori, dipendenze, `plan_key`), `sched_view.rs` (stati, schede, registro eventi), `sched_events.rs` (`State`, `NodeView`, testo JSON degli eventi), prove in `sched_tests.rs` e `sched_cases.rs`.

## Lavori e dipendenze
- `JobId = (instance, item)`; `item` è il NOME dell'ingresso del piano che guida il lavoro, mai un indice. Un passo `per_item` ha un lavoro per elemento della prima porta `many` che legge (come `jobs()` di `run.rs`); un passo non `per_item` ha un lavoro (`item = None`).
- Se quella porta legge l'uscita `many` di un altro passo `per_item`, `(B, f)` dipende solo da `(A, f)`. Un passo non `per_item` che legge una porta `many`, o una porta che non guida, dipende da TUTTI i lavori a monte (barriera).
- Se la porta guida legge l'uscita di un passo che non è `per_item` (numero di elementi ignoto in anticipo) o di un passo non valido, il passo ha UN lavoro con `item = None`: la forma dei lavori cambia quando il monte diventa valido. I nomi dei lavori sono dunque stabili solo finché il monte lo è.
- Gli elementi si ordinano come i riferimenti della porta; la priorità di `next` usa la posizione dell'ingresso in `plan.inputs`.

## plan_key
SHA-256 (`budget::sha256_hex`) del testo `sched/1`: `step@versione`, `algoritmo@versione`, `Resolved::params_sha256()` (senza i parametri `noresult`), versione del programma data a `Sched::new`, poi per ogni porta nell'ordine DICHIARATO dal passo (non quello scritto nel file) l'impronta di ogni file letto (`sha256` del piano, altrimenti il testo di `identities`/`set_identity`) e le `plan_key` dei lavori a monte con il nome della porta di uscita.
Né il nome né la posizione dell'elemento entrano nella chiave: il file si identifica col contenuto. Stesso piano scritto con le chiavi JSON o l'ordine dei passi diverso = stesse chiavi; tornare a un valore già provato = la stessa chiave.
- **Ricetta del primo indice** (decisione D2): `features.detect`, `features.align`, `features.gapfill` hanno un arco in più verso il lavoro che produce l'indice del PRIMO ingresso del piano da cui discendono i dati (porta `index`, o la ricetta della sorgente HR per align). Aggiungere un file in coda non cambia nulla; aggiungerlo IN TESTA cambia la chiave di tutti i rilevamenti (e dei passi a valle), non delle conversioni né degli indici. È voluto: la ricetta HR parte dal primo indice (`run::recipe_of_layers`).
- La chiave non dipende dall'esito dei lavori a monte, quindi si conosce prima di eseguire; la chiave di oggi (`cache_key`, SHA delle uscite) resta come verifica del file preso dalla cache.

## Stati
`waiting` (ingressi non pronti) · `ready` · `running{frac}` · `done` · `reused` (dalla cache) · `stale` (c'è un'uscita di una chiave vecchia; il lavoro si rifà appena i suoi ingressi sono `done`/`reused`) · `cancelled` · `error{key,params}` · `refused` (`error.featMemory`) · `blocked{by}` (`plan.problem`, `platform.computer`, o il permesso dato a `block`) · `skipped` (un antenato è in errore, rifiutato, saltato o annullato) · `paused` (solo in vista: `ready` mentre lo scheduler è in pausa).
- Stati che restano finché la chiave non cambia: `running`, `done`, `reused`, `error`, `refused`, `cancelled`; `retry(id)` rimette in fila gli ultimi tre. Gli altri si ricavano a ogni cambio dai lavori da cui dipendono.
- `finished(id, Err(e))`: `error.featMemory` → `refused`, `err.flow.cancelled` → `cancelled`, altro → `error`.

## Aggiornare il piano
`update(&Plan) -> Vec<Command>` (solo `Cancel(JobId)`): ricalcola i lavori e le chiavi, poi per ogni lavoro con lo stesso `JobId`: chiave uguale = resta com'è (anche `running`); chiave diversa = se girava, `Cancel` e il lavoro non riparte finché l'esecutore non riporta (`finished` risponde `Discarded{key}`: il risultato è vero per la vecchia chiave, il chiamante lo mette in cache); se aveva un'uscita diventa `stale`, altrimenti `waiting`/`ready`. I lavori scomparsi: `Cancel` se in corso, evento `cancelled`, spariscono dalla vista.
I problemi di `flow::check` bloccano (`blocked`) i lavori dell'istanza interessata, il resto continua. Un ciclo dà l'evento `plan` e lascia i lavori com'erano.
- Tornare a un parametro già provato: il chiamante chiama `cached(id)` quando la `plan_key` è in cache e il lavoro va a `reused` senza girare.

## Scelta del lavoro
`next(limits{slots, mem_mb}, estimate) -> Vec<JobSpec>` NON avvia nulla: due `next` di fila danno gli stessi lavori finché non si chiama `started`. Ordine: prima gli elementi già avviati (con un lavoro `running`/`done`/`reused`), per profondità decrescente; poi l'ordine degli ingressi del piano; poi l'ordine dei passi nel file. Un lavoro che non sta in `slots` o `mem_mb` (stima in MB data dal chiamante) o che condivide una risorsa esclusiva (`set_resources`, es.
`dataset:<percorso>`) con uno in corso o già scelto viene saltato, non blocca i seguenti. In pausa `next` è vuoto. Un lavoro con una corsa annullata non ancora riportata non parte.

## Schede (aggregato per nodo)
`nodes()` dà una `NodeView` per istanza del piano: stato peggiore (`error > refused > blocked > skipped > running > stale > paused > ready > cancelled > waiting > reused > done`; tutti `reused` = `reused`, misti = `done`), `frac` = media (`done`/`reused` = 1, in corso senza frazione = indeterminata → `None`), `parte/di` = lavori finiti/totali, `ms` = somma dei lavori `done` (il tempo lo misura il chiamante; nessun tempo rimanente).

## Eventi `flowlive/1`
`events_since(seq)` = array JSON degli eventi con `seq` maggiore; `seq` cresce di 1 da 1, `rev` = numero di `update` finora (0 alla creazione). Un evento `job` a ogni cambio di stato, chiave o tempo di un lavoro, un evento `node` a ogni cambio della scheda, un evento `plan` per un piano non pianificabile. Il registro tiene gli ultimi 8192. Schema: `docs/schemi/flowlive-1.json`; prova `tests/test_flowlive_schema.py` sugli eventi registrati in un caso. Scritto con `json::Out`, senza serde.

## Casi condivisi
`crates/mzlab-core/tests/data/sched_cases.json`: passi (come dati), piano, chiamate e risposta di ognuna (`expect`); `sched_cases.rs` li riproduce. Rigenerare le risposte con `MZLAB_UPDATE_SCHED_CASES=1 cargo test -p mzlab-core --lib flow::sched` e rileggere il diff (la risposta è il contratto). Servono alla parità del `.wasm`.

## `flow::execute` sopra lo scheduler
`run.rs::execute` crea `Sched`, chiede `next` (slots 1, memoria illimitata) un lavoro alla volta, `started`, esegue con `Runner::run` su `workers` thread (pool con `Condvar`; sul wasm in sequenza), riporta `finished` e tiene i risultati per `JobId`. Gli ingressi di un lavoro si leggono dai risultati dei lavori a monte (la porta guida prende solo l'elemento di quel lavoro).
Alla fine il record si scrive come prima, livello per livello (`levels`), con `settle`: stesso `prov/1` per qualsiasi ordine di fine. Un'uscita `many` di un lavoro per elemento ha la chiave `istanza.porta[nome-ingresso]` (`[nome#k]` se un lavoro ne dà più d'una).
`Job::item` resta la posizione (vista, `Lineage`); `item_name` è l'ingresso del piano, `item_root` la radice del nome del file. Il file per elemento si chiama `istanza-<radice>.ext`: radice = nome dell'ingresso senza estensione, caratteri non sicuri in `_`, più 8 cifre di SHA-256 del nome intero se due ingressi hanno la stessa radice. La chiave della cache del CLI non contiene la posizione dell'elemento.
Prove: `flow/run_tests.rs` (10 semi × 1 e 4 thread), `crates/mzlab-feat/tests/flow.rs`, `flow_cache_tests.rs`.

## Avanzamento dentro i lavori (`mzlab_core::pace`)
- `Pace::new(sink).at(done, total)` inoltra `(done, total)` al massimo una volta per percento intero, mai all'indietro; l'ultimo richiamo è `(total, total)` (un lavoro vuoto dà `(0, 0)` una volta). I byte prodotti non dipendono dalla callback.
- Chi lo usa: `flow_centroid::job_with` (scansioni scritte da `mzml_to_centroids_with`), `flow_pace::build_index_with` (scansioni decodificate dal lettore, `Scans`, o byte letti di un mzML, `Counted`; `build_index_of` è il caso senza callback), `mzlab_feat::trace::detect_with` (picchi esaminati, per blocco). Prove: `flow_centroid_tests.rs`, `flow_pace.rs`, `lr_fullscan.rs`.
- Nel browser il worker di `TP_Mine/files/feat-worker.js` manda `{id, progress:[done,total]}` e il pool lo gira a `opts.onStep({phase, name, done, total})`; il `.wasm` `feat` deve ancora richiamare il 6° argomento di `detect_file_to`.
- Da collegare: `FileRunner` (l'esecutore passa a `job_with` e `build_index_with` una callback vuota), la conversione (`write_mzml` passa `|_, _| {}` a `Conversione::scrivi`), l'allineamento (per file letto).

## Trappole
- `finished` per un lavoro cancellato o scomparso risponde `Discarded{key}`: va messo in cache, non usato. Un risultato di un `JobId` sconosciuto dà `Unknown`.
- `set_identity` e `set_resources` valgono dal `update` o dal `next` successivo; le risorse restano legate al `JobId`.
- Il registro è copiato in `Sched::new` (nessun prestito); i passi con versione assente o parametri non risolvibili hanno un lavoro `blocked` con `plan.problem`.

## Nel `.wasm` del sito
- Lo scheduler gira nel motore `mzlab-wasm` (non nei moduli `hr`: il margine di `limit_hr` non lo regge, e nessun tetto è alzato), in un oggetto per sessione (`thread_local`) dietro la funzione `flow_sched(op, body)` (`crates/mzlab-wasm/src/campioni.rs`, `mzlab_core::flow::sched_route`). Il worker del motore (`rust-worker.js`, op `lr_flow_sched`) la serve alla rotta `api/flow/sched/<op>` (`statRoute` in `lr-map.js`); un errore è `{error_key, params}`.
- Operazioni (testo JSON in e out; un lavoro è `{"instance","item"}`): `new` (`{plan, registry, version, identities}`), `drop`, `update`, `next` (`{slots, mem_mb, mb:{"istanza|voce":stima}, mb_default}`), `started`, `progress`, `finished` (`{job, result:{ms}|{error_key,params}}`: `"accepted"`, `"discarded:<chiave>"` o `"unknown"`), `cached`, `block`/`unblock`, `retry`, `pause`/`resume`, `jobs`, `nodes`, `events` (`{since}`), `rev`, `set_identity`, `set_resources`.
- Il registro non è compilato nel motore: arriva come JSON dei passi `step/1` dal piano della pagina (`F.registry`) ed è riletto da `mzlab_core::step::registry_from_value` (porte, parametri, algoritmi, preset, dove gira un passo; i testi per le persone restano vuoti). Il motore è lo stesso che serve già `api/flow/command`: la pagina non lo aspetta.
- Parità: `tests/test_flow_sched_wasm.py` costruisce il motore per Node e `tests/flow_sched_nodo.js` esegue i casi di `sched_cases.json` (SKIP senza cargo, wasm-bindgen o node); i test nativi di `sched_route` e della rilettura del registro stanno in `flow/sched_route_tests.rs`.
- Misura del motore (wasm-opt -Oz): 1 610 624 B prima, 1 678 954 B con lo scheduler e il lettore del registro, contro il tetto 1 703 936 (residuo 24 982 B).
