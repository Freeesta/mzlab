# mzFlow «live»: esecuzione continua e incrementale del grafo

Studio ARCH per la richiesta di Federico: «mzFlow deve poter permettere di lavorare mentre lui carica i vari moduli. All'inizio ho i file raw grezzi. Li carico. Mentre costruisco il grafo prendendo tutto e mettendolo, lui già può trasformarli in mzML centroidati; se c'è già una feature table impostata comincia a farla. Le varie box del grafo mostrano la percentuale di completamento ma io posso continuare a modificare il grafo.»

Tradotto in requisiti: (R1) ogni nodo parte appena ingressi e parametri sono pronti; (R2) a livello di FILE: il file 1 avanza nella catena mentre il file 5 si converte; (R3) il grafo resta modificabile mentre gira; (R4) ogni box mostra stato (in attesa / in corso / fatto / obsoleto / errore), % e tempo; (R5) una modifica invalida solo i nodi a valle toccati, riusa la cache per impronta (ingressi + parametri + versione del passo) e annulla/riavvia solo i lavori obsoleti, senza bloccare l'interfaccia; (R6) l'interfaccia compare subito (regola di avvio rapido, `docs/agenti/avvio-rapido.md` con #1444); (R7) stesso modello sul web (Worker + `.wasm`, tetti di `docs/agenti/wasm-tetti.md`) e su desktop/CLI; (R8) deterministico: stessi byte per stessi ingressi, qualunque sia l'ordine in cui i lavori finiscono; (R9) memoria limitata (contropressione).

Prompt delle chat DEV: `PROMPTS.md` (stessa cartella). Bozza dello schema degli eventi: `flowlive-1.draft.json`.

## 1. Stato attuale (main `b496f6b`)

### 1.1 Motore Rust (`mzlab_core::flow`)
- `flow::execute(plan, registry, runner, workers)` (`crates/mzlab-core/src/flow/run.rs:310`) controlla il piano (`:311`), poi esegue **a onde**: `for wave in levels(plan)?` (`:320`; `levels` = profondità topologica, `order.rs:70`). Per ogni onda risolve e legge gli ingressi (`:323-340`), espande i lavori (`jobs`, `:142`: un lavoro, o uno per elemento della prima porta `many` se il passo è `per_item`, `:153-171`) e li lancia **tutti insieme**, aspettando la fine dell'onda (`run_all`, `:69`: `std::thread::scope` con un contatore atomico, `:77-86`; sul wasm in sequenza, `:93-94`).
- Conseguenza: è un run INTERO a barriere. Il file 1 non entra nel rilevamento finché TUTTI i file non hanno finito l'indice (stessa onda); un'onda lenta ferma anche i rami indipendenti più corti. Non c'è nessuna dipendenza per elemento: un passo `per_item` a valle di un altro `per_item` aspetta l'intera istanza a monte.
- Determinismo: le uscite si raccolgono per indice (`gather`, `:198`; `settle` in ordine di file, `:349-367`), quindi il `prov/1` non dipende da `workers` (prova `cargo test -p mzlab-feat --test flow`). Un errore ferma solo i discendenti (`:334-337`, `Run::skipped`).
- Nessun annullamento, nessuna nozione di piano che cambia durante la corsa, nessun avanzamento dentro un lavoro: il `Runner` (`run.rs:52`) restituisce solo l'`Outcome` finale.

### 1.2 CLI e cache (`crates/mzlab-cli`)
- `flow run` (`flow_run.rs:339`) → `write_run` (`flow_out.rs:19`) → `run_checked` → `execute`; `--workers N` (`flow_run.rs:30`, default 1). `run_plan_notify` (`flow_out.rs:135`) passa un `DoneFn` chiamato **dopo** ogni lavoro (`flow.rs:130`, chiamata a `flow.rs:460-465`): id, item, ms, cached, errore. Niente "iniziato", niente %.
- Cache della cartella di corsa (`flow_cache.rs`): `Cache::key` (`:56-81`) = `flow::cache_key(SHA-256 degli ingressi, "step@ver istanza item versione commit", algoritmo, params_sha256)`. Due difetti per l'incrementale: (a) la chiave contiene l'**indice dell'item** (`job.item`, `:65`) e i file per elemento sono nominati per indice (`convert_file` = `{istanza}-{item}.mzML`, `flow_base.rs:260`): aggiungere un file che si ordina in mezzo rinumera e fa ricalcolare i successivi; (b) la chiave dipende dallo SHA delle uscite a monte, quindi si conosce solo DOPO che il monte ha finito: non serve per decidere in anticipo cosa è ancora valido.
- `FileRunner::keep` (`flow.rs:183`) legge l'intero file in memoria per lo SHA-256 (memoria = dimensione del file più grande).

### 1.3 App desktop
- `flow/run` (`crates/mzlab-desktop/src/flow_run.rs:41`) esegue tutto il piano in una chiamata bloccante (`run_plan_notify`, `:69`) con `workers = cores-1` limitato a 4 (`:66-67`); `flow/progress` (`flow_progress.rs`) dà la lista dei lavori finiti; la pagina la chiede ogni 0,7 s (`flow/lazy/flow-desktop.js:70`) e scrive «waiting / working, N files done / done in X s» (`:55-62`). Una sola corsa alla volta, nessun annullamento, il pulsante Run resta occupato.
- Esiste già una **coda di conversione in sottofondo** (`conv_coda.rs`): ogni file del produttore aperto in Dati diventa mzML indicizzato su `cores-1` thread, mai mentre Dati costruisce un indice (`:1-3`, `INDEXING`); `flow/run` prende l'mzML pronto invece di riconvertire (`flow_run.rs:56-64`). È il primo pezzo di R2 già fatto, ma è fuori dal grafo: la pagina Flow non ne mostra lo stato.

### 1.4 Sito («This browser»)
- `F.runners.web.run` (`flow/lazy/flow-web-run.js:149`) cammina i passi del piano **in ordine e in sequenza** (`:158-166`): `index.mzidx` è «reused» (l'indice l'ha già fatto Dati), `features.detect` lancia un lavoro per file nel pool di `feat-worker.js` (`detect`, `:84-101`, `Promise.all` sui file), `features.align` aspetta tutti i rilevamenti (`:108-134`), `gapfill` è «reused» dal pool; ogni altro passo è `skipped` con `via: computer` (`:164`). I passi MSn passano a `F.webMsn.run` (`:151`).
- Eventi `flow:web-step` `{passo, stato, mb, via, parte, di, modo}` (`:81`, `docs/agenti/flow-web.md`): «parte/di» conta i file finiti, nessun % dentro un file.
- Ripresa: ogni uscita in OPFS `mzlab/flow/<indice>.mzfeatf` con `.key` accanto = `dimensione:data dell'indice|ricetta` (`:92-95`); l'allineamento usa `series.mzfeat` con la chiave di tutti i file (`:110`). È già una cache per impronta, ma con chiavi scritte in JS e non quelle di Rust.
- Annullamento: solo `pool.cancel()` (`TP_Mine/files/feat-worker.js:321-325`), che uccide TUTTI i worker e rifiuta la coda; non si può annullare un solo lavoro.
- I file `.mzidx` vengono dall'indicizzazione di Dati (lettura nativa, `docs/agenti/lettura-nativa.md`; barra unica `docs/agenti/progresso.md`, evento `mzlab-index`): sul sito «trasformare i raw» = l'indice che Dati costruisce già al caricamento.
- Interfaccia: una modifica del piano emette `flow:changed` (`flow/lazy/flow-page.js:100`, ascoltato da `flow-shell.js:149`, `flow-web-ui.js:62`, `flow-desktop.js:97`) e ridisegna; le carte della tela (`flow-canvas.js`, 220×64) non hanno stato di corsa.

### 1.5 Riassunto
Oggi: run intero, a onde (CLI/desktop) o in sequenza (web), avanzamento a fine lavoro, nessun annullamento mirato, nessuna invalidazione: dopo una modifica si rilancia tutto e la cache salva i passi con chiave uguale. Mancano: dipendenze per file, piano mutabile durante la corsa, chiave "prima di eseguire", %, annullamento per lavoro.

## 2. Progetto

### 2.1 Idea
Uno **scheduler a stati, puro, in Rust** (`mzlab_core::flow::sched`), senza thread, file né orologio, come il resto di `flow`. Riceve il piano (e ogni sua nuova versione), sa quali lavori esistono, in che stato sono e quali sono pronti; gli esecutori (thread nativi o Worker del browser) gli chiedono lavoro, gli riportano avanzamento e fine. Lo stesso codice gira nativo (CLI, desktop) e in `.wasm` (sito). `flow::execute` diventa un ciclo sopra lo scheduler, così la CLI e la pagina hanno un solo motore.

### 2.2 Grafo dei lavori (per istanza e per elemento)
- **Lavoro** = `(istanza, elemento)`. Elemento = la chiave stabile del file che lo guida: il **nome dell'ingresso del piano** da cui discende (non l'indice). Un passo non `per_item` ha un solo lavoro (`elemento = None`).
- **Dipendenza per elemento**: se un passo `per_item` legge, sulla porta che lo guida, l'uscita `many` di un altro passo `per_item`, il lavoro `(B, f)` dipende solo da `(A, f)` («zip» per elemento). È ciò che dà R2: `convert(f1) → centroid(f1) → index(f1) → detect(f1)` scorre mentre `convert(f5)` gira.
- **Barriera**: un passo che legge tutta la porta `many` (non `per_item`: `features.align`, `features.table`, `formula.ms1`…) dipende da TUTTI gli elementi a monte. Parte solo quando l'ultimo è fatto; finché manca un file è «in attesa (k di n file pronti)».
- **Ordine delle uscite**: le uscite di una porta `many` si raccolgono SEMPRE nell'ordine degli ingressi del piano (l'ordine di oggi), mai in quello di fine. È il punto di R8.

