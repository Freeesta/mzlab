# Misure dei motori
Chromium Linux, sito costruito con il motore Rust, stessa macchina, un giro. «open» = caricamento + primo TIC disegnato; «TIC+XIC» = TIC e 5 XIC di ogni file. Con `?motore=rust` il motore Python resta caricato e legge comunque i file (Rust risponde solo a TIC, XIC e scansioni): la memoria è la somma dei due.
Le righe «motore Python» (Pyodide nelle pagine degli studenti) sono il termine di confronto: nessuno strumento di `tools/` le rigenera.
Memoria dopo l'apertura dei file e le richieste di TIC e XIC: heap WASM = somma delle `WebAssembly.Memory` del worker; IndexedDB = byte dei file salvati; heap JS della pagina da `performance.memory` (nei worker non c'è); RSS = somma dei processi Chromium. Rifare i tempi: `python3 tools/build_site.py`, costruire il Rust come in `pages.yml`, poi `python3 tools/bench.py --files 6` (file LR: `MZLAB_MZML`; Chromium: `MZLAB_CHROMIUM`).

## Tempi (6 file Full Scan, 31.9 MB)
| engine | start_s | open_s | tic_xic_s | requests | browser_rss_mb |
|---|---|---|---|---|---|
| python | 3.54 | 3.35 | 0.24 | 36 | 1072.5 |
| rust | 3.58 | 4.24 | 0.14 | 36 | 1089.7 |

## Ripartizione della memoria (MB)
| set | engine | file_mb | pyodide_wasm_mb | rust_wasm_mb | page_js_heap_mb | indexeddb_mb | browser_rss_mb |
|---|---|---|---|---|---|---|---|
| 6 Full Scan LR | python | 31.9 | 107.9 |  | 10.7 | 31.9 | 1095.7 |
| 6 Full Scan LR | rust | 31.9 | 107.9 | 32.0 | 10.5 | 31.9 | 1112.3 |
| 4 HRMS | python | 27.1 | 107.9 |  | 9.9 | 27.1 | 1097.3 |
| 4 HRMS | rust | 27.1 | 107.9 | 32.0 | 10.5 | 27.1 | 1114.4 |

Empty page (about:blank), same browser: RSS 679.4 MB.

## Molti file (motore Python)
Apertura = invio dei file, lettura mzML e primo disegno; riapertura = nuova pagina nello stesso profilo (IndexedDB), fino al primo disegno (HR: fino alla lista dei file, poi «Apri»). I passi sono i secondi tra i messaggi del worker; «ready» include la rilettura da IndexedDB e l'avvio di `mzlab.browser`.
Files: lab data (copies renamed when the folder has fewer files).

| scenario | files | file_mb | engine_start_s | read_mzml_s | first_draw_s | tic_xic_s | idb_mb |
|---|---|---|---|---|---|---|---|
| 20 Full Scan LR | 20 | 105.8 | 5.62 | 7.9 | 6.95 | 1.47 | 105.8 |
| 9 HR DDA | 9 | 339.8 | 6.32 | 35.37 | 25.28 | 1.08 | 339.8 |

| scenario | reopen_files_listed_s | reopen_first_draw_s | reopen_files | pyodide_wasm_mb | rss_peak_mb |
|---|---|---|---|---|---|
| 20 Full Scan LR | 21.73 | 21.89 | 20 | 223.9 | 1440.0 |
| 9 HR DDA | 32.66 | 56.49 | 9 | 696.3 | 2682.4 |

Reopening, seconds spent between worker messages:
| scenario | reopen_step[Loading Python...]_s | reopen_step[Loading numpy...]_s | reopen_step[Loading the program...]_s | reopen_step[Reopening the files of the last visit...]_s | reopen_step[ready]_s |
|---|---|---|---|---|---|
| 20 Full Scan LR | 0.1 | 4.2 | 0.84 | 0.03 | 4.5 |
| 9 HR DDA | 0.1 | 3.88 | 0.93 | 0.01 | 2.95 |

