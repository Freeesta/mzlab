# Mappa fluida: studio delle transizioni della mappa RT×m/z

Misure, diagnosi e lavori aperti per zoom, spostamento, mirino, cambio di file, scala e 3D della mappa 2D, con file sintetici fino alla scala HRMS. Le funzioni della mappa sono descritte in `dati-mappa.md`.

## Come rifare le misure
- `MZLAB_CHROMIUM=<chrome> python3 tools/bench_map.py lr,hrc,hrp --wd DIR [--json]`: crea i file mzML sintetici in `DIR` (non nel repository), costruisce il sito con il motore WASM (`build_site.py` + `cargo build` + `wasm-bindgen`), apre due file per caso in Chromium headless con profilo persistente (la quota di un contesto in incognito rifiuta i file da 300 MB) e misura i gesti. `--json` stampa una riga per caso.
- Casi: `lr` 1500 scansioni × 500 picchi (0,75 M picchi, 14 MB); `hrc` 3000 × 3000 centroidi (9 M, 150 MB); `hrc_big` 6000 × 5000; `hrp` 400 × 50 000 in profilo (20 M, 320 MB); `hrp_big` 600 × 150 000. `nome:SxP` cambia le dimensioni.
- Ganci dello script (solo nel browser del test): `fetch` delle rotte `api/map|spectra|xic`, `drawMap`, `mapImage`, `MAPPA.zoomImg` (vero = disegnata la regione nitida), intervalli fra `requestAnimationFrame` e long task, picco della memoria WASM dai messaggi del worker, RSS di Chromium da `/proc`.
- Lettura: `rdStat` di `rust-worker.js` (campo `rd` di ogni risposta) conta tempo, byte e byte da handle dentro i lettori; lo script li riporta (`first_read` cumulativo, `read_ms` per gesto). `rdStat.ops` (campo `rd.ops`) elenca le ultime 300 richieste come [op, percorso o nome, ms fino alla risposta, ms nei lettori, letture]: le richieste si intrecciano a ogni `await`, quindi è un profilo, non una misura esatta.
- Le operazioni di laboratorio (`lr_metodo`, `lr_method`, `lr_analog`, `lr_pda`...) leggono le intestazioni di centinaia di scansioni del mzML da un `SyncAccessHandle` (`rust-worker-lab.js` `open`), con ripiego su `FileReaderSync` se il handle è preso.
- Gesti: prima mappa a schermo intero (rotta, `JSON.parse`, base64, colore); 12 scatti di rotella; spostamento con Maiusc+trascinamento; zoom solo in RT (Maiusc+rotella); zoom profondo (2 min × 2 m/z); cambio di file freddo, ritorno e di nuovo caldo; 60 passi del mirino; 20 spostamenti (voci di `CACHE` e heap JS aggiunti).
- Confronto nativo: un piccolo binario fuori dal repository che apre l'`.mzidx` con `Index::open(File)` e chiama `Routes::map_route` con le stesse query (un giro freddo e uno caldo).