### 2.3 Impronta del lavoro (chiave di piano)
- Nuova chiave calcolabile PRIMA di eseguire (albero di Merkle): `plan_key(lavoro) = SHA-256(step@versione, algoritmo@versione, params_sha256, versione del programma, chiavi dei lavori a monte nell'ordine delle porte, impronta dei file del piano)`. Impronta di un file del piano = `sha256` se il piano lo porta, altrimenti `nome:dimensione:mtime` (come la `.key` del web) o l'impronta che il lettore dà già (lettura nativa: impronte nel segnaposto).
- Usi: (a) decidere a ogni modifica cosa è ancora valido senza aspettare il monte; (b) chiave della cache (su disco nella cartella di lavoro; sul sito in OPFS `mzlab/flow/live/<plan_key>`); (c) annulla e ripeti: tornare a un valore già provato trova la sua uscita.
- La chiave di oggi (SHA delle uscite, `flow_cache.rs:56`) resta come **verifica** del file preso dalla cache (`take`, `:84-90`). Né la nuova né la vecchia contengono l'indice dell'item.
- **Ricetta del primo indice**: in HR la ricetta di `features.detect`/`align` parte dalla ricetta stimata del primo indice della serie (`run::recipe_of_layers`, `docs/agenti/rust-flow.md`). È una dipendenza nascosta: ogni rilevamento dipende dal primo file. Va resa esplicita nel grafo: il lavoro `detect(f)` dipende anche da `index(primo)`, e «primo» = il primo ingresso nell'ordine del piano (aggiungere file in coda non lo cambia). Decisione D2.