## Librerie spettrali (`python3 tools/bench.py --librerie --gb --scrivi`, Chromium Linux con profilo persistente)
«js» = percorso JavaScript (`?libjs=1`, fino a 10^5 spettri), «wasm» = motore Rust/WASM. Tempi misurati dalla pagina (comprendono il passaggio dei messaggi al worker); identità e analoghi = mediana di 30 e 10 richieste a caldo, «cold» = la prima; «identify_all» = 5000 MS2 (400 per MassBank, che ne ha meno). Memoria = picco RSS di tutto il browser meno la base prima dell'importazione. Spettri sintetici (8-40 picchi uniformi: comprimono poco: caso sfavorevole per il peso).
Bersagli: identità < 5 ms a caldo, analoghi < 100 ms su 10^6, MGF da 1 GB < 90 s con ≤ 200 MB. Browser: identità 8-12 ms (giro messaggi e JSON; <1 ms nativo), analoghi 136 ms su 10^6, MGF da 1 GB 130 s e 331 MB (74 s nativo con 110 MB). MassBank su disco: 8,0 MB contro 15,7 MB MSP. Gestione librerie `.mzlib` v1: indici Flash (frammenti e perdite neutre), ricerca a caldo 8-12 ms, analoghi ~136 ms su 10^6 spettri, supporto decoy e stima FDR.

| library | engine | spectra | file_mb | disk_mb | import_s | ident_cold_ms | ident_ms | analogs_cold_ms | analogs_ms | identify_all_s | identify_all_named | rss_peak_import_delta_mb | rss_peak_mb |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| synthetic 1e5 | js | 100000 | 48.6 | 24.8 | 3.0 | 154 | 8.7 | 423.1 | 388.5 | 0.4 | 5000 | 320.0 | 1473.8 |
| synthetic 1e5 | wasm | 100000 | 48.6 | 54.2 | 5.1 | 119.2 | 11 | 165.8 | 76.3 | 2.2 | 5000 | 150.0 | 1375.4 |
| synthetic 1e6 | wasm | 1000000 | 486.9 | 541.7 | 53.4 | 118.8 | 12.4 | 782.9 | 136.4 | 19.9 | 5000 | 251.0 | 1407.6 |
| MassBank (data repository) | js | 17290 | 15.7 | 5.4 | 0.7 | 54.8 | 8.9 | 205.6 | 165.3 | 0.1 | 400 | 123.0 | 1157.6 |
| MassBank (data repository) | wasm | 17002 | 15.7 | 8.0 | 1.0 | 52.3 | 8.3 | 86 | 19.2 | 0.1 | 400 | 54.0 | 1054.6 |
| synthetic MGF 1.04 GB | wasm | 2310000 | 989.2 | 1253.2 | 130.5 | 127.9 | 11.6 | 1703.5 | 148.4 |  |  | 331.0 | 1404.8 |

Native builder (cargo example build; the memory is the peak RSS of the process):
| library | engine | spectra | file_mb | disk_mb | import_s | rss_peak_mb |
|---|---|---|---|---|---|---|
| sint_100000.msp | native | 100000 | 51.0 | 56.8 | 2.9 | 18 |
| sint_1000000.msp | native | 1000000 | 510.5 | 568.0 | 30.2 | 50 |
| massbank_eu_qft_pos.msp | native | 17002 | 16.5 | 8.4 | 0.5 | 9 |
| sint_1gb.mgf | native | 2310000 | 1037.2 | 1314.1 | 74.4 | 110 |

## Match di tabelle di feature (`UNT_BENCH=1 python3 tests_e2e/e2e_untarget.py`, Chromium Linux, motore WebAssembly)
5000 feature con MS2 (12 picchi) cercate in una libreria sintetica di 20 000 voci, polarità positiva, 10 ppm sul precursore: lettura della tabella e dell'MGF 0,1 s, ricerca e livelli 0,7 s (soglia 15 s, un solo giro). Con una libreria di 10^6 voci il costo è quello di «Identifica tutte le MS2» sopra (5000 richieste in circa 20 s). La lettura di un `merged_pos.csv` vero da 58 MB (249 colonne) non è misurata qui: si fa solo con `MZLAB_UNTARGET`.

