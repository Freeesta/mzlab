# Memoria elastica del motore nel browser (studio)

Problema: 12 file `.raw` Orbitrap (≈4,2 milioni di picchi e ≈7950 scansioni ciascuno) fermano il Dig di Tabui in «Alignment and filters» con `error.memoryBudget {fase: align, stima_mb: 188, budget_mb: 192, eccesso_mb: 1}`. Questo studio dice dove va la memoria, perché la stima è 188 MB, come rendere il budget adattivo e in che ordine lavorare. Nessun dato reale: i numeri vengono dal codice, dalle guardie e da conti algebrici.

## 0. Sintesi
- La causa principale NON è il tetto: il Dig usa il modo «famiglia» (`Recipe::default`, `mode: Mode::Family`; `feat-worker.js` manda solo `{polarity, mode}`) e in `slabs::align_and_fill_stored` il modo famiglia è SEMPRE in memoria: le lastre esistono solo per il modo «ione». Rendere il modo famiglia divisibile in lastre (pacchetto M1) fa entrare la serie anche a 192 MB, con gli stessi byte.
- Seconda causa: il Dig non riceve il budget adattivo: `feat-worker.js` usa `opts.budgetMb || 0` e nessun chiamante lo imposta, quindi ogni passo di Tabui gira a 192 MB anche sul computer che ne avrebbe di più.
- Terza (da confermare con le misure del Mac): ≈145 000 feature per file sono tante per un Orbitrap; la soglia del Dig è fissa (`MIN_HEIGHT` 2e5) e la stima del rumore di `params.rs` non entra nella ricetta. Cambiarla cambia i risultati: decide Federico (pacchetto M7).
- Il messaggio inganna: `eccesso_mb` è calcolato contro il tetto (192) e non contro lo spazio libero (≈177 MB), così «stima 188, budget 192» sembra entrare. Si corregge in M2.
- Dopo M1 si alza il tetto (4 GiB) con un budget a livelli scelto in Rust dai fatti del dispositivo, una prova di crescita all'avvio, un solo worker «grande» alla volta, avviso prima e discesa a gradini dopo una morte.

## 1. Dove va la memoria oggi
Le guardie (`tests/test_guardie_memoria.py`, `tools/guardie_memoria.json`) sulle serie sintetiche restano al pavimento di 32 MB (memoria iniziale) in ogni fase; solo `completo` alle prove sale a 45 154 304 B (43 MB) e a 24 MB va a lastre restando a 32 MB. Quindi le guardie provano l'assenza di regressioni, ma non dicono come cresce la memoria con serie vere: per quello valgono i pesi per riga delle stime, che sono la base delle decisioni del motore.