### 2.4 Stati
Per lavoro: `waiting` (ingressi non pronti) · `ready` · `running` (con `frac` 0-1 se l'esecutore la dà, altrimenti indeterminato) · `done` · `reused` (dalla cache) · `stale` (c'è un'uscita, ma di una chiave vecchia: la si mostra attenuata mentre si rifà) · `cancelled` · `error` (con `error_key`/`params`) · `refused` (memoria: `error.featMemory`) · `blocked` (manca un permesso: «Use more memory», o il passo gira solo su computer, `via: computer`) · `skipped` (un antenato è in errore) · `paused` (pronto, ma l'utente ha premuto «Pause»).
Per nodo (carta): aggregato calcolato in Rust: stato peggiore secondo l'ordine `error > refused > blocked > running > stale > ready > waiting > reused/done`, `frac` = media delle frazioni dei lavori (un `done` = 1), `parte/di`, `ms` = somma dei tempi dei lavori fatti in questa corsa, `elapsed_ms` del lavoro più vecchio in corso (calcolato dal chiamante: Rust non ha orologio). Mai un tempo rimanente (stessa regola del caricamento, `lettura-nativa.md`).

### 2.5 Modifica del piano durante la corsa
`sched.update(nuovo_piano)` (debounce di 300 ms in pagina su `flow:changed`):
1. ricontrolla il piano (`flow::check`); con problemi i lavori dei nodi toccati vanno `blocked` con il problema, il resto continua;
2. ricalcola le `plan_key`; un lavoro con chiave uguale resta com'è (anche `running`); uno con chiave diversa: se `running` → comando `cancel(job)` all'esecutore, se `done` → `stale`; poi `waiting`/`ready` con la nuova chiave (se la chiave nuova è in cache → `reused` subito);
3. i lavori scomparsi (nodo tolto, file tolto) → `cancel` se in corso, eliminati dalla vista; la loro uscita resta in cache (annulla la ritrova);
4. restituisce la lista di comandi (`cancel`, `start`) e gli eventi di cambio stato.
Un lavoro annullato che l'esecutore non riesce a fermare (un passo lungo dentro il `.wasm`) finisce comunque: lo scheduler scarta il risultato se la chiave non è più attesa, ma lo mette in cache (è un risultato vero di quella chiave).