## Indice `.mzidx` (Chromium Linux con profilo persistente)
Sito con `.mzidx` in OPFS, una macchina. Le colonne «motore Python» (Pyodide) sono un giro e il termine di confronto: 20 Full Scan LR (2,4 MB), 15 HR DDA (78 MB, 15.800 scansioni). La colonna «HR, motore Rust» è la mediana di 3 giri di `tests_e2e/misura_pack.py` (4 CPU) su 15 HR DDA sintetici Orbitrap (61,3 MB, 11.880 scansioni, 30 voci: MS1 e MS2): per MB, 0,41 s/MB contro 2,14 s/MB del motore Python. Apertura = invio, lettura, scrittura OPFS e indice; primo disegno = fino al TIC; riapertura = nuova pagina nello stesso profilo.

| misura | 20 Full Scan LR (motore Python) | 15 HR DDA (motore Python) | 15 HR DDA, motore Rust |
|---|---|---|---|
| apertura (invio + lettura + indice), s | 8.65 | 166.62 | 24.84 |
| primo disegno, s | 1.38 | 12.98 | 0.28 (clic su un file, file già indicizzati) |
| TIC + 5 XIC di ogni file, s | 1.47 | 6.04 | 2.17 |
| mappa di tutti i file, s | 1.1 | 5.64 | – |
| riapertura fino al primo disegno, s | 8.16 | 24.27 | 3.4 |
| mappa di tutti i file dopo la riapertura, s | 1.19 | 7.49 | – |
| spettro (mediana / max), ms | 2.5 / 9.6 | 7.5 / 21.8 | – |
| XIC su tutti i file (mediana / max), ms | 45 / 66.1 | 500.3 / 551.7 | 33.0 / 53.3 |
| cambio file, TIC (mediana / max), ms | 5.7 / 14.1 | 73.9 / 109.7 | 9.9 / – |
| heap WASM dopo il lavoro, MB | 155.5 | 570.9 | 32.0 (solo il worker del motore; i worker di indicizzazione non sono visti) |
| heap WASM dopo la riapertura, MB | 129.6 | 570.9 | – |
| RSS massima del browser, MB | 831.3 | 1355.3 | 1688.4 |

## Esporta / importa i miei dati (`python3 tests_e2e/misura_pack.py --parts export`, Chromium Linux, sito costruito, mediana di 3 giri)
`esporta-dati-worker.js` istanzia l'intero `.wasm` del motore (3,6 MB, 1,17 MB gzip) solo per `zip` / `unzip`. «Avvio» = dal `new Worker` alla prima risposta a uno `zip` di una voce. Il server di prova non comprime. ZIP di 50 MB in OPFS (5 file in `librerie/`, byte casuali) con il vero `esporta-dati.js`.

| misura | ms |
|---|---|
| avvio a freddo (browser nuovo, cache HTTP vuota, localhost) | 52 |
| avvio a freddo su 10 Mbit/s e 40 ms (non compresso) | 3234 |
| avvio a caldo (cache HTTP e modulo compilato) | 40 |
| avvio nella pagina con il motore già pronto (prima chiamata / seguenti) | 80 / 75 |
| esportazione di 50 MB: lettura OPFS (`collect`) | 104 |
| esportazione di 50 MB: `pack` | 34 |
| esportazione di 50 MB: `zip` nel worker (avvio incluso) | 421 |
| esportazione di 50 MB: Blob e indirizzo di scarico | 46 |
| esportazione di 50 MB: totale | 613 |
| importazione: `unzip` nel worker (50 MB) | 475 |

## Disegno dei grafici sul thread principale (`python3 tests_e2e/misura_pack.py --parts draw`, Chromium Linux, sito costruito, mediana di 3 giri)
Dall'azione a due fotogrammi dipinti; attività lunghe = `longtask` (> 50 ms) iniziate nella finestra; fotogrammi lunghi = `long-animation-frame` con lo script più lungo. CPU 1x e rallentata 4x (CDP). Mappa: un Full Scan sintetico di 1500 scansioni × 500 centroidi a 1920×1080, «f» = schermo intero (canvas 1604 → 1698 px). TIC e XIC: 20 Full Scan sintetici. Grafo MSn: `msngrafo_80.json` (80 ioni, SVG, non canvas) aperto da `#pick`.