| Fase | Peso stimato (codice) | Via oltre il budget |
|---|---|---|
| Indice `.mzidx` (per file, worker dell'indice) | tabella 640 B/scansione + indice 220 B/scansione + al massimo `budget/4` di picchi (48 MB) + 8 MB di partizioni | secchi da OPFS, stessi byte |
| Detect L1-L3 (per file, pool di `feat-worker.js`) | picchi letti a finestre dall'indice; tabella del file 91 B/riga | per file: entra |
| Align L4 «ione» | `s.all` 8 B/riga + 39 (lean) + 40 (lavoro) B/riga + tabella 21 B/cella + 28 B/riga | lastre di m/z, stessi byte |
| Align L4 «famiglia» (predefinito del Dig) | come «ione» | nessuna: in memoria o rifiuto `align` |
| Strato debole L4b (`series::finish`) | 29 B/riga di tutte le tabelle deboli insieme, rifiuto oltre metà del libero | nessuna |
| Riempimento L6, `rowmeta` | un file alla volta, tabella intera | entra se entra la tabella |
| Ipotesi L7 (`target::run_loading`) | una tabella di file alla volta | entra |
| Prove L8-L9 (`run::evidence_to`) | `righe × 2400 B` (`IDENTITY_BYTES`) + tabella | `GraphStream`: due passate, stessi byte |
| Motore Dati (`mzlab-wasm`) | cache LRU 5/6 del budget (160 MB) + lavoro 1/6 (32 MB) | espulsione LRU |

Memoria della scheda: il motore Dati, fino a `min(core−1, 4)` worker di `feat-worker.js`, fino a 4 worker dell'indice (cancello comune) e l'mzML di una conversione in un Blob. Ogni istanza può crescere fino al suo tetto e la memoria WASM non si restringe: solo chiudere il worker la rende.

## 2. Perché align a 12 file stima 188 MB
- Conto: nel modo famiglia `whole = righe × 79 + tabella`, con tabella = `più_larga × 1,5 × (n × 21 + 28)`. Prima si riservano `righe × 8` per `s.all`, quindi lo spazio libero è `192 MB − 8 × righe`. Il rifiuto arriva quando `whole` supera il libero.
- Con 12 file simili (più larga ≈ righe/12) `whole ≈ righe × 114 B`: 188 MB corrispondono a ≈1,7 milioni di righe, cioè ≈145 000 feature per file; il libero è ≈178 MB, da cui il rifiuto anche se 188 < 192.
- Costo teorico delle sole righe lean: 39 B × 1,7 M ≈ 63 MB. La stima è 3× quel valore: 40 B/riga di lavoro (copie ordinate, logaritmi) e ≈60 MB di tabella stimata. Non è uno spreco della stima: `align_grouped` passa due volte su tutti i file e `link` tiene `group_of` per ogni riga.
- Inefficienza strutturale: il modo famiglia ha bisogno di tutto solo per `famalign::link`, che per ogni riga usa `group_of`, `role`, `family_id`, `charge`, `rt`. I gruppi di ioni si formano per m/z: si possono fare a lastre come nel modo ione, tenendo solo la tabella allineata e `group_of` (4 B/riga), e poi contare voti e legami leggendo un file alla volta. Peso a lastre: tabella + 4 B/riga + una lastra + un file, ≈1/4 di oggi (M1).
- Strato debole: se align passa, `finish` tiene insieme le tabelle deboli di tutti i file (29 B/riga, soglia `WEAK_RATIO` 0,3 × 2e5): con molte feature deboli è il prossimo rifiuto `align`. Si sposta fuori dalla memoria lineare e si allinea a lastre (M1, seconda parte).
- Feature di rumore: 145 000 feature per file su ≈530 picchi per scansione indicano che la soglia assoluta 2e5 sta vicino al rumore di quello strumento. La misura del Mac deve dire, per file: righe del `.mzfeatf`, righe con altezza < 3 × rumore stimato (`params::estimate`), righe deboli. Se gran parte è rumore, la soglia proposta da `params.rs` riduce la stima più di qualunque tetto, ma è una scelta scientifica (M7).

## 3. Contratto di memoria v2

### 3.1 Due numeri per istanza
- `sicuro_mb`: budget con cui il motore lavora senza chiedere nulla. `alto_mb`: limite che si può usare solo dopo l'avviso. Tetto di costruzione 4 GiB (`--max-memory=4294967296`), che non riserva memoria: dice solo fin dove `memory.grow` può arrivare.
- Livelli: 96, 128, 192, 512, 1024, 2048, 3072 MB. Mai sopra 3072 nel browser (margine per codice, pila e frammentazione sotto i 4 GiB di wasm32; `usize` è a 32 bit).

| Dispositivo (fatti) | sicuro | alto |
|---|---|---|
| `deviceMemory` ≤ 1 | 96 | 96 |
| `deviceMemory` 2 | 128 | 192 |
| telefono/tablet (`telefono.js`) o Safari su iOS/iPadOS | 192 | 192 |
| computer, `deviceMemory` 4 | 512 | 1024 |
| computer, `deviceMemory` 8 (= 8 GB o più), core < 8 | 1024 | 3072 |
| computer, `deviceMemory` 8, core ≥ 8 | 2048 | 3072 |
| computer, `deviceMemory` sconosciuto (Firefox, Safari) | 512 | 2048 |

- Poi `alto = min(alto, prova)` e `sicuro = min(sicuro, alto)`, dove `prova` è il massimo concesso dalla prova di crescita (3.2). Affidabilità come oggi: `high` se il numero viene dal dispositivo, `medium` se è un predefinito.
- Discesa: una morte per memoria (firma del supervisore) porta `sicuro` al livello sotto quello in uso e `alto = sicuro`; la seconda morte entro un minuto porta a 192; la terza ferma con `error.memoryBudget`. La discesa vale per la sessione (`sessionStorage`), si scrive nel registro `mzlab.memBudget.log` con chi = supervisore.

### 3.2 Prova di avvio
- Cosa: un worker usa e getta (`rust-sonda.js`, modulo ES) crea `new WebAssembly.Memory({initial: 1, maximum: 65536})` e chiama `grow(4096)` (256 MB) a gradini fino a 3072 MB o al primo errore; poi il worker si chiude. Non scrive le pagine: misura il limite del browser, non la RAM libera (quella viene da `deviceMemory`).
- Perché un'istanza a parte: la memoria WASM non si restringe; una prova dentro il motore gli lascerebbe gigabyte.
- Costo: atteso < 20 ms sul computer (pagine non toccate); tetto 2 s, oltre si registra `prova: null` e vale la tabella. Si fa una volta per sessione, a pagina ferma (`requestIdleCallback`), prima della prima richiesta grande; risultato in `sessionStorage` `mzlab.memProbe` e nel registro (`inputs.probe_mb`, `inputs.probe_ms`).
- Altri fatti: `navigator.deviceMemory`, `hardwareConcurrency`, `telefono.js`, `performance.memory.jsHeapSizeLimit` se c'è (solo registro, non decide: su Chromium desktop vale ≈4 GB ovunque). `measureUserAgentSpecificMemory` richiede l'isolamento cross-origin, che GitHub Pages non dà: escluso.

### 3.3 Chi decide
- Rust decide (`mzlab-core::budget_auto`, esportato come oggi da `budget_choice`): riceve i fatti `{device_gb, cores, small, ios, probe_mb}` e restituisce `{sicuro_mb, alto_mb, livelli, reason, confidence, inputs}`. Anche la discesa (`budget_down(attuale)`) è in Rust. Prova Rust: tabella dei casi.
- JS raccoglie soltanto i fatti, li passa e scrive il registro; non contiene soglie in MB (oggi `rust-bridge.js` ha 192 e 96 scritti a mano: diventano campi della scelta).
- Ogni fase continua a stimare prima di allocare: il budget più grande cambia solo la via (memoria o lastre), mai i byte del risultato.

### 3.4 Più worker, una scheda
- Un solo «permesso grande» per scheda e per origine: un worker usa più di 192 MB solo dopo aver preso il Web Lock `mzlab-mem-grande` (esclusivo, vale anche fra schede); gli altri lavorano a 192. Lo prendono le fasi lunghe di Tabui (align, prove) e, in seguito, il motore Dati per una mappa grande.
- Chi ha usato il permesso chiude il proprio worker a fine fase se la sua memoria supera 512 MB (la memoria non torna indietro); il pool lo ricrea alla richiesta successiva.
- Motore Dati: la cache LRU resta a `min(5/6 × budget, 512 MB)`; il resto del budget è lavoro. Il riciclo di `rust-bridge.js` scatta quando `mem` supera `sicuro_mb` (non più 192 fisso) e il motore è fermo da 30 s.
- Somma della scheda: al massimo `alto` + 192 × (worker attivi − 1) + heap JS; i pool restano a `poolSize` di oggi.

### 3.5 OPFS e heap JS come appoggio
- I dati intermedi grandi che una fase rilegge per intero (tabelle lean per lastra, tabelle deboli) non stanno nella memoria lineare: passano per un tratto `Spill` (scrivi un blocco, rileggi un intervallo di m/z) che nel worker è prima un array JS (fuori dai 256 MB, M1) e poi un file OPFS con `createSyncAccessHandle` (M5).
- Già in OPFS: sorgenti, `.mzidx`, scratch dell'indice, `.mzfeatf`. Oggi però `feat-worker.js` copia tutti i `.mzfeatf` della serie nell'heap del worker (`copy(stored)`) e `Stored::bytes(k)` copia un file intero nella memoria WASM per ogni lastra; M5 li legge a intervalli dall'OPFS.
- Non conviene lo spill della tabella allineata (letta a caso da `finish` e `rowmeta`) né della cache del motore (già rifatta dall'indice).

### 3.6 Browser
- Chromium desktop: 4 GiB per istanza da Chrome 83; `deviceMemory` si ferma a 8. Chromium Android: `memory.grow` fallisce spesso fra 1 e 2 GB; la prova lo misura e comunque è «telefono».
- Firefox desktop: 4 GiB; niente `deviceMemory` né `performance.memory` (livello «sconosciuto»).
- Safari macOS e iOS: da provare che un modulo con massimo 4 GiB si istanzi (il commento attuale di `.cargo/config.toml` parla di 256 MB come limite di iPadOS). Prova in M2: Federico apre il sito sull'iPad e su Safari macOS e guarda «Diagnostica». Se iOS rifiuta il modulo, ripiego già pronto da provare nello stesso pacchetto: `--import-memory`, con la `WebAssembly.Memory` creata dalla pagina con il massimo scelto (stesso `.wasm` per tutti).
- iOS chiude le schede oltre ≈1-1,5 GB: resta 192.

## 4. Esperienza d'uso
- Predefinito sicuro: nessuna domanda finché la stima entra in `sicuro`. Le lastre sono silenziose (stessi byte); si vede solo l'avanzamento.
- Avviso (prima di partire, una sola volta per serie): quando la stima della fase supera `sicuro` ma entra in `alto`. Due pulsanti, Invio = il primo: «Lavora a parti (più lento)» e «Usa più memoria». Se la fase non ha lastre il primo pulsante è «Annulla». La scelta vale per la sessione.
- Testo (Tabui, solo inglese): «This step needs about 1.4 GB of memory; this browser safely gives 1 GB. If the tab closes, finished steps are kept.» Pulsanti «Work in parts (slower)», «Use more memory», «Cancel».
- Dati (`lang/`, it/en): «Questa fase richiede circa 1,4 GB di memoria; il browser ne garantisce 1 GB. Se la scheda si chiude, le fasi finite restano salvate.» / «Lavora a parti (più lento)», «Usa più memoria», «Annulla».
- Dopo una morte: barra in alto del pannello, non una finestra: «The engine ran out of memory during Alignment. Finished steps are saved; it restarts with less memory.» + pulsante «Resume». In italiano «Il motore ha finito la memoria durante l'allineamento. Le fasi finite sono salvate; riparte con meno memoria.» + «Riprendi».
- Rifiuto (anche a lastre non entra): il testo di `error.memoryBudgetFase` con i numeri corretti (`eccesso` contro il libero) più un consiglio: «Try fewer files at once.» / «Prova con meno file insieme.».
- Avanzamento a lastre: «Alignment, part 3 of 9» nella barra del passo (Tabui) e la fase con «parte N di M» nella barra di `progresso.js` (Dati).

## 5. Pacchetti DEV (in serie)
Ordine: M1 e M2 in parallelo (file diversi); poi M3; poi M4; M5 dopo M4; M6 e M7 indipendenti. Ogni PR misura i `.wasm` prima e dopo (`tools/build_feat_wasm.sh`, `limit*`) e riporta i numeri: finder ≤ 512 000 B, gruppo HR 1 118 979 su 1 212 416, table 447 688 su 512 000, libro 152 008 su 159 744, motore principale ≈231 B di margine (un aumento sopra il tetto chiede l'assenso di Federico nella PR, mai un tetto alzato in silenzio).

### 5.1 M1 · Align «famiglia» e strato debole a lastre (nuova DEV, Rust)
- Obiettivo: il modo famiglia (`famalign::align_families`) e lo strato debole (`series::finish`) seguono la regola di `slabs.rs`: in memoria se entra in metà del libero, altrimenti a lastre di m/z, con gli stessi byte del `.mzfeat`.
- Perimetro: `crates/mzlab-feat/src/{slabs,famalign,align,series,store_read}.rs`, `crates/mzlab-feat/tests/slabs.rs`, `TP_Mine/files/feat-worker.js` solo per il callback `Spill`, `docs/agenti/memoria.md` (riga «slabs» e «Strato debole») e `docs/agenti/mzfeat.md` se cita le vie.
- Disegno: prima passata `align_grouped` senza mappa a lastre (cuts di oggi) → tabella intera → `RtMap`; seconda passata con mappa a lastre, `group_of[k]` (u32 per riga, indice globale del file = partizione + locale); `collect` legge un file alla volta solo `role`, `family_id`, `charge`, `rt` (nuovo `Want`); poi `link` e `rows` come oggi.
- Strato debole: ogni tabella debole va in un `Spill` (tratto nuovo: `put(k, bytes)`, `get(k, range_mz)`); in WASM è un callback verso array JS del worker; poi allineamento a lastre come il modo ione. Nativo e test: `Vec<Vec<u8>>`.
- Vincoli: nessun file della DEV «memoria wasm» (`mzlab-core/src/budget*.rs`, `cache.rs`, `.cargo/config.toml`, `rust-*.js`); stima prima di allocare; niente soglie cambiate; tetto di `finder`/`table` misurato.
- Prove: `cargo test -p mzlab-feat --test slabs` con casi famiglia e debole a 4/8/24 MB e in memoria: stesso SHA-256; `python3 -m pytest tests/test_feat_wasm.py tests/test_guardie_memoria.py -q`; `python3 tools/verifica.py --cambiati`.
- Accettazione: (a) serie sintetica in modo famiglia che oggi a 24 MB è rifiutata in `align` completa a 24 MB con lo stesso SHA-256 del `.mzfeat` in memoria (nativo e Node); (b) guardie entro +10 % (se cambiano, `MZLAB_GUARDIE_CREA=1` motivato); (c) conto nella PR: peso stimato famiglia per 1,7 M righe e 12 file, prima ≈188 MB, dopo (atteso ≤ 60 MB).
- Consegna: PR `claude/align-famiglia-lastre`, messaggio al Coordinatore con SHA, CI, pages e i numeri di (c).

### 5.2 M2 · Tetto 4 GiB e budget a livelli in Rust (riprende la DEV «memoria wasm», session_019kBc5HdjLDihEQX3unwZHq)
- Obiettivo: `.cargo/config.toml` a `--max-memory=4294967296`; `budget_auto` restituisce `sicuro_mb`, `alto_mb`, livelli e `budget_down` (tabella 3.1); `refusal` calcola `eccesso_mb` contro lo spazio libero e aggiunge `libero_mb`.
- Perimetro: `.cargo/config.toml`, `crates/mzlab-core/src/{budget,budget_auto,stima,lettori,mzidx}.rs`, `crates/mzlab-wasm/src/{cache,budget_js}.rs` (cache = `min(5/6, 512 MB)`), `crates/mzlab-desktop/src/ram.rs` solo se cambia la firma; i suoi file già toccati.
- Vincoli: aritmetica delle stime con `checked_mul`/`saturating_mul` (usize a 32 bit: oltre 2 GiB `n × size` trabocca), ogni `Vec` grande con `try_reserve`; i byte dei risultati non cambiano; il motore principale ha ≈231 B di margine: la tabella dei livelli come dati costanti, misura prima/dopo.
- Prove: `cargo test -p mzlab-core budget`, tabella dei casi 3.1 (incluse le combinazioni con `probe_mb`), test di trabocco (stima con 600 M righe → rifiuto ordinato, non panic); `tests/test_guardie_memoria.py`; build dei `.wasm` con `limit*`.
- Accettazione: Chromium del container istanzia ogni `.wasm` con massimo 4 GiB; parità dei byte a 24 MB e in memoria invariata (`test_feat_wasm.py`); `eccesso_mb ≥ 1` e `stima ≤ budget` non compaiono più insieme senza `libero_mb`.
- Prova Safari: chiede a Federico (tramite il Coordinatore) di aprire il sito dalla PR su iPad e su Safari macOS; se il modulo non si istanzia, nello stesso pacchetto prova `--import-memory` e lo propone.
- Consegna: PR, SHA, misure dei `.wasm` prima/dopo, esito Safari.

### 5.3 M3 · Prova di avvio, fatti e supervisore (DEV «memoria wasm», dopo M2)
- Obiettivo: `rust-sonda.js` (modulo ES, `// @ts-check`) come in 3.2; `rust-budget.js` raccoglie i fatti e chiama `budget_choice`; `rust-supervisor.js` e `rust-bridge.js` usano `budget_down` e `sicuro_mb` al posto di 96 e 192 scritti a mano; registro con la prova.
- Perimetro: `mzlab/web/{rust-sonda,rust-budget,rust-supervisor,rust-bridge,rust-worker}.js`, `tools/tipi/` (progetto e 0 errori), `index.html` o `carica.js` per il caricamento, `lang/it.js`/`lang/en.js` per il messaggio dopo la morte (4), `docs/agenti/memoria.md` (righe del budget adattivo, supervisore, riciclo).
- Vincoli: JS senza numeri di soglia; prova fuori dal motore; nessun avviso se la prova fallisce (vale la tabella); i test e2e vogliono `MZLAB_LR_MISS` vuoto.
- Prove: e2e nuovo `tests_e2e/e2e_memoria_sonda.py` (registro con `probe_mb`, tempo < 200 ms in CI); e2e del supervisore con morte simulata due volte: livelli 1024 → 512 → 192, fasi salvate; `python3 tools/controlla_i18n.py`; `tsc` del progetto; `python3 tools/verifica.py --cambiati`.
- Accettazione: in Chromium del container la scelta automatica dà `sicuro`/`alto` coerenti con la tabella (`deviceMemory` forzato con `addInitScript` a 2, 4, 8); dopo una morte simulata il registro ha la voce `supervisor` e il livello sceso.
- Consegna: PR, SHA, CI, pages.

### 5.4 M4 · Tabui usa il budget: permesso grande, avviso, avanzamento (dopo M1 e M3)
- Obiettivo: il pool di `feat-worker.js` riceve `budgetMb = sicuro_mb` (oggi 0 = 192); align e prove chiedono prima la stima (esportazione nuova e leggera `align_estimate`/`evidence_estimate` dal sondaggio dei `.mzfeatf`) e, se supera `sicuro` ed entra in `alto`, mostrano l'avviso (4); con «Usa più memoria» prendono il Web Lock `mzlab-mem-grande` e lavorano ad `alto`; il worker oltre 512 MB si chiude a fine fase; «parte N di M» nella barra del passo.
- Perimetro: `TP_Mine/files/feat-worker.js`, `TP_Mine/js/24_tpmine_diretta.js` e il pannello Pipeline (`TP_Mine/js/32_pipeline.js`), `crates/mzlab-feat-wasm/src/{lib,run}.rs` per la stima e il callback di avanzamento; `docs/agenti/mzfeat-wasm.md`, `docs/agenti/mzfinder.md`.
- Vincoli: Tabui solo in inglese (nessuna chiave `lang/`, guardie `test_tabui_inglese.py`); un solo avviso per serie; Invio = predefinito; tetti dei `.wasm` del gruppo HR e table misurati.
- Prove: serie sintetica che con budget 24 MB è rifiutata in una fase senza lastre e con 64 MB completa, stesso SHA-256 del nativo (scala del caso 192 → 512, in CI); nella notte (`notte.yml`) la stessa serie a scala vera 192 → 512; e2e di Tabui con l'avviso (scelta «Work in parts» e «Use more memory»: stessi byte); `e2e_axe_grafici` e il tour invariati.
- Accettazione: il Dig sulla serie sintetica grande non chiede nulla se entra in `sicuro`; mostra l'avviso una volta se no; due worker non tengono insieme il permesso (prova con due schede).
- Consegna: PR, SHA, CI, pages.

### 5.5 M5 · Appoggio in OPFS per la serie (dopo M4)
- Obiettivo: `Stored` e `Spill` del worker leggono e scrivono file OPFS con `createSyncAccessHandle` a intervalli, invece di copiare tutti i `.mzfeatf` nell'heap del worker e un file intero nella memoria WASM a ogni lastra.
- Perimetro: `TP_Mine/files/feat-worker.js`, `crates/mzlab-feat/src/{slabs,store_read}.rs` (lettura per intervallo di m/z dall'offset), `archivio.js` solo per i nomi in `feat/` e la pulizia dei `.part`; `docs/agenti/memoria.md`, `docs/agenti/dati-esporta.md` se tocca `export/`.
- Prove: stessi SHA-256 di M1/M4; misura con `tests_e2e/e2e_memoria_misure.py` (heap del worker prima/dopo); `ensureRoom` prima di scrivere oltre 50 MB.
- Accettazione: heap JS del worker durante align ≤ una lastra + un file; nessuna regressione di tempo oltre +30 % sulla serie `completo`.

### 5.6 M6 · Motore Dati con il budget nuovo (dopo M3, piccolo)
- Obiettivo: il motore Dati usa `sicuro_mb` (lavoro = budget − cache), il riciclo segue `sicuro_mb`; mappe e `subxic` invariati. Il permesso grande serve solo se una rotta stima oltre `sicuro` (oggi nessuna): solo il punto d'aggancio.
- Prove: guardie `engine_*` invariate; `e2e_memoria_riduzioni.py`; misura della scheda prima/dopo.

### 5.7 M7 · Soglia del rumore nel Dig (decisione di Federico, prima della DEV)
- Domanda: usare `max(2e5, soglia stimata da params::estimate)` come altezza minima del Dig, mostrata nel passo «Features» e scritta nella ricetta (`.mzfeatf` con nuova impronta)? Cambia i risultati (meno feature di rumore), quindi serve un sì esplicito.
- Prima, dalle misure del Mac: feature per file, quota sotto 3 × rumore, righe deboli. Se la quota supera metà, M7 vale più di M2-M4 per questi dati.
- DEV (se sì): `crates/mzlab-feat/src/{params,recipe,trace}.rs`, `feat-worker.js` (ricetta), `docs/agenti/mzfeat.md`; prove di parità con soglia fissa invariate; serie sintetica con rumore: meno righe, stesse feature vere.