### 2.6 Scelta del lavoro e contropressione (R9)
- `next(limiti) -> Vec<JobSpec>` dà i lavori `ready` in un **ordine deterministico di priorità**: prima i lavori più a valle di un elemento già avviato (profondità decrescente: si finisce il file 1 lungo la catena prima di aprire il file 6), poi l'ordine degli ingressi del piano, poi l'ordine del piano. Così i risultati intermedi in memoria/OPFS restano pochi e il primo risultato utile arriva presto.
- Limiti passati dall'esecutore: `slots` (thread o Worker liberi), `mem_mb` libero; ogni lavoro ha una stima `mb` (stima del passo se c'è, `stima_series` per l'allineamento; altrimenti la classe del descrittore) e non parte se non ci sta. Un lavoro oltre il budget ma entro il livello alto va `blocked` con la domanda «Use more memory» (mai una finestra a sorpresa mentre si modifica il grafo).
- **Risorse esclusive** («hot file»): un lavoro dichiara le risorse che tiene (`dataset:<percorso>` per un `.wiff` condiviso fra iniezioni o una cartella `.D`; `opfs:<file>` per un SyncAccessHandle; il Web Lock `mzlab-mem-grande` per i passi > 512 MB); due lavori con la stessa risorsa esclusiva non partono insieme. Sul desktop la coda di conversione continua a cedere i core all'indice di Dati (`conv_coda.rs`, `INDEXING`).