| azione | CPU | ms | attività lunghe (n / massima ms) | fotogrammi lunghi (n / massimo ms) | script |
|---|---|---|---|---|---|
| nuovo pannello TIC, 20 file | 1x / 4x | 30 / 139 | 0 / 1 (59) | 0 / 0 | – |
| ridimensiona la finestra, 20 file | 1x / 4x | 66 / 339 | 0 / 2 (153) | 0 / 2 (172) | `pannelli.js` (rAF) |
| XIC da un clic, 20 file | 1x / 4x | 81 / 312 | 0 / 2 (84) | 0 / 1 (138) | `pannelli.js` (rAF) |
| mappa: apertura | 1x / 4x | 282 / 615 | 0 / 3 (166) | 0 / 2 (106) | `eventi.js` (clic) |
| mappa: schermo intero | 1x / 4x | 223 / 396 | 1 (51) / 3 (146) | 1 (63) / 1 (127) | `pannelli.js` (rAF) |
| mappa: ridimensiona la finestra | 1x / 4x | 119 / 545 | 1 (60) / 2 (255) | 0 / 2 (145) | `pannelli.js` (rAF) |
| grafo MSn, 80 ioni | 1x / 4x | 261 / 487 | 0 / 1 (107) | 0 / 1 (166) | `infusione-grafo.js` (caricamento) |

