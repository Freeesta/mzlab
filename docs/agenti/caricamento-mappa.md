# Mappa del caricamento di un file (sito e app desktop)

Fotografia del codice, non un piano: dove passa un file dal clic alla schermata, cosa costa, cosa manca. Dove la guida di area dice altro dal codice è scritto in «Dubbi». «Non trovato» = cercato e assente. Per i dettagli non si ripete ciò che è già scritto: `avvio.md` e `avvio-rapido.md` (avvio della pagina), `progresso.md` (barra), `lettura-nativa.md` (`.raw`, `.wiff`, `.D` senza mzML), `lettori-modulo.md` (moduli per famiglia), `mzidx.md` e `rust-mzidx.md` (indice e pool), `rust-desktop.md`
(desktop), `capacita.md`, `visore-veloce.md` (dopo l'apertura: XIC, zoom), `misure-velocita.md` e `ottimizzazioni-tentate.md` (numeri e strade già scartate: leggerle PRIMA di ottimizzare), `misure-macchine.md` (apertura su macchine lente), `studi/RAW_VELOCE.md`, `studi/wasm-potenza.md`.

## 1. Sito: il percorso comune (mzML)

| # | Passo (file:funzione) | Dove gira | Cosa fa / cosa legge |
|---|---|---|---|
| 1 | `shell.js` `#drop`/`#pick` (`onchange`, `drop`) → `upload()` (308) | pagina, async | Cancello `AVVIO.pronto()`: i file prima del motore restano in coda. `dropDups` toglie i doppioni. Cartelle: `cartella-carica.js` (`fromDrop`, `fromInput`) o `cartella-puntata.js` (File System Access). |
| 2 | `upload0` (314) | pagina | Prima converte ciò che il motore non legge (`wiffToMzml` 184, `rawToMzml` 179, `moduloToMzml` 227), poi `fileKind` scarta il resto. `PROGRESSO.begin` per TUTTI i file e una riga `CONVROWS` ciascuno subito. Più file nel browser: `Promise.all(files.map(one))`, altrimenti uno alla volta. |
| 3 | `one` → `sendFile` → `fetch POST api/upload` → `browser.js` → `lr.js` `upload` (84) | pagina | Il browser non ha rete: `browser.js` intercetta `fetch("api/...")` e lo gira al motore. Rifiuta file > 4 GB (`err.memory.over4gb`). Il `File` passa al worker per riferimento (nessuna copia). |
| 4 | `rust-worker.js` `lrPut` → `rust-carica.js` `copyPrints` | worker del motore, async | `ARCH.ensureRoom` (spazio ×1,8 per file+indice), toglie l'indice vecchio dello stesso nome (238), copia in OPFS a pezzi (`blob.slice().arrayBuffer()`, `putChunks`; «Annulla» ferma la copia al pezzo successivo e lo spazio promesso torna libero a ogni errore). Per un campione `.wiff` lo spazio chiesto è dei soli membri non ancora in OPFS. Nella stessa passata SHA-256, SHA-1 e `probe` vanno a `impronta-worker.js` (≤4 pezzi in attesa). Barra 0-40 %, aggiornata ogni 100 ms. Legge TUTTO il file una volta. |
| 5 | `lrIndex` (277) → `prepare_upload` (`mzlab-wasm/src/lib.rs` 624 → `upload::prepare_in`) | worker del motore, SINCRONO (il worker non risponde ad altro) | Tabella delle scansioni: con `indexListOffset` valido (`scantable::read_listed`) legge testata, ultimi 2 KB, blocco indice e ≥4 KB di intestazione per spettro (mai gli array); senza indice (`read_walked`) percorre tutto il file in blocchi da 1 MiB. Impronte già note: non rilegge. |
| 6 | `earlyTicOf` (258) | worker del motore | Dalla tabella ricava rt e TIC dichiarati e manda `progress {early_tic}`: la pagina disegna un TIC PROVVISORIO (`explore.js` `getChrom`, `provvisorio:true`) mentre l'indice si costruisce. |
| 7 | `idxPool().build` → `idx-worker.js` `build` + `build_mzidx` | pool di worker a vita breve (min(core−1, 4); 2 su iPad/telefono) | Legge la copia in OPFS con `SyncAccessHandle` (`lettura-sync.js`) a blocchi da 4 MiB (`BLOCK`, `mzlab-wasm/src/lib.rs` 74), decodifica ogni spettro UNA volta, ordina per m/z in secchi, i picchi oltre `budget` (64 MB, 32 su iPad) vanno a un file di scorta OPFS `.sp.part` (~20 B/picco). Con isolamento (SharedArrayBuffer) fino a 4 aiutanti decodificano base64/zlib (`idx-aiuti.js`). Scrive `.mzidx.part`, poi `ARCH.promote`. Barra 40-100 %. |
| 8 | `lrOpen`, `lr_entry`, `mergeFiles` | worker motore + pagina | Apre l'indice (intestazione, meta, tabella scansioni; i picchi restano nel file e si leggono a finestre) e riempie la tabella di carico. Nessun picco in memoria. |
| 9 | file letto → `OPENNOW` (`open-now.js`) | pagina | Ogni file entra nella sessione appena letto, nascosto; il primo non bianco si mostra da solo (`showFirst`, `bootSession`), gli altri si mostrano con un clic su un nome (elenco laterale o tabella dei campioni). I file in attesa sono righe grigie (`E.pending`). Nessun pulsante da premere: `upload()` e apertura procedono insieme. |
| 10 | `bootSession0` (`explore.js` 424) → `getChrom` TIC → `rust-bridge.js` `indexRoute` | pagina → worker | Primo grafico: TIC dall'indice (`op:"tic"`). Il risultato è un `Float64Array` TRASFERITO (non copiato) dal worker, restituito con `recycle` dopo il disegno (`rust-worker.js` 24, `rust-recycle.js`). Cache delle risposte `CACHE` (`cache-dati.js`), LRU a byte, 256 MB (96 MB se `deviceMemory` ≤ 2). `CHROM1` ordina le prime richieste: il primo cromatogramma prima del resto. |

Riapertura (stessa pagina, giorno dopo): `lr_boot` (`rust-worker.js` 309) apre ogni `.mzidx` già in OPFS (`lrOpen`) senza leggere spettri; un mzML senza indice (caricamento interrotto) si reindicizza (`lr.js` `boot`). Ricaricare lo stesso nome con `lr_put` cancella invece l'indice e rifà tutto.

Un solo worker del motore serve TUTTE le richieste (`rust-worker.js` `handle`): una chiamata lunga (come `prepare_upload`) ritarda le altre dello stesso worker, non la pagina. Se muore: `rust-supervisor.js` / `rust-bridge.js` `died` lo riavvia e riapre gli slot; la memoria WASM non torna mai al sistema, quindi `rust-recycle.js` chiude un worker gonfio dopo 30 s di quiete (soglia ≥192 MB).

## 2. Per formato

| Formato | Estensioni | Strada nel sito | Strada nel desktop | Cosa legge all'apertura |
|---|---|---|---|---|
| mzML | `.mzml` | §1, motore `.wasm` | `lr_index` → `IdxBuilder::build` (`idxbuild.rs` 131): `mmap`, `prepare_parallelo`, `build_mzml_spill` | testata, indice, intestazioni (§1 passo 5); array solo nell'indice |
| mzML con MS-Numpress | `.mzml` | il motore rifiuta (`err.file.numpress`); la pagina lo apre come campione nativo (`numpressToNat` in `shell.js`) e il motore lo indicizza dal lettore del modulo `aperti` (`aperti/mzml_numpress.rs`), senza riscrivere l'mzML | CLI: `flow_pace::is_numpress_mzml` manda l'indice al lettore `mzml-numpress` del registro; app: `natsamples` + `lr_put` con `nat` (`tests/numpress_mzml.rs`); dialogo/trascinamento non provati | tabella (`scantable::read`) + byte di ogni spettro |
| mzXML | `.mzxml` | solo modulo `aperti` (`soloModulo`, `lettori-famiglie.js`): conversione in mzML in OPFS, poi §1 | non trovato nel dialogo (`pick_files` offre mzML/raw) | `mzxml_indice.rs`: un passaggio sequenziale a blocchi da 1 MiB (l'`<index>` finale non è usato) |
| Thermo `.raw` | `.raw` | modulo `thermo` caricato a richiesta (`lettore-esterno.js`, `carica-modulo.js`, SHA-256 verificato), il motore lo legge via `native_external` SENZA mzML; indice v2 dal lettore | lettore nel core (feature `thermo`), `build_nat` (`idxbuild_nat.rs`) | `rawlettore.rs`: indici e tabella eventi, nessun picco; pacchetto per offset a richiesta; finestra 64 KB (3 blocchi da 256 KB nel modulo) |
| Waters `.raw` (cartella) | cartella con `_FUNC001.DAT` | modulo `waters` (conversione in mzML) | lettore nativo | `_FUNCTNS.INF` e indici; `.DAT` per posizione |
| SCIEX `.wiff` + `.wiff.scan` | coppia | motore (3200 QTRAP) come campione nativo; varianti → modulo `sciex` | `build_nat` | `.wiff` INTERO in un `Vec` (tetto `Limiti`), `.wiff.scan` mai intero (per blocchi); `apri_solo`: un campione |
| Agilent `.D` | cartella | motore base; varianti (QqQ MRM, Q-TOF, compressi) → modulo `agilent` | nativo | `MSScan.bin` intero (con tetto), `MSPeak.bin` per offset. Cartelle `.D` nel desktop: non trovato un percorso di apertura (`cartella.rs`, `apertura.rs` le escludono) |
| Bruker `.d` TDF/TSF | cartella | modulo `bruker` | nativo | tabelle SQLite, frame per offset; `.baf` rifiutato (`err.file.brukerBaf`) |
| Shimadzu | `.qgd`, `.lcd` | modulo `shimadzu` (conversione) | nativo | file INTERO in memoria (`dim*2 > limiti.byte` rifiuta); non a offset |
| ANDI/netCDF | `.cdf`, `.nc` | modulo `andi` | non trovato | colonne intere delle variabili di scansione |
| UV/DAD | dentro mzML o file dei costruttori | `mzml_dad.rs` (matrice `dad_matrix` letta dalla coda, oppure spettri `MS:1000804` a passi); vendor: `dad_map` del trait `Lettore`; `canali_uv.rs` serve `uv:twc`, `uv:<nm>` | stesso codice | `.uv` come file: non trovato |
| `.mzidx` | indice | solo come derivato in OPFS (`idx/`); come file da aprire: non trovato | solo come cache (`app_cache_dir()/mzidx`) | intestazione 64 B + meta + tabella scansioni; picchi a finestre di 64K (`idxwalk.rs`); versione > 2 → `error.newerFormat` |
| `.dam` | metodo SCIEX | `lr_put` senza indice (`rust-worker.js` 231) | idem | letto a richiesta |
| mzMLb, imzML | – | non trovato | non trovato | – |
| Librerie `.msp`, `.mgf`, `.json`, MassBank, `.mzlib` | – | non sono dati grezzi: `mzlab-lib` (`parse.rs` `detect`; `.mzlib` letto per directory, blocchi decodificati a richiesta); `libreria-worker.js` | – | `librerie.md` |
| `.mzfeat`, `.mzflow` | – | derivati/piani, non dati grezzi (`file-sicuri.js`: 8 MB, profondità 32) | – | – |

Il motore del sito ha nel registro solo Agilent `.D` base e SCIEX `.wiff` (`lettori.rs` 270, `Limiti::sito()` 192 MB); ogni altra famiglia è un `.wasm` a parte (`crates/mzlab-lettori-wasm`), caricato solo quando arriva un file di quella famiglia (`lettori-modulo.js`, `lettori-worker.js`: UN worker per le conversioni, nessun parallelismo interno trovato). Trasporto modulo→motore: `trasporto.rs` (`PROTOCOLLO` 2, colonne binarie).

Campioni nativi (`.raw`, `.wiff`, `.D`): `lr_put` con `nat` copia i membri in OPFS (nessun mzML), poi `build_mzidx_native` scrive l'indice v2 (fasi copy .2, sha .1, open .1, scans .35, chrom .25: `nativo-fasi.js`); `natsame` riconosce una coppia già caricata («dalla cache»). Dettagli: `lettura-nativa.md`.

## 3. App desktop

- **Ingresso**: clic su `#drop` o `#dropdam` intercettato da `shim/desktop-shim.js` → comando Tauri `pick_files` (dialogo in `spawn_blocking`, filtro mzML/raw, `main.rs` 53-70); trascinamento: `register_paths` (`main.rs` 116, solo `metadata`); doppio clic dal sistema: `apri.rs` (`single_instance`, argv, macOS `RunEvent::Opened`; coda `Pending`, `apertura.rs`; solo `.mzml`/`.raw` file). La pagina non vede mai il percorso: `State::pick` (`engine.rs` 137) assegna un numero, `toFile` crea un `File` vuoto con `type=application/x-mzlab-file;id=N`. Nessun dato letto in questo passo.
- **Trasporto**: `desktop-worker.js` (thread Worker JS) fa `fetch` su `mzlab://` (POST con JSON; risposte JSON o binarie per `tic`/`xic`/`spectra`, `scanbin`, `idxxicbin`, `natsrm`); coda unica dei messaggi (`line`), fuori coda solo `lr_index`, `lr_index_cen`, `nat2mzml`, `mzml2cen`. Lato Rust ogni richiesta ha un `std::thread::spawn` (`main.rs` 456). `State::rpc` (`rpc.rs` 340): le operazioni lunghe vanno SENZA il lock `Mutex<Eng>`; `lr_put`, `lr_entry`, `idxopen`, `route`, `scanbin` lo prendono e sono seriali.
- **mzML**: `lr_put` (`rpc_lr.rs` 65) non copia (stamp size+mtime in `store.rs`), tranne un file temporaneo dell'app (`fs::copy`); cancella l'indice del nome. `lr_index` (`rpc.rs` 125) → `IdxBuilder::build`: `Mapped::open` (mmap, `mapped.rs`), `upload::prepare_parallelo` (tabella + SHA-256 su un secondo thread), `ram::check_index`, `mzidx::build_mzml_spill` (budget 64 MB, scorta `.sp.part`), scrive `.part` poi `rename`. Pool rayon: min(core−1, 4), 1 minimo (`idxbuild.rs` 23-28). Decodifica con
  `parse_helped` (3 thread, `mzml/help/pool.rs`) e ordinamento dei secchi fino a 8 thread (`mzidx_ordine.rs`).
- **Nativi**: nessuna copia dei membri (mappati); `build_nat` (`idxbuild_nat.rs` 207) con impronte su un secondo thread; `lr_entry` legge da `Src::Nat` (`nativi.rs` 91): al più 4 lettori aperti e 32 MB di gusci, LRU (`nativi_slots.rs`). Dopo il caricamento ogni campione entra nella coda di conversione in sottofondo (`conv_coda.rs`: core−1 thread, aspetta con sleep di 100 ms finché si indicizza).
- **Cache dell'indice**: `app_cache_dir()/mzidx/<nome>.mzidx` (`avvio.rs` 17-20), validità size+mtime (`store.rs` 107, `lr_boot` `rpc_lr.rs` 51-63), poi `Index::open` controlla magic e dimensione. A freddo (nuovo processo) il codice trovato fa `?riparti=1` → `lr_clear` che rimuove gli indici (`rpc_lr.rs` 182): l'indice non sopravvive al riavvio dell'app (da confermare a mano).
- **Letture successive** (`mzmlroute`, `scanbin`, `specexact`): `map_file` (`rpc.rs` 399) rifà `open`+`mmap` a ogni richiesta, nessuna cache della mappa.
- **Memoria**: budget `native_auto_mb` = RAM/4 tra 1024 e 16384 MB (`ram.rs`, `budget_auto.rs`); `check_index` rifiuta prima con `error.memoryBudget`. `ram_mb()` legge `/proc/meminfo` o `sysctl`: su Windows probabilmente 0 → 1024 MB (non verificato). `Limiti::desktop()`: 1024 MB, 50 M voci. Il vecchio `State::open` (`engine.rs` 165) legge TUTTO il file in RAM (`mzml::read`/`raw::read`) ed è solo ripiego.

## 4. Annullamento, attesa, limiti

- **Sito, Annulla** (`cancelLoads`, `shell.js` 298): aborta i file non partiti (`LOADS`), ferma la coda di conversione e manda `lr_cancel` → `CANCELLED` + `idxPool().cancel(nome)` che termina il worker dell'indice (`idxpool.js` `cancel`); `lr.js` toglie il segnaposto (`lr_remove`, `err.load.cancelled`). La copia di un mzML semplice NON controlla `CANCELLED` (solo la copia dei nativi lo fa, `rust-worker.js` 246) e `prepare_upload` è sincrono e non interrompibile: si controlla prima di `lr_index`
  (280) e il builder muore a metà. Un worker del motore bloccato si ferma solo con `abort` (`rust-bridge.js` `cancelable`), che lo riavvia.
- **Desktop, Annulla**: `lr_cancel` non esiste in Rust (`unknown operation`, la pagina lo ignora); `lr_put`, `lr_index`, `natsamples`, `nat2mzml` non hanno flag; `conv_cancel` ferma solo i job in coda. Fermare un `nat2mzml` ferma il worker JS ma la scrittura Rust finisce.
- **Cosa vede l'utente**: righe di caricamento con barra e fase (`CONVROWS`, `progresso.js`, evento `mzlab-index {name, frac, phase, done, total, wait, early_tic}`), tempo trascorso ogni secondo, «nessuna notizia da N s» dopo 5 s, «in coda» (`wait`) se il pool è pieno, nessun velo sulla pagina (l'avanzamento sta solo nell'elenco laterale); TIC provvisorio (passo 6); mappa con anteprima grigia (`dati-mappa.md`). Desktop: il worker invia progresso solo da `rawprogress` (sondato ogni 300 ms) per `nat2mzml`/`mzml2cen`; per
  `lr_put`/`lr_index` non emette `progress`, mentre `lr.js` (93-98) li attende (cosa disegni la barra: non verificato).
- **Memoria sito**: motore WASM fino a 4 GiB, budget di lavoro per livelli 96…3072 MB, predefinito 192 (`budget_auto.rs`, `rust-budget.js`, scelta automatica); indice: 64 MB (32 iPad); tabella scansioni ~1,5 KB/scansione (145 MB su 100 000); `error.memoryBudget` dimezza pool e budget e ritenta una volta; quota OPFS prenotata prima (`ensureRoom`, `error.quota`); file > 4 GB rifiutato. Senza isolamento il builder decodifica da solo e un mzML ≥ 32 MB mostra l'avviso `indiceParallelo` (`capacita.js`). Prova pesante: `tests_e2e/e2e_carica_grandi.py`, `e2e_wiff_240.py`.
- **Copie lungo il percorso** (sito): disco → `Blob` (riferimento) → OPFS (una copia JS per pezzo: `arrayBuffer()` poi scrittura) → ogni `read` del lettore Rust: `Uint8Array` JS + `copy_to` nel buffer WASM (`JsReader::read_at` `lib.rs` 179; `Source::read` di `scantable.rs` 450 alloca un `Vec` a ogni lettura) → risultati verso la pagina: `Float64Array` trasferito. In più `prepare_upload` copia le tabelle in `Uint8Array` (rt e tic due volte, `lib.rs` 624) e `build_mzidx` rifà `to_vec`/`collect`
  (`levels`, polarità, rt, tic). Desktop: `mmap` senza copia verso Rust; copie inutili: `f64_bytes` (`protocol.rs` 22), `idxxicbin` (`rpc.rs` 273, due `Vec`), `decode_inputs` (`idxbuild.rs` 55), `Blob → ArrayBuffer → POST` per un `File` senza tag (`desktop-worker.js` 34-39, a pezzi da 32 MB).

## 5. Strategie: presenti?

| Strategia | Stato | Dove / nota |
|---|---|---|
| Streaming (mai tutto in memoria) | presente | copia a pezzi in OPFS; builder che legge a blocchi da 4 MiB; scorta su disco per i picchi. Eccezioni: `.wiff`, Shimadzu e netCDF interi in memoria; il vecchio `State::open` del desktop |
| Lettura pigra con indice | presente | `.mzidx`: tabella scansioni in memoria, picchi a finestre (`idxwalk.rs`), TIC/picco base dentro l'indice (aprire non legge picchi); mzML con indice di offset: solo intestazioni |
| Prima immagine approssimata poi raffinata | parziale | TIC provvisorio dalla tabella (passo 6); anteprima della mappa. Nessun XIC/spettro approssimato; il TIC provvisorio c'è solo se la tabella arriva prima dell'indice |
| Worker / thread | presente | sito: worker del motore, pool di indici, aiutanti, worker delle impronte, worker dei moduli; desktop: rayon, thread per richiesta, coda di conversione. La pagina non fa I/O pesante |
| Precaricamento del prossimo / del probabile | parziale | motore e `.wasm` partono a `DOMContentLoaded` (`browser.js`), script del costruttore messi in cache dal service worker, Ketcher e mappa 3D a idle, `XICBTN.prefetch`; i file in coda sono indicizzati dal pool PRIMA di aprirli. Nessun precarico speculativo di TIC/XIC/spettri di dati: non trovato (strade provate in `ottimizzazioni-tentate.md`) |
| Cache dei decodificati | parziale | sì: `.mzidx` su disco (il decodificato ordinato per m/z), `CACHE` LRU delle risposte, piramide delle tessere della mappa in OPFS (`rust-worker.js` 39-70). No: spettri decodificati, mappe dei mmap (desktop), indice desktop dopo il riavvio a freddo |
| Decodifica parallela | presente | aiutanti SharedArrayBuffer (solo con isolamento), `parse_helped` (3 thread), ordinamento fino a 8 thread (nativo), codifica mzML in pool (`mzmlwrite/pool.rs`); il `.wasm` del motore resta sequenziale dove non c'è isolamento |
| Zero-copy Rust/WASM/JS | parziale | risultati numerici trasferiti e riciclati; `Blob` per riferimento; `mmap` nel desktop. Una copia per ogni `read` verso WASM e per ogni pezzo scritto in OPFS; tabelle copiate in `prepare_upload` |
| Strutture compatte | presente | `.mzidx` a colonne con `fence` per blocchi di 16384, MS1 HR in `f32`, trasporto binario a colonne dei moduli (`trasporto.rs`), `xicbin`; righe della tabella come JSON compatto |
| Annullamento | parziale | sito: coda e builder sì, copia semplice e `prepare_upload` no; desktop: assente per i lavori di apertura (`lr_cancel` non trovato) |
| Limiti di memoria | presente | §4; tetti dei moduli (`tools/build_lettori_wasm.sh`), `.wasm` del motore ≤ 1 703 936 B |
| Progresso fedele | presente nel sito, parziale nel desktop | `progresso.md`; desktop senza eventi per l'indicizzazione |

## 6. Dubbi e discrepanze trovate (da verificare prima di agire)

1. `mzidx.md` dice che il motore riconosce l'indice da dimensione e `probe`; nel codice letto `lrOpen` (`rust-worker.js` 116) usa il NOME e `Index::open` controlla magic e `total` dell'indice; un confronto di `probe`/`sha256` con l'mzML alla riapertura: non trovato (`infoDi` legge `sorgente_sha256` solo come informazione). Un file con lo stesso nome ma altro contenuto è scartato perché `lr_put` cancella l'indice, non per un confronto.
2. `rust-desktop.md` dice che l'indice è `<dimensione>_<nome>`; il codice scrive `<nome>.mzidx` (`idxbuild.rs` 84): due mzML con lo stesso nome si sovrascrivono.
3. Desktop: `lr_cancel` senza risposta in Rust; nessun `progress` per `lr_put`/`lr_index`; indice che non sopravvive al riavvio a freddo; budget su Windows probabilmente 1024 MB per `ram_mb()`; più thread del tetto di 4 dichiarato.
4. Desktop: cartelle `.D` e `.mzidx` come ingresso: non trovato; non tracciato cosa mostri la pagina quando si trascina una cartella `.D`.
5. Il piano di lavoro del sito dopo `lr_index`: `lr_entry` e `lr_items` rileggono l'indice per ogni file (sequenza per file in `lr.js` `explore`): non misurato quanto pesi con molti file (vedi `misure-macchine.md`, apertura a 6×).

## 7. Lotti per Gemini (priorità: ciò che pesa di più sul tempo di apertura)

Uso: `python3 tools/lotto_gemini.py docs/agenti/caricamento-mappa.md --lotto 1 -o lotto1.txt` (avvisa se il lotto supera `--max-kb`, 200 per default; `--controlla` stampa le dimensioni). Dimensioni in KB tra parentesi nell'elenco di `--controlla`; i lotti sono ≈150-200 KB.

@lotto 1
crates/mzlab-core/src/upload.rs
crates/mzlab-core/src/upload/passata.rs
crates/mzlab-core/src/upload/late.rs
crates/mzlab-core/src/upload/non_mzml.rs
crates/mzlab-core/src/scantable.rs
crates/mzlab-core/src/mzml.rs
crates/mzlab-core/src/mzidx.rs
crates/mzlab-core/src/mzidx_lettore.rs
crates/mzlab-core/src/mzidx_spill.rs
crates/mzlab-core/src/mzidx_ordine.rs
crates/mzlab-core/src/idxread.rs
crates/mzlab-core/src/idxwalk.rs
crates/mzlab-core/src/idxcheck.rs
crates/mzlab-core/src/sorgente.rs
crates/mzlab-core/src/scanrow.rs
crates/mzlab-core/src/window.rs
crates/mzlab-core/src/blocchi.rs
crates/mzlab-core/src/budget.rs
crates/mzlab-core/src/stima.rs
crates/mzlab-core/src/lettori.rs

@lotto 2
mzlab/web/rust-worker.js
mzlab/web/rust-bridge.js
mzlab/web/rust-nativo.js
mzlab/web/rust-carica.js
mzlab/web/rust-nativo-pagina.js
mzlab/web/idx-worker.js
mzlab/web/idxpool.js
mzlab/web/idx-spill.js
mzlab/web/idx-aiuti.js
mzlab/web/idx-aiuto.js
mzlab/web/archivio.js
mzlab/web/impronta.js
mzlab/web/impronta-worker.js
mzlab/web/lettura-sync.js
mzlab/web/rust-opfs.js
crates/mzlab-wasm/src/lib.rs
crates/mzlab-wasm/src/nativi.rs
crates/mzlab-wasm/src/nativi_motore.rs
crates/mzlab-wasm/src/contato.rs
crates/mzlab-wasm/src/corriere.rs
crates/mzlab-wasm/src/cache.rs

@lotto 3
mzlab/web/shell.js
mzlab/web/lr.js
mzlab/web/browser.js
mzlab/web/carica.js
mzlab/web/progresso.js
mzlab/web/avvio-progressivo.js
mzlab/web/nativo.js
mzlab/web/nativo-fasi.js
mzlab/web/conversione.js
mzlab/web/cartella-carica.js
mzlab/web/tipo-file.js
mzlab/web/file-sicuri.js
mzlab/web/chrom-primo.js
mzlab/web/cache-dati.js
mzlab/web/first-paint.js
mzlab/web/telefono.js

@lotto 4
mzlab/web/lettori-modulo.js
mzlab/web/lettori-worker.js
mzlab/web/lettori-famiglie.js
mzlab/web/lettore-esterno.js
mzlab/web/carica-modulo.js
mzlab/web/cartella-puntata.js
crates/mzlab-wasm/src/esterno.rs
crates/mzlab-lettori/src/lib.rs
crates/mzlab-lettori-wasm/src/lib.rs
crates/mzlab-lettori-wasm/src/sessione.rs
crates/mzlab-core/src/trasporto.rs
crates/mzlab-core/src/rawlettore.rs
crates/mzlab-core/src/wiff/lettore.rs
crates/mzlab-core/src/agilent/lettore.rs
crates/mzlab-lettori/src/waters/lettore.rs
crates/mzlab-lettori/src/shimadzu/mod.rs
crates/mzlab-lettori/src/andi/ms.rs
crates/mzlab-lettori/src/aperti/mzxml.rs
crates/mzlab-lettori/src/aperti/mzxml_indice.rs
crates/mzlab-lettori/src/aperti/mzml_numpress.rs

@lotto 5
crates/mzlab-desktop/shim/desktop-worker.js
crates/mzlab-desktop/shim/desktop-shim.js
crates/mzlab-desktop/src/main.rs
crates/mzlab-desktop/src/engine.rs
crates/mzlab-desktop/src/rpc.rs
crates/mzlab-desktop/src/rpc_lr.rs
crates/mzlab-desktop/src/protocol.rs
crates/mzlab-desktop/src/store.rs
crates/mzlab-desktop/src/idxbuild.rs
crates/mzlab-desktop/src/idxbuild_nat.rs
crates/mzlab-desktop/src/mapped.rs
crates/mzlab-desktop/src/ram.rs
crates/mzlab-desktop/src/nativi.rs
crates/mzlab-desktop/src/nativi_slots.rs
crates/mzlab-desktop/src/conv_coda.rs
crates/mzlab-desktop/src/apri.rs
crates/mzlab-desktop/src/apertura.rs
crates/mzlab-desktop/src/avvio.rs
crates/mzlab-desktop/src/cartella.rs

@lotto 6
mzlab/web/explore.js
mzlab/web/rust-supervisor.js
mzlab/web/rust-recycle.js
mzlab/web/rust-budget.js
mzlab/web/capacita.js
mzlab/web/memoria-avviso.js
mzlab/web/avanzamento.js
mzlab/web/sw.js
mzlab/web/rust-scans.js
mzlab/web/rust-xicbin.js
mzlab/web/rust-centroidi.js
mzlab/web/peaks-worker.js
crates/mzlab-core/src/routes.rs
crates/mzlab-core/src/hrscans.rs
crates/mzlab-core/src/fileroutes.rs
crates/mzlab-core/src/lr.rs
crates/mzlab-wasm/src/items.rs
crates/mzlab-wasm/src/scans.rs
crates/mzlab-wasm/src/info.rs
crates/mzlab-wasm/src/xicbin.rs