### 2.7 Eventi `flowlive/1` (schema versionato)
Bozza in `flowlive-1.draft.json`; la versione finale va in `docs/schemi/flowlive-1.json` con la PR L1. Un evento per cambio di stato di un lavoro e uno per nodo, più gli eventi di avanzamento (al più uno ogni 250 ms per lavoro, filtrati dall'esecutore):
```json
{"format":"flowlive/1","seq":42,"rev":7,"kind":"job","instance":"detect","item":"S01.raw","state":"running","frac":0.4,"ms":null,"mb":180,"via":"browser","key":"9f2c…","error":null}
{"format":"flowlive/1","seq":43,"rev":7,"kind":"node","instance":"detect","state":"running","frac":0.55,"parte":3,"di":6,"ms":5120,"via":"browser"}
```
`seq` cresce sempre; `rev` = numero di versione del piano a cui l'evento si riferisce (la pagina scarta un evento di una revisione vecchia). Gli eventi `flow:web-step` di oggi si derivano da questi (compatibilità per `e2e_flow_web.py` finché esiste).

### 2.8 Cosa sta in Rust e cosa in JS
- **Rust** (`mzlab-core/src/flow/sched.rs`, nuovo, puro): grafo dei lavori, `plan_key`, stati, `update`, `next`, aggregati di nodo, eventi JSON. `flow::execute` riscritto sopra di esso (stessi byte del `prov/1`). Esecutore nativo con thread e annullamento cooperativo in `mzlab-cli` (`flow_live.rs`, nuovo; `std::thread`, nessuna dipendenza nuova; Rayon resta solo nel desktop dove già c'è). Rotte desktop in un file nuovo `crates/mzlab-desktop/src/flow_live.rs`. Avanzamento dentro i lavori (callback `step(done,total)` già usata dai lettori, `nativi.rs`) nei passi lunghi.
- **`.wasm`**: lo scheduler si esporta dal modulo `hr` già spedito (`crates/mzlab-feat-wasm`, `sched.rs` nuovo dietro la feature `detect`, che `hr` ha già): nessuna feature Cargo nuova, nessun `.wasm` nuovo; ospite = `worker-flow-web.js`, che carica già il `feat` per la stima. Margine misurato del tetto `limit_hr`: 1 212 416 − 1 099 407 = 113 009 B; lo scheduler (senza serde, JSON di `mzlab_core::json`) deve stare sotto ~40 KB. Se la misura supera il tetto la PR si ferma e chiede a Federico (D4).
- **JS** (solo interfaccia e trasporto): `flow/lazy/flow-live.js` (nuovo, in `LATE` di `flow-tool.js`): manda il piano al worker a ogni `flow:changed`, riceve i comandi `start/cancel` e li passa agli esecutori del sito (pool di `feat-worker.js`, `F.webMsn`, l'indice di Dati che si osserva e non si rifà), rimanda avanzamento e fine; scrive lo stato sulle carte. `feat-worker.js` guadagna l'annullamento di UN lavoro (`cancelJob(id)`: uccide solo il worker che lo esegue). Nessun calcolo in JS: frazioni, aggregati, chiavi e scelta dei lavori vengono dal worker.
- **Desktop/CLI**: rotte `flow/live/start|update|events|pause|cancel` (polling ogni 300-700 ms come `flow/progress`, eventi dal `seq` dato); `mzlab-cli flow run --jsonl` stampa gli eventi `flowlive/1` (stesso schema). La coda di conversione diventa il passo `convert.mzml`/`import.raw` del grafo: i suoi elementi pronti sono lavori `done`.

### 2.9 Interfaccia
- **Live** acceso per difetto nella scheda Flow (amministratore); «Pause» ferma l'avvio di nuovi lavori (quelli in corso finiscono), «Resume» riprende; «Run» diventa «Run now» solo dove un passo è `blocked` per permesso. Il piano con problemi non blocca i rami sani.
- Carta (220×64): riga di stato con icona + parola (mai solo colore: «waiting», «3 of 6 files», «42 %», «done in 12 s», «out of date», «error: …»), barra sottile `role="progressbar"` con `aria-valuenow` (assente se indeterminata), contorno tratteggiato per `stale`. Colori solo da variabili CSS (Okabe-Ito), `prefers-reduced-motion` spegne strisce e animazioni. `#flx-live` (polite) annuncia solo i cambi di stato dei nodi, al più uno ogni 2 s.
- Pannello del passo: tabella «Files» (file, stato, %, tempo, memoria) = la tabella dei dati della vista (non è un grafico, ma vale la regola TDATI di leggibilità).
- Avvio rapido (R6): niente di tutto ciò nel primo disegno: `flow-live.js` è `LATE`; lo scheduler parte solo dopo `whenLate` e dopo il primo disegno della tela; il worker si avvia a riposo.

## 3. Rischi
1. **Determinismo** (alto). Ordine di fine ≠ ordine delle uscite; stime per lavoro (`estimates`) che cambiano i parametri registrati (`settle`, `run.rs:279-287`); ricetta dal primo indice. Rimedi: raccolta per ordine del piano; test con un `Runner` che finisce i lavori in ordine casuale con seme (10 permutazioni) e `prov/1` identico a `execute` di oggi salvo `ms`/`heap_peak_bytes`; `tests/test_flow_determinismo.py` con le stesse costanti (o costanti nuove dichiarate nella PR se cambia il nome dei file per item, D3).
2. **Nomi dei file per indice** (medio). `convert_file` = `{istanza}-{item}.mzML` (`flow_base.rs:260`) e la chiave della cache con `job.item`: aggiungere un file rinumera. Rimedio: nome per elemento (radice del nome dell'ingresso, resa unica), chiave senza indice; cambia i byte del `prov/1` (etichette): costanti dei test da rigenerare in quella PR, esplicitamente.
3. **Tetti `.wasm`** (medio). Lo scheduler nel modulo `hr` consuma margine (113 KB). Rimedio: niente serde, niente regex, misura nella PR (`lib.wasm_cap`); oltre il tetto ci si ferma (D4).
4. **Annullamento nel `.wasm`** (medio). Un lavoro dentro una chiamata wasm non si interrompe: si uccide il worker (memoria liberata, ripartenza di decine di ms) o si lascia finire. Rimedio: `cancelJob` uccide solo quel worker; i lavori corti si lasciano finire e il risultato va in cache.
5. **Hot file serializzati** (medio). `.wiff` condiviso fra iniezioni, cartelle `.D`, OPFS con handle esclusivi, lock `mzlab-mem-grande`: senza risorse dichiarate due lavori si pestano o si serializzano a caso. Rimedio: risorse esclusive nello scheduler (§2.6), test con due iniezioni dello stesso `.wiff`.
6. **Memoria** (medio). Più lavori insieme = somma dei picchi; sul web la memoria di un `.wasm` non scende mai. Rimedio: ammissione per `mb` stimato contro il budget, pool già limitato a `cores-1 ≤ 4` (`feat-worker.js:12`), chiusura dei worker cresciuti oltre 512 MB come oggi (`flow-web-run.js:132`). `FileRunner::keep` legge interi i file per lo SHA: passare a SHA a pezzi (stessa impronta) nella PR L3.
7. **Lavoro sprecato mentre si modifica** (basso). Un parametro trascinato produce dieci revisioni. Rimedio: debounce 300 ms, annullamento dei soli lavori a valle, cache per `plan_key` (tornare indietro è gratis).
8. **Concorrenza con Dati** (basso). L'indicizzazione di Dati e i lavori del Flow competono per i core. Rimedio: sul desktop `INDEXING` già cede; sul web il nodo `index.mzidx` osserva Dati e non costruisce mai un secondo indice.
9. **Test fragili** (medio). Niente pause fisse né millisecondi: gli e2e contano eventi e stati; la parità Rust↔wasm dello scheduler su un file di casi condiviso (`crates/mzlab-core/tests/data/sched_cases.json`), come `flow_check_cases.json`.
10. **Conflitti con le altre chat Flow** (medio). Flow-MSn tocca `flow-web-run.js`/`flow-web-msn.js`, Flow-Pipeline la ricetta e il rilevamento, Flow-Avvio `flow-tool.js` (CORE/LATE). Rimedio: ordine del piano §4, una PR alla volta su `flow.rs`/`lib.rs`/`run.rs`.

## 4. Piano in PR
Ordine e dipendenze (prompt completi in `PROMPTS.md`):

| PR | Contenuto | Dopo |
|---|---|---|
| L0 | questo studio (solo docs) | — |
| L1 | `mzlab_core::flow::sched` puro: lavori per elemento, `plan_key`, stati, `update`, `next` con priorità e risorse, aggregati, eventi `flowlive/1`, schema; casi condivisi; nessun chiamante | L0 |
| L2 | `flow::execute` sopra lo scheduler (stesso `prov/1`, test a ordine casuale); cache e nomi per elemento senza indice | L1 |
| L3 | esecutore nativo live (`mzlab-cli/src/flow_live.rs`: thread, annullamento cooperativo, `--jsonl` `flowlive/1`) e rotte desktop `flow/live/*`, coda di conversione dentro il grafo | L2 |
| L4 | scheduler nel `.wasm` `hr` (`mzlab-feat-wasm/src/sched.rs`) ospitato da `worker-flow-web.js`, parità sui casi condivisi, misura del tetto | L1 |
| L5 | `flow/lazy/flow-live.js` sul sito: piano → worker, comandi → pool / `F.webMsn` / indice di Dati; `cancelJob` in `feat-worker.js`; «This browser» diventa live | L4 (e L2 per le chiavi) |
| L6 | carte e pannello: stato, %, tempo, «out of date», Pause/Resume, annunci; desktop sullo stesso modello (`flow-desktop.js` su `flow/live/*`) | L5, L3 |
| L7 | avanzamento dentro i lavori lunghi (`step(done,total)` in indice, rilevamento, allineamento; `frac` negli eventi) | L1 |

Ogni PR: ramo `claude/flow-live-<n>` da `origin/main`, PR aperta mai unita dalla chat DEV, `tools/fondi_main.py` e `tools/pre_pr.py` completo dopo l'ultima fusione di main, merge a CI verde sul SHA esatto (i sei controlli richiesti) dopo un fold di main.

## 5. Decisioni per Federico
- **D1** Live acceso per difetto nella scheda Flow (proposta: sì, con Pause; i passi oltre il budget aspettano il permesso invece di chiederlo a metà modifica).
- **D2** «Primo indice» della ricetta HR = primo ingresso nell'ordine del piano (proposta), non il primo in ordine alfabetico: aggiungere file in coda non invalida i rilevamenti fatti.
- **D3** I file per elemento si chiamano col nome dell'ingresso invece che con l'indice: cambiano le etichette nel `prov/1` e le costanti di `test_flow_determinismo.py` una volta (proposta: sì, in L2).
- **D4** Lo scheduler va nel modulo `hr` senza alzare il tetto; se non ci sta, si chiede (nessuna proposta di nuovo `.wasm`).
- **D5** Sul web le carte mostrano il % dentro un file solo dopo L7; prima mostrano «k of n files» e il tempo trascorso (proposta: va bene così per L5-L6).