## Costruttore del `.mzidx` (Chromium Linux 4 CPU)
«python» = costruttore Python in Pyodide (`?pool=0`, il riferimento), «pool» = worker Rust (predefinito). Tempo = dall'invio dei file a tutti nell'elenco (l'indice di ciascuno è costruito nel frattempo); memoria = somma dei processi Chromium (massimo campionato ogni 0,25 s); `digest` = hash di TIC e 5 XIC di ogni file; riapertura = nuova pagina nello stesso profilo (le differenze tra le due righe sono rumore). File: esempi anonimi del sito (LR: Full Scan; HR: ritaglio Orbitrap ripetuto fino a 140 MB). Un giro per scenario.

| scenario | constructor | files | file_mb | build_total_s | rss_peak_build_mb | first_draw_s | reopen_first_draw_s | rust_passes_max | digest |
|---|---|---|---|---|---|---|---|---|---|
| 20 Full Scan LR | python | 20 | 107.3 | 15.39 | 747.9 | 0.26 | 5.96 | 0 | 35255d6425e8 |
| 20 Full Scan LR | pool | 20 | 107.3 | 9.51 | 749.5 | 0.34 | 7.91 | 1 | 35255d6425e8 |
| 15 HR DDA (140 MB each) | python | 15 | 2101.0 | 299.63 | 1010.2 | 1.85 | 12.19 | 0 | 77a5810c63d1 |
| 15 HR DDA (140 MB each) | pool | 15 | 2101.0 | 252.59 | 1321.9 | 1.34 | 11.61 | 3 | 77a5810c63d1 |

Lettore mzML nativo (`.wasm` del motore 1.703.705 → 1.703.558 B; `cargo run --release -p mzlab-core --example carico -- FILE.mzML` e `--example mzidx -- FILE.mzML OUT.mzidx`, 4 CPU, un file Orbitrap sintetico di 101 MB con gli spettri ripetuti 25 volte, 19.800 scansioni e 2,55 M di picchi; mediana di 3 giri): lettura e somma del TIC 1,34 s → 0,85 s; costruzione del `.mzidx` (`build`) 1,74 s → 1,16 s. `.mzidx` identico con `cmp` su 37 file sintetici e sul file grande, TIC identico.

## Schermata smartphone (iPhone 390×844, touch, server di sviluppo, mediana di 5)
| | valore |
|---|---|
| primo disegno (FCP), ms | 88 |
| byte scaricati fino a rete ferma | 330 178 (comprende ~85 KB di Teoria scaldata) |

## Rust legge l'indice (`python3 tools/bench.py --files 6 --json`, Chromium cloud con WASM, 6 Full Scan sintetici, un giro)
Il motore Rust risponde dall'indice `.mzidx` (TIC, XIC, filtri, elenco scansioni, scansione più vicina) e apre l'mzML solo per le scansioni singole.

| misura | Rust | Python (stesso giro) |
|---|---|---|
| avvio (pagina + motore), s | 5.38 | 4.37 |
| apertura dei file fino al primo TIC, s | 2.27 | 2.24 |
| TIC + 5 XIC di ogni file, s | 0.11 | 0.35 | 2.17 |
| RSS del browser, MB | 1033 | 1073 |

## Teoria da telefono (`tests_e2e/e2e_teoria_mobile.py`, `e2e_indirizzi.py`)
Chromium Linux, 390×844 con tocco, server di sviluppo, un giro, dati stampati come `MEASURE`: capitolo 4 ≈ 45 kB e LCP ≈ 0,18 s, capitolo 20 ≈ 140 kB (le due raster ridotte da 1400 px, 88 + 57 kB, caricate «lazy»; i file grandi da 384 + 182 kB solo all'apertura della figura), LCP ≈ 0,07 s. Apertura a freddo di `#/theory` 11 richieste ≈ 160 kB, senza Pyodide, WASM, `api/` né script di Dati o Disegno (verificato). Rifare la misura: lanciare i due script (stampano le righe `MEASURE`).

## Mirino della mappa (`python3 tools/bench_mirino.py [lr|hr] [--site]`, Chromium Linux)
Mappa a schermo intero con il mirino acceso. LR: un Full Scan sintetico di 1500 scansioni × 500 centroidi (750 000 picchi); HR: le scansioni Full Scan di un Orbitrap sintetico (poche, ~600 scansioni). «site» = sito costruito (Pyodide in un worker; `python_ms` = tempo in Python dalla intestazione `X-Py-Ms`, il resto è andata e ritorno e copie JS↔worker); «dev» = server di sviluppo. Un giro; tempi in ms, mediana / p95.

| misura | valore |
|---|---|
| spettro di un punto: Python, LR (mediana) | 3.2 (`/api/spectra`: legge la scansione dal file) |
| spettro di un punto: totale, LR / HR | 4.1 / 2.2 |
| richieste, spazzata lenta, LR / HR | 38 / 36 |
| attesa dal punto al disegno esatto, spazzata, LR / HR (mediana) | 24 / 23 |
| attesa p95, spazzata, LR / HR | 29 / 30 |
| aggiornamenti esatti durante il trascinamento, LR / HR (120 eventi) | 68 / 90 (più 132 / 106 anteprime) |
| 40 frecce premute di seguito, LR: richieste / aggiornamenti esatti / attesa | 33 / 40 / 21 |
| fotogramma durante il trascinamento (mediana / p95 / max) | 16.7 / 33.4 / 50.0 (LR), 16.7 / 16.8 / 16.8 (HR) |
| lavoro del mirino per fotogramma (mediana / p95 / max), dev | 0.8 / 1.4 / 4.2 |
| attività lunghe (> 50 ms) durante il trascinamento | 0 |

## Transizioni (`transizioni.css`, `transizioni.js`)
Peso: 0,9 KB gzip il CSS + 2,2 KB gzip il JS (3,1 KB in tutto, soglia 4 KB). Apertura a freddo sul server di sviluppo (mediana di 7, `loadEventEnd`): 461 ms con, 456 ms senza (nessuna regressione). Durante apertura/chiusura del pannello a sinistra, a destra e come foglio: 0 *long animation frame* (`e2e_transizioni` li stampa); un solo `fitWidth` e un solo giro di `draw` per transizione.

## Misure Rust e dei moduli
`docs/agenti/misure-rust.md`: flusso Rust senza Pyodide, famiglie di ioni, `api/origin`, mappa e punti, composizione, spettri in profilo, rilevatori di picchi, HR/DDA, rilevamento feature, gesti della mappa, moduli lettori, profilo `desktop`, tabella e `.mzidx`, Dig LR, interpretazione TP, `.wasm` interp.
