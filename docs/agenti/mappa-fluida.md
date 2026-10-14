# Mappa fluida: studio delle transizioni della mappa RT×m/z

Studio (nessuna modifica al prodotto) su zoom, spostamento, mirino, cambio di file, scala e 3D della mappa 2D, con misure su file sintetici fino alla scala HRMS. Le funzioni della mappa sono descritte in `dati-mappa.md`; qui ci sono le misure, la diagnosi e i pacchetti di lavoro numerati.

## Come rifare le misure
- `MZLAB_CHROMIUM=<chrome> python3 tools/bench_mappa.py lr,hrc,hrp --wd DIR [--json]`: crea i file mzML sintetici in `DIR` (non nel repository), costruisce il sito con il motore WASM (`build_site.py --no-pyodide` + `cargo build` + `wasm-bindgen`), apre due file per caso in Chromium headless con profilo persistente (la quota di un contesto in incognito rifiuta i file da 300 MB) e misura i gesti. `--json` stampa una riga per caso.
- Casi: `lr` 1500 scansioni × 500 picchi (0,75 M picchi, 14 MB); `hrc` 3000 × 3000 centroidi (9 M, 150 MB); `hrc_big` 6000 × 5000; `hrp` 400 × 50 000 in profilo (20 M, 320 MB); `hrp_big` 600 × 150 000. `nome:SxP` cambia le dimensioni.
- Ganci dello script (solo nel browser del test): `fetch` delle rotte `api/map|spectra|xic`, `drawMap`, `mapImage`, `MAPPA.zoomImg` (vero = disegnata la regione nitida), intervalli fra `requestAnimationFrame` e long task, picco della memoria WASM dai messaggi del worker, RSS di Chromium da `/proc`.
- Gesti: prima mappa a schermo intero (rotta, `JSON.parse`, base64, colore); 12 scatti di rotella; spostamento con Maiusc+trascinamento; zoom solo in RT (Maiusc+rotella); zoom profondo (2 min × 2 m/z); cambio di file freddo, ritorno e di nuovo caldo; 60 passi del mirino; 20 spostamenti (voci di `CACHE` e heap JS aggiunti).
- Confronto nativo: un piccolo binario fuori dal repository che apre l'`.mzidx` con `Index::open(File)` e chiama `Routes::map_route` con le stesse query (un giro freddo e uno caldo).

## Il percorso dei dati (cosa sta in memoria)
1. **Indice** `.mzidx` in OPFS: i picchi MS1 ordinati per m/z (colonne mz, intensità, posizione; 12 B/picco in HR), con un recinto ogni 16384 picchi. Il worker lo legge con `FileReaderSync.readAsArrayBuffer(blob.slice(...))` (`rust-worker.js` `readerFor`) a pezzi di 65 536 picchi (`idxread.rs` `each_peak`, tre letture per pezzo): ogni lettura è una chiamata sincrona che copia dal processo del browser.
2. **Mappa intera** (`api/map` senza finestra): la griglia della sessione (≤600 righe RT, celle di 1 Da anche in HR) costruita leggendo TUTTI i picchi del file; Rust tiene le ultime 6 per file (`ion_map`), la pagina le ricorda in `memo`.
3. **Regione** (`ionmap_region`, al massimo 2048 × 1024): legge tutti i picchi della banda m/z per tutto il tempo e scarta quelli fuori RT, quindi uno zoom solo in RT legge l'intero file.
4. **Trasporto**: `Vec<f32>` → base64 → testo JSON → `postMessage` → `Response` → `r.json()` → `atob` carattere per carattere → `Float32Array`: circa sei copie, 1,33 volte la dimensione.
5. **Colore** (`grafici.js` `mapImage`): un array JS delle celle non nulle, ordinato per il 99,5° percentile, poi un pixel alla volta con una funzione; l'immagine sta in un solo posto per pannello (`p._img`), quindi ogni cambio di file ricolora tutto.
6. **Regioni in memoria**: `CACHE` di `explore.js` è una `Map` senza limite; ogni regione resta per sempre (il `Float32Array` e il suo testo base64).
7. **Disegno**: `draw(p)` a ogni evento di input (non uno per frame); `MAPPA.after` ridimensiona la tela del sovrapposto e riscrive legenda e tabella a ogni disegno; la regione nitida parte 150 ms (`QUIET`) dopo l'ultimo gesto.
8. **Mirino**: LRU proprio di spettri (64 MB, 32 su iPad) con lettura anticipata: è già adeguato.
9. **3D** (`mappa3d.js`): la stessa rotta di regione fino a 1024², quindi eredita il punto 3.