## Il percorso dei dati (cosa sta in memoria)
1. **Indice** `.mzidx` in OPFS: i picchi MS1 ordinati per m/z (mz, intensità, posizione; 12 B/picco in HR), con un recinto ogni 16384 picchi. Il worker lo legge da un `SyncAccessHandle` (`archivio.js` `lease`/`leaseRead`, `rust-worker.js` `readerFor`) in un buffer riusato, a pezzi di 65 536 picchi (`idxread.rs` `each_peak`). Il handle, esclusivo, si chiude dopo 2 s senza letture e prima di scrivere, spostare o cancellare il file, e `wake()` lo riapre a ogni richiesta; senza handle si ripiega su `FileReaderSync`.
2. **Mappa intera** (`api/map` senza finestra): la griglia della sessione (≤600 righe RT, celle di 1 Da anche in HR) costruita leggendo TUTTI i picchi del file; Rust tiene le ultime 6 per file (`ion_map`), la pagina le ricorda in `memo`.
3. **Regione** (`ionmap_region`, al massimo 2048 × 1024): legge tutti i picchi della banda m/z per tutto il tempo e scarta quelli fuori RT, quindi uno zoom solo in RT legge l'intero file.
4. **Trasporto**: `Vec<f32>` → base64 → testo JSON → `postMessage` → `Response` → `r.json()` → `MAPDATI.b64f32` → `Float32Array`: circa sei copie, 1,33 volte la dimensione.
5. **Colore** (`grafici.js` `mapImage`): `Float32Array`, 99,5° percentile da `MAPREND.ref` (`mappa-render.js`), tavolozza precalcolata scritta a 32 bit per pixel; l'immagine sta in un solo posto per pannello (`p._img`), quindi ogni cambio di file ricolora tutto.
6. **Regioni in memoria**: LRU a byte delle regioni (`REG`/`regionKeep` in `mappa.js`: 96 MB, 48 MB con `deviceMemory` ≤ 2) che toglie anche la voce di `CACHE`.
7. **Disegno**: `draw(p)` a ogni evento di input; il sovrapposto del mirino si ridisegna una volta per fotogramma (`frame`, `FRAME.next`); la regione nitida parte dopo un silenzio adattivo (`quiet`: 40 ms se l'ultima rotta è sotto 30 ms, altrimenti `QUIET` 150 ms).
11. **Piramide di tessere** (`maptiles.rs`, `api/maptiles`): livelli di massimi su una griglia agganciata, tessere sparse 256 × 256, uguali a `region_map` su griglie allineate (`tests/maptiles_parity.rs`); il worker la salva in OPFS (`mzlab/tiles/<file>.L<livello>.mzt`, `tilesEnsure` in `rust-worker.js`). Prova `e2e_hr_mappa`. La pagina non la chiede ancora per lo zoom né per il 3D.
8. **Mirino**: LRU proprio di spettri (64 MB, 32 su iPad) con lettura anticipata: è adeguato.
9. **3D** (`mappa3d.js`): la stessa rotta di regione fino a 1024², quindi eredita il punto 3.
10. **`.raw` Thermo** (lettore esterno): le scansioni lette dal modulo `thermo` (mappa, XIC fuori indice, scorrimento MS1/MS2) arrivano al motore come un solo `Uint8Array` (`lettori-modulo.md`, trasporto binario), senza meta JSON; la riga del `.raw` in questa tabella si misura sui dati veri con `python3 tools/bench_raw.py FILE.raw --steps scansione` (µs per scansione nel modulo).

## Misure (Chromium headless, container a 4 CPU, sito con motore Rust/WASM; riferimento: lettura con `FileReaderSync`, colore a funzione, `CACHE` senza limite)
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

Profilo HR (`hrp` 400 × 50 000, 2 × 320 MB, 20 M punti per file): nel browser l'apertura dei due file non è finita in 18 minuti (giro fermato, nessun gesto misurato); nativo, indice di 228 MB: mappa intera 506 ms (18 ms dalla cache), regione 100-1000 m/z su tutto il tempo 316-366 ms, regione profonda 30 ms. Il profilo è il caso in cui D1 pesa di più.

Nativo, stesso indice `hrc` (108 MB, file nella cache del sistema): mappa intera 254 ms a freddo e 14 ms dalla cache; regione 100-1000 m/z su tutto il tempo 212-273 ms; regione profonda 2 min × 2 m/z 29-45 ms. Nel browser le stesse rotte prendono 5830 ms e 1465 ms: il calcolo è lo stesso codice, il resto è lettura.

Colore della mappa intera HRC (600 × 901, Node): algoritmo a pixel con funzione 314 ms; con `Float32Array` ordinato, scala precalcolata in `Uint32Array` e scrittura a 32 bit (quello in uso) 52 ms.

## Diagnosi (in ordine di peso)
- **D1 Lettura dell'indice nel browser**: con `FileReaderSync` su `Blob.slice` ogni lettura è una copia attraverso il processo del browser (mappa intera HR 9 M picchi: 5,8 s contro 0,25 s nativi); il worker legge dal `SyncAccessHandle` in blocchi grandi («Il percorso dei dati», punto 1) e il calcolo WASM resta al più 2 volte il nativo.
- **D2 Zoom solo in RT**: la regione legge tutta la banda m/z per tutto il tempo (1,5 s in HR); il recinto ogni 16384 picchi serve solo all'm/z. La piramide (punto 11 del percorso) lo toglie quando la pagina la usa.
- **D3 Colore**: 52 ms per la mappa intera HRC, ma rifatto a ogni cambio di file (un solo `p._img`).
- **D5 Frame lunghi durante il gesto**: p95 33-50 ms; un `draw` per evento e lavoro del sovrapposto (ridimensiona la tela, legenda, tabella) a ogni disegno.
- **D7 Trasporto testuale**: sei copie e +33 %; pochi ms di tempo ma memoria doppia nei picchi.
- **D8 Picco WASM 175 MB in HR** con tetto 256 e bilancio 192 (`memoria.md`): su iPad è il limite più vicino.
- **D9 3D**: come D2 per finestre lunghe in RT.
- **D10 Mirino**: 10-35 ms per spettro in HR con LRU e lettura anticipata: non è un collo di bottiglia.

## Obiettivi
- Frame durante zoom e spostamento: p95 ≤ 17 ms (60 fps), anche in HR.
- Nitido dopo il gesto: ≤ 120 ms in LR, ≤ 200 ms in HR; mai più di 300 ms, anche solo in RT.
- Prima mappa ≤ 1 s per un file HR tipico (~10 M picchi MS1); cambio di file caldo ≤ 50 ms, freddo ≤ 300 ms.
- Mirino: spettro percepito < 16 ms dalla cache, p95 < 35 ms.
- Picco WASM ≤ 128 MB in HR; memoria della pagina limitata da un bilancio in byte, non dal numero di gesti.

## Strade scartate
- OffscreenCanvas nel worker: con il colore a 32 bit il colore costa poco.
- Decimazione min-max: utile per le tracce 1D, non per una griglia già al massimo.
- SharedArrayBuffer: richiede COOP/COEP, che GitHub Pages non manda.
- Tetto dei `.wasm` più alto: vietato; la lettura a handle e la piramide aggiungono poco codice al motore.

## Lavori aperti
Ogni lavoro si consegna con la sua misura `bench_map.py` prima e dopo nella descrizione della PR.
- **P2b Piramide nella pagina** (D2): lo zoom e lo spostamento chiedono `api/maptiles` al livello della vista; la regione esatta resta per media, fondo, differenza e zoom più profondo. Accettazione: zoom solo RT HRC ≤ 0,2 s; `e2e_mappa*` verdi.
- **P3 Immagini per file** (D3): LRU di immagini per (file, scala, livello) con un bilancio in byte (es. 64 MB, 32 su iPad) al posto del solo `p._img`. File: `grafici.js` (`mapImage`), `mappa.js`. Accettazione: cambio di file caldo HRC ≤ 50 ms, pixel uguali.
- **P4 Trasporto binario** (D7): il worker restituisce JSON piccolo + `ArrayBuffer` trasferito, la pagina costruisce il `Float32Array` senza base64; per `api/map`, la regione e il 3D. File: `crates/mzlab-wasm`, `rust-worker.js`, `rust-bridge.js`, `browser.js`, `explore.js`, `mappa.js`, `mappa3d.js`; il JSON con base64 resta per l'app desktop e i test di parità. Rischio medio (contratto fra worker e pagina; `MZLAB_LR_MISS` ed e2e delle rotte). Dopo P3.
- **P6 Un disegno per frame** (D5): `draw(p)` della mappa in coda a un solo `requestAnimationFrame`; `MAPPA.after` ridimensiona la tela solo se cambia la misura e riscrive legenda e tabella solo se cambiano i dati. File: `mappa.js`, `grafici.js` (`drawMap`). Accettazione: frame p95 ≤ 17 ms in rotella e spostamento, LR e HRC; `e2e_tour` verde.
- **P7 Colore su GPU** (solo se dopo P3-P6 i frame non sono a 60 fps su griglie 2048 × 1024): celle in una texture float, scala come texture 1D, ripiego sul colore in JS senza WebGL; rischio medio-alto (CSP, contesti persi su iPad, stessa resa dei colori).
- **P8 App desktop**: `memmap2` per l'`.mzidx` e `rayon` per costruire le tessere su più file (`crates/mzlab-desktop`, feature di `mzlab-core` che il `.wasm` non compila). Accettazione: piramide di un file HR ≤ 0,5 s nativa.
- **P9 3D sulla piramide** (D9): `mappa3d.js` chiede le tessere al livello della sua griglia invece di `api/map`. Accettazione: apertura del 3D su HRC ≤ 0,3 s; `e2e_mappa3d`, `e2e_mappa_nav3d`, `e2e_mappa3d_errori` verdi.
- **P10 Bilanci di memoria per iPad** (D8): picco WASM ≤ 128 MB; la cache delle mappe intere (`mapcells.rs`, 6 griglie per file) entra nel bilancio di `mzlab-core::budget`; misura su iPad reale (`memoria.md`).

## Limiti dello studio
- Misure su un container Linux headless a 4 CPU; iPad e Safari non sono misurati (la lettura a handle e P10 chiedono una prova reale).

## Visore 3D della mappa (`mappa3d.js`)
Visore della vista Dati: errori di apertura, comandi, precarico, costo del disegno. La mappa 2D è in `dati-mappa.md`.

- **Se il 3D non si apre** (`failure3d` in `mappa.js`, `Fail`/`makeContext`/`diagnose` in `mappa3d.js`; prova `e2e_mappa3d_errori`): la finestra dice la causa vera — `nowebgl2` (nessun contesto, provati opzioni del visore, senza opzioni, low-power; WebGL1 solo per dirlo), `shader` (riga del compilatore), `import` (file non scaricato o vecchio in cache), `error` (errore JavaScript, mai spacciato per WebGL2),
  `lost` (contesto perso, messaggio nel visore) — con cosa fare in Chrome e le righe tecniche copiabili (versione, renderer, GPU, attributi, `getError`); la 2D resta intatta e il contesto di prova si rilascia subito.
- **Visore 3D** (`mappa3d.js`, modulo ES caricato al primo «3D», WebGL2 scritto a mano, nessuna libreria): finestra di dialogo modale sopra la pagina (Esc chiude, il fuoco torna al pulsante) con il file e la regione della 2D (nel telefono non c'è; senza WebGL2 un messaggio).
  Barra laterale ritraibile: Dati (file, confronto A − B, RT e m/z numerici, «Usa lo zoom della 2D»), Rappresentazione (superficie | aste per l'HR | cascata di spettri | curve di livello sul pavimento; altezza lineare (predefinita)/radice/log, esagerazione verticale, proporzione RT:m/z, palette, luce, mappa 2D sul pavimento, piano del tempo), Fondo (soglia con istogramma, «togli il fondo», smussatura: 0 in HR, 1 in LR), Viste (dall'alto, lato RT = cromatogramma, lato m/z = spettro, isometrica, ripristina, PNG ad alta risoluzione).
  Griglia fino a 1024×1024 (512 su telefono, iPad o `deviceMemory` ≤ 4), HR in celle di ppm uguali (`mzscale=log`), LR a masse nominali; aste dai punti di `/api/mappunti3d` (200 000, 50 000 sui dispositivi piccoli).
  Trascina = ruota, Maiusc+trascina o due dita = sposta, rotella o pizzico = zoom, doppio clic = centra (su un punto segnato: pannello spettro); al passaggio il raggio dal pavimento alla superficie (intersezione sulla griglia delle altezze, CPU) dà RT, m/z, intensità e due riquadri (XIC a quel m/z, spettro a quell'RT, dalla griglia); clic = fissa il punto (come nella 2D lo aggiunge a `NB.mappaPunti` solo con «Segna» attivo); frecce = cursore di un bin (e piano del tempo), +/− zoom, 1-4 viste, R ripristina (anche altezza e regione), L blocca il cursore, ? gesti;
  gesti e menu: «Navigazione della mappa» di `dati-mappa.md`; `prefers-reduced-motion`: niente ▶ né inerzia. Scelte del visore in `qqq.mappa` (`d3`); le chiavi `t3*` vecchie si ignorano. Solo misure: nessuna etichetta automatica. Test `e2e_mappa3d`, `e2e_pannelli2`.
- **Visore 3D** (stessi comandi, stesso `menu()`, stessa barra `MAPPA.barFor` in alto a destra di `.m3d-main`; test `e2e_mappa_nav3d`): adattatore `ad3(hit)` in `mappa3d.js`; il punto è il cursore bloccato (`V.lock`: il passaggio del puntatore non lo muove, la lettura dice «locked»), altrimenti la cella sotto il puntatore al clic destro (raggio `pick`) o il cursore.
  Clic destro senza spostarsi o pressione lunga (550 ms) o Menu/Maiusc+F10 = menu (il tasto destro trascinato sposta la vista); il `#ctx` della pagina viene spostato dentro il dialogo finché è aperto (Esc chiude prima il menu, poi il visore) e ritorna nel body alla chiusura.
  Voci: Estrai XIC/spettro/entrambi (il visore si chiude e l'estratto compare nei pannelli XIC e spettro della pagina; il file è quello scelto nel visore), Blocca/Sblocca, Reset zoom = camera (35°, 32°), Vista dall'alto / laterale RT / laterale m/z (tasti 1-3), Copia coordinate, Gesti (foglio `mapnav.h.3d.*`). Un dito ruota, due dita spostano e pizzicano, rotella = zoom, + / − zoom. La paletta Ctrl/⌘K non si apre sopra il dialogo modale: i comandi del 3D stanno nel menu e nella barra.
- **Precarico del 3D**: alla prima `init` di una mappa 2D (`preload3d` in `mappa.js`) `static/mappa3d.js` si scarica in sottofondo (`modulepreload` in idle, non con `saveData` né 2g/3g), così `open3d` lo trova già pronto.
- **Costo del disegno 3D**: la cascata (`fall`) disegna tutte le righe con due `drawArraysInstanced` (la riga è `uRow + gl_InstanceID * uStep` in `ROW_VS`); la tavolozza `pal` si carica solo quando cambia (`V.palTab`); il puntamento (`pick`) col mouse libero gira al più una volta per fotogramma (`hoverPick`).