## Misure (Chromium headless, container a 4 CPU, sito con motore Rust/WASM)
| gesto | LR 1500×500 | HRC 3000×3000 |
|---|---|---|
| apertura di due file | 3,6 s | 48 s |
| prima mappa (rotta / colore) | 791 ms (505 / 238) | 6332 ms (5830 / 453) |
| mappa intera calda: rotta / parse / base64 / colore | 45 / 2,7 / 10,6 / 192 ms | 79 / 3,8 / 17,9 / 397 ms |
| celle e JSON della mappa intera | 600×521, 1628 kB | 600×901, 2816 kB |
| rotella: frame p50 / p95 / max | 16,7 / 50 / 83 ms | 16,7 / 33,4 / — ms |
| rotella: fino al nitido / rotta | 176 / 43 ms | 249 / 113 ms |
| spostamento: fino al nitido / rotta | 176 / 41 ms | 217 / 101 ms |
| zoom solo RT: fino al nitido / rotta | 295 / 163 ms | 1617 / 1465 ms |
| zoom profondo: fino al nitido / rotta | 192 / 50 ms | 259 / 92 ms |
| cambio file freddo / ritorno / caldo | 456 / 219 / 211 ms | 1992 / 490 / 441 ms |
| mirino: `api/spectra` p50 / p95 | 6,1 / 16,4 ms | 9,6 / 34,6 ms |
| RSS Chromium / heap JS / picco WASM | 1314 / 91 / 32 MB | 1931 / 103 / 175 MB |

Profilo HR (`hrp` 400 × 50 000, 2 × 320 MB): non ancora misurato.

Nativo, stesso indice `hrc` (108 MB, file nella cache del sistema): mappa intera 254 ms a freddo e 14 ms dalla cache; regione 100-1000 m/z su tutto il tempo 212-273 ms; regione profonda 2 min × 2 m/z 29-45 ms. Nel browser le stesse rotte prendono 5830 ms e 1465 ms: il calcolo è lo stesso codice, il resto è lettura.

Colore della mappa intera HRC (600 × 901, Node): algoritmo attuale 314 ms; con `Float32Array` ordinato, scala precalcolata in `Uint32Array` e scrittura a 32 bit 52 ms (6 volte meno).

## Diagnosi (in ordine di peso)
- **D1 Lettura dell'indice nel browser**: la mappa intera HR legge 9 M picchi in 5,8 s contro 0,25 s nativi (23 volte). Il sospetto principale è `FileReaderSync` su `Blob.slice` (una copia attraverso il processo del browser a ogni lettura) più la copia `Uint8Array` → memoria WASM; il calcolo WASM vale al più 2 volte il nativo.
- **D2 Zoom solo in RT**: la regione legge tutta la banda m/z per tutto il tempo (1,5 s in HR); il recinto ogni 16384 picchi serve solo all'm/z.
- **D3 Colore in JS**: 0,2-0,4 s per ogni mappa intera e rifatto a ogni cambio di file (un solo posto `p._img`): è la quasi totalità dei 441 ms del cambio di file caldo in HR.
- **D4 Attesa del nitido**: 150 ms di silenzio + rotta + disegno ≈ 180-250 ms anche quando la rotta costa 40 ms.
- **D5 Frame lunghi durante il gesto**: p95 33-50 ms; un `draw` per evento e lavoro del sovrapposto (ridimensiona la tela, legenda, tabella) a ogni disegno.
- **D6 `CACHE` senza limite**: ogni regione e il suo base64 restano in memoria; in una lunga sessione HR cresce senza tetto (a 20 spostamenti l'aumento è misurato dallo script).
- **D7 Trasporto testuale**: sei copie e +33 %; pochi ms di tempo ma memoria doppia nei picchi.
- **D8 Picco WASM 175 MB in HR** con tetto 256 e bilancio 192 (`memoria.md`): su iPad è il limite più vicino.
- **D9 3D**: come D2 per finestre lunghe in RT.
- **D10 Mirino**: 10-35 ms per spettro in HR con LRU e lettura anticipata: non è un collo di bottiglia.

## Obiettivi
- Frame durante zoom e spostamento: p95 ≤ 17 ms (60 fps), anche in HR.
- Nitido dopo il gesto: ≤ 120 ms in LR, ≤ 200 ms in HR; mai più di 300 ms, anche solo in RT.
- Prima mappa ≤ 1 s per un file HR tipico (~10 M picchi MS1); cambio di file caldo ≤ 50 ms, freddo ≤ 300 ms.
- Mirino: spettro percepito < 16 ms (già vero dalla cache, p95 da tenere < 35 ms).
- Picco WASM ≤ 128 MB in HR; memoria della pagina limitata da un bilancio in byte, non dal numero di gesti.

## Ipotesi valutate
| ipotesi | guadagno atteso | costo | esito |
|---|---|---|---|
| letture con `FileSystemSyncAccessHandle` o a blocchi grandi | D1: 5,8 s → < 1 s | basso, solo worker + Rust | sì, P1 |
| piramide di massimi a tessere in Rust, salvata in OPFS | D1-D2-D9: ogni zoom costa le celle, non i picchi | medio | sì, P2 |
| scala precalcolata + percentile da Rust + LRU di immagini per file | D3: 400 → 50 ms, cambio caldo ~0 | basso | sì, P3 |
| `Float32Array` trasferibile al posto di base64 nel JSON | D7, -2 copie | medio (tocca le rotte) | sì, P4 |
| LRU delle regioni a byte, margine prefetch, attesa adattiva, annullo delle richieste vecchie | D4-D6 | basso | sì, P5 |
| un disegno per frame (rAF) e sovrapposto solo se cambia | D5 | basso | sì, P6 |
| colore su GPU (WebGL, scala come texture) | D3 sotto 5 ms, zoom continuo | medio-alto | dopo P3, se serve (P7) |
| OffscreenCanvas nel worker | toglie il colore dal filo principale | medio | assorbita da P3/P4: il colore costa poco dopo P3 |
| decimazione min-max | utile per le tracce 1D, non per una griglia già al massimo | — | no per la mappa |
| SharedArrayBuffer | zero copie | richiede COOP/COEP, che GitHub Pages non manda | no |
| app desktop: mmap + rayon | D1 nativo già 0,25 s; tessere in parallelo | medio | sì, P8 (dopo P2) |
| tetto dei `.wasm` più alto | — | vietato | no: P1-P2 aggiungono poco codice al motore (stimato < 20 KB su 1 703 936 B) |

## Pacchetti (in ordine)
Ogni pacchetto è una PR con la sua misura `bench_mappa.py` prima e dopo nella descrizione. «Libero» = non tocca i file delle chat in corso (mappa 2D: spettro laterale in profilo e blocco dell'asse in `mappa.js`/`grafici.js`; lista dei file e pannelli; caricamento).

### P1 Lettura veloce dell'indice nel worker
- Scopo: D1, D8. Prima uno strumento che separa lettura e calcolo (tempo dentro `read_at` contato nel worker); poi `createSyncAccessHandle()` sull'`.mzidx` di OPFS al posto di `FileReaderSync` (lettura diretta in un buffer riusato), con `read_at` su blocchi da 1-4 MB e un piccolo LRU di blocchi; ripiego su `FileReaderSync` se il handle è già preso o assente.
- File: `mzlab/web/rust-worker.js` (`readerFor`), `crates/mzlab-wasm/src/lib.rs` (`JsReader`), `crates/mzlab-core/src/idxread.rs` (passo di `each_peak`).
- Guadagno misurabile: prima mappa HRC da 6,3 s a ≤ 1 s; zoom solo RT da 1,5 s a ≤ 0,3 s; nessuna variazione dei byte delle rotte (parità invariata).
- Rischio: basso. Il handle è esclusivo per file (una sola apertura nel worker); su Safari il comportamento di `read` con `{at}` cambia fra versioni e va provato su iPad; il ripiego resta.
- Ordine: primo, indipendente. Libero.

### P2 Piramide di tessere della mappa in Rust
- Scopo: D2, D9 e prima mappa istantanea al secondo accesso. Livelli di massimi (e somme) su una griglia agganciata (RT per scansione, m/z a potenze di 2 della cella), a tessere 256 × 256 sparse; costruita una volta per file (in un passaggio sui picchi già ordinati per m/z) e salvata in OPFS accanto all'`.mzidx`; una rotta nuova `api/maptiles` rende il rettangolo della vista al livello giusto; la regione esatta resta per media, fondo, differenza e per lo zoom più profondo.
- File: nuovo `crates/mzlab-core/src/maptiles.rs` + rotta in `maproute.rs`; `mzlab-wasm` per salvare/leggere le tessere; parità in `crates/mzlab-core/tests/` contro `region_map` su griglie allineate (Rust contro Rust, nessun Python nuovo).
- Guadagno: ogni zoom e spostamento costa le celle della vista (≤ 2 M) e non i picchi; nitido HR ≤ 100 ms anche solo in RT; prima mappa ≤ 0,2 s quando la piramide esiste.
- Rischio: medio (spazio OPFS: circa 1/8 dell'indice; invalidazione con l'impronta dell'indice; definizione del massimo per celle a cavallo).
- Ordine: dopo P1 (che ne accelera la costruzione). Libero fino all'aggancio nella pagina, che va in P5.

### P3 Colore veloce e immagini per file
- Scopo: D3. `mapImage` con `Float32Array`, percentile da un istogramma (o calcolato in Rust e restituito con la mappa), scala Okabe-Ito/viridis precalcolata in `Uint32Array` e scrittura su `Uint32Array` dell'`ImageData`; LRU di immagini per (file, scala, livello) con un bilancio in byte (es. 64 MB, 32 su iPad).
- File: `mzlab/web/grafici.js` (`mapImage`, `drawMap`), `mzlab/web/mappa.js` (`p._img`).
- Guadagno: colore HRC da 400 a ≤ 60 ms; cambio di file caldo da 441 a ≤ 50 ms; cambio di scala ≤ 60 ms.
- Rischio: basso; i colori devono restare gli stessi pixel (test che confronta l'immagine prima e dopo).
- Ordine: quando la chat della mappa 2D ha unito. Conflitto con quella chat sugli stessi file.

### P4 Trasporto binario delle rotte della mappa
- Scopo: D7. Il worker restituisce JSON piccolo + `ArrayBuffer` trasferito (`postMessage(..., [buf])`), la pagina costruisce il `Float32Array` senza `atob`; vale per `api/map`, la regione e il 3D.
- File: `crates/mzlab-wasm` (uscita a offset come in `memoria.md`), `rust-worker.js`, `rust-bridge.js`, `browser.js`, `explore.js` (`getMap`, `getRegion`), `mappa3d.js`; il JSON con base64 resta per l'app desktop e i test di parità.
- Guadagno: -2 copie e -33 % di memoria per regione; 10-20 ms per mappa intera HR.
- Rischio: medio (cambia il contratto fra worker e pagina; `MZLAB_LR_MISS` e gli e2e delle rotte vanno ricontrollati).
- Ordine: dopo P3; indipendente da P1-P2. Conflitto possibile con la chat del caricamento su `rust-worker.js`.

### P5 Regioni in memoria, prefetch e attesa adattiva
- Scopo: D4, D6. LRU di `CACHE` per byte (es. 96 MB, 48 su iPad) senza il testo base64; richiesta con margine (la vista ±50 %) così lo spostamento successivo è già nitido; `QUIET` adattivo (40 ms se la rotta recente è sotto 30 ms, 150 ms se lenta); annullo delle risposte vecchie (numero di sequenza) e una sola richiesta in volo; con P2, tessere invece di regioni.
- File: `mzlab/web/mappa.js` (`getRegion`, `regionGrid`, `paint`), `mzlab/web/explore.js` (`CACHE`).
- Guadagno: nitido dopo il gesto da 176-250 a ≤ 120 ms; memoria a tetto fisso.
- Rischio: basso-medio (i test che contano le richieste).
- Ordine: dopo la chat della mappa 2D; meglio dopo P2.

### P6 Un disegno per frame
- Scopo: D5. `draw(p)` mette in coda e un solo `requestAnimationFrame` disegna; `MAPPA.after` ridimensiona la tela solo se cambia la misura, riscrive legenda e tabella solo se cambiano i dati (non a ogni gesto, la tabella al più dopo il silenzio).
- File: `mzlab/web/mappa.js`, `mzlab/web/grafici.js` (`drawMap`).
- Guadagno: frame p95 da 33-50 a ≤ 17 ms durante il gesto.
- Rischio: basso; il tour e i test che leggono la tabella subito dopo un gesto vanno ricontrollati.
- Ordine: insieme a P3 o subito dopo (stessi file). Conflitto con la chat della mappa 2D.

### P7 Colore su GPU (facoltativo)
- Scopo: zoom continuo a 60 fps anche su griglie 2048 × 1024. Celle in una texture float, scala come texture 1D, shader che applica log/lineare e percentile; ripiego su P3 se WebGL manca.
- File: nuovo `mzlab/web/mappa-gl.js` (modulo ES), aggancio in `grafici.js`.
- Guadagno: colore < 5 ms, cambio di scala istantaneo.
- Rischio: medio-alto (CSP e contesti persi su iPad, stessa resa dei colori, accessibilità invariata).
- Ordine: solo se dopo P3-P6 i frame non sono ancora a 60 fps.

### P8 App desktop: lettura mappata e parallela
- Scopo: la mappa nativa già 0,25 s; con `memmap2` per l'`.mzidx` e `rayon` per costruire le tessere di P2 su più file.
- File: `crates/mzlab-desktop`, `mzlab-core` dietro una feature che il `.wasm` non compila.
- Guadagno: piramide di un file HR in < 0,5 s; sessioni con molti file.
- Rischio: basso; nessun effetto sul sito.
- Ordine: dopo P2. Libero.

### P9 3D sulla piramide
- Scopo: D9. `mappa3d.js` chiede le tessere di P2 al livello della sua griglia invece della regione esatta.
- File: `mzlab/web/mappa3d.js`.
- Guadagno: apertura del 3D HR da secondi a ≤ 0,3 s.
- Rischio: basso.
- Ordine: dopo P2 e P4. Libero.

### P10 Bilanci di memoria per iPad
- Scopo: D8. Picco WASM ≤ 128 MB: con P1 il buffer di lettura è fisso, la mappa intera di Rust tiene 6 griglie per file solo se entrano nel bilancio di `mzlab-core::budget`; misura su iPad reale.
- File: `crates/mzlab-core/src/budget.rs`, `maproute.rs` (`ion_map`), `docs/agenti/memoria.md`.
- Rischio: basso.
- Ordine: insieme a P1 o subito dopo. Libero.

## Ordine consigliato
P1 → P10 → P2 (motore, liberi, subito) · P3 + P6 → P5 (pagina, dopo l'unione della chat della mappa 2D) · P4 (dopo P3, accordo con la chat del caricamento) · P9, P8 · P7 solo se serve.

## Limiti dello studio
- Misure su un container Linux headless a 4 CPU; iPad e Safari non sono misurati (P1 e P10 chiedono una prova reale).
- I documenti di contesto del repository privato dei dati non erano leggibili da questa sessione.
