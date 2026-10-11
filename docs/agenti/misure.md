# Misure dei motori (`python3 tools/bench.py --memoria --scrivi`)
Chromium Linux, sito costruito con il motore Rust, 2026-10-10, commit eb6c6f1, stessa macchina, un giro. «open» = caricamento + primo TIC disegnato; «TIC+XIC» = TIC e 5 XIC di ogni file. Con `?motore=rust` il motore Python resta caricato e legge comunque i file (Rust risponde solo a TIC, XIC e scansioni): la memoria è la somma dei due. Memoria dopo l'apertura dei file e le richieste di TIC e XIC: heap WASM = somma delle `WebAssembly.Memory` del worker; IndexedDB = byte dei file salvati; heap JS della pagina da `performance.memory` (nei worker non c'è); RSS = somma dei processi Chromium. Rifare la misura: `python3 tools/build_site.py`, costruire il Rust come in `pages.yml`, poi `python3 tools/bench.py --memoria --scrivi` (file LR: `MZLAB_MZML`; Chromium: `MZLAB_CHROMIUM`).

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

## Molti file (`python3 tools/bench.py --velocita --scrivi`, motore Python, 2026-10-10, commit 8e3160a)
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

Avvio del motore, prove senza guadagno (non tenute): rilettura di tutti i file da IndexedDB in una sola transazione (riapertura di 20 file LR 18,8–21,5 s prima, 19,7–21,2 s dopo: dentro il rumore, il passo «ready» resta 3,7–4,4 s). Il precarico in parallelo di `numpy` e `mzlab.zip` c'è già (`browser-worker.js`); `numpy` pesa ~3,5–4,3 s e serve a ogni lettura, quindi non si può rimandare. L'istantanea della memoria di Pyodide non è stabile nella versione fissata: no. Dopo `ready` restano ~12 s (LR) per rileggere e rielaborare i file: è il lavoro della cache dei risultati (pacchetto MZIDX).

## Librerie spettrali (`python3 tools/bench.py --librerie --gb --scrivi`, Chromium Linux con profilo persistente, 2026-10-11, commit 5d49ecf)
«js» = percorso JavaScript di prima (`?libjs=1`, solo fino a 10^5 spettri: oltre non è pratico), «wasm» = motore nuovo. Tempi misurati dalla pagina (comprendono il passaggio dei messaggi al worker); identità e analoghi = mediana di 30 e 10 richieste a caldo, «cold» = la prima; «identify_all» = 5000 MS2 (400 per MassBank, che ne ha meno). Memoria = picco RSS di tutto il browser meno la base prima dell'importazione. Spettri sintetici (8-40 picchi uniformi: comprimono poco, il peso su disco è quindi un caso sfavorevole). Bersagli: identità < 5 ms a caldo, analoghi < 100 ms su 10^6, MGF da 1 GB < 90 s con ≤ 200 MB. Non raggiunti nel browser: identità 8-12 ms (la maggior parte è il giro dei messaggi e del JSON; il calcolo nativo è sotto il millisecondo), analoghi 136 ms su 10^6, MGF da 1 GB 130 s e 331 MB; il costruttore nativo (stesso codice) importa lo stesso MGF in 74 s con 110 MB. Il peso su disco della MassBank è 8,0 MB contro 15,7 MB di MSP (i due indici Flash non sono compressi).

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

## Match di tabelle di feature (`UNT_BENCH=1 python3 tests_e2e/e2e_untarget.py`, Chromium Linux, motore WebAssembly, 2026-10-11)
5000 feature con MS2 (12 picchi) cercate in una libreria sintetica di 20 000 voci, polarità positiva, 10 ppm sul precursore: lettura della tabella e dell'MGF 0,1 s, ricerca e livelli 0,7 s (soglia 15 s, un solo giro). Con una libreria di 10^6 voci il costo è quello di «Identifica tutte le MS2» sopra (5000 richieste in circa 20 s). La lettura di un `merged_pos.csv` vero da 58 MB (249 colonne) non è misurata qui: si fa solo con `MZLAB_UNTARGET`.

## Indice `.mzidx` (`python3 tools/bench.py --mzidx [--solo lr|hr] [--site DIR]`, motore Python, Chromium Linux con profilo persistente, 2026-10-11)
«Prima» = il sito costruito dal commit precedente al punto 2 (tabella dei picchi in memoria, file in IndexedDB/WORKERFS), «dopo» = sito con `.mzidx` in OPFS; stessi file, stessa macchina, un giro. File: sintetici (nessun dato del laboratorio nel contenitore): 20 Full Scan LR di 2,4 MB; 15 file HR DDA di 78 MB ottenuti ripetendo 20 volte il file Orbitrap sintetico (`inflate`: ~15.800 scansioni e 2 M di picchi in MS1 ciascuno, più scansioni e meno picchi per MB di un file vero). Apertura = invio dei file, lettura, scrittura in OPFS e costruzione dell'indice di ognuno; primo disegno = clic su «Apri» fino al TIC; riapertura = nuova pagina nello stesso profilo, fino al primo disegno. Tempi di risposta su 3 × N spettri (`api/scanbin`), 5 m/z × 2 giri di XIC su tutti i file insieme, 2N richieste di TIC (cambio file). La mappa è `api/map` di ogni file sulla griglia di sessione.

**20 Full Scan LR (sintetici, 2,4 MB l'uno)**

| misura | prima | dopo |
|---|---|---|
| apertura (invio + lettura + indice), s | 4.28 | 8.65 |
| primo disegno, s | 3.92 | 1.38 |
| TIC + 5 XIC di ogni file, s | 1.02 | 1.47 |
| mappa di tutti i file, s | 1.06 | 1.1 |
| riapertura fino al primo disegno, s | 11.35 | 8.16 |
| mappa di tutti i file dopo la riapertura, s | 1.15 | 1.19 |
| spettro (mediana / max), ms | 2.2 / 7.1 | 2.5 / 9.6 |
| XIC su tutti i file (mediana / max), ms | 68.5 / 112.9 | 45 / 66.1 |
| cambio file, TIC (mediana / max), ms | 5.2 / 8.4 | 5.7 / 14.1 |
| heap WASM dopo il lavoro, MB | 155.5 | 155.5 |
| heap WASM dopo la riapertura, MB | 155.5 | 129.6 |
| RSS massima del browser, MB | 861.9 | 831.3 |

**15 HR DDA da 78 MB (sintetici)**

| misura | prima | dopo |
|---|---|---|
| apertura (invio + lettura + indice), s | 139.41 | 166.62 |
| primo disegno, s | 127.86 | 12.98 |
| TIC + 5 XIC di ogni file, s | 6.95 | 6.04 |
| mappa di tutti i file, s | 23.46 | 5.64 |
| riapertura fino al primo disegno, s | 252.52 | 24.27 |
| mappa di tutti i file dopo la riapertura, s | 23.15 | 7.49 |
| spettro (mediana / max), ms | 10.9 / 38.3 | 7.5 / 21.8 |
| XIC su tutti i file (mediana / max), ms | 636 / 675.3 | 500.3 / 551.7 |
| cambio file, TIC (mediana / max), ms | 74.6 / 107 | 73.9 / 109.7 |
| heap WASM dopo il lavoro, MB | 1011.4 | 570.9 |
| heap WASM dopo la riapertura, MB | 1011.7 | 570.9 |
| RSS massima del browser, MB | 1701.2 | 1355.3 |

Letture: la riapertura dei file HR passa da 253 s a 24 s e il primo disegno da 128 s a 13 s (l'indice si costruisce durante l'invio, che dura il 20% in più; apertura + primo disegno 267 s → 180 s); i picchi non stanno più nella memoria del motore (heap WASM 1011 → 571 MB: resta la tabella delle scansioni, ~1,9 KB per scansione, 440 MB per 237.000 scansioni sintetiche) e la RSS scende di 350 MB. Nei file LR piccoli l'invio costa il doppio (8,7 s contro 4,3: l'indice è costruito subito, prima si costruiva al primo disegno, 3,9 s → 1,4 s) e la riapertura scende da 11,4 s a 8,2 s, di cui ~4,5 s sono l'avvio del motore. Mappa di tutti i file: ×4 per HR (23 s → 5,6 s, `np.maximum.at` al posto dell'ordinamento per cella), invariata per LR. XIC su 20 file LR 69 → 45 ms (mediana), sotto i 100 ms; su 15 file HR resta 500 ms perché la risposta contiene 475.000 numeri da scrivere in JSON (`rounded()` ha tolto 130 ms): il lavoro sull'indice è ~1 ms. Spettro 2,5 ms (LR) e 7,5 ms (HR), cambio file 5,7 e 74 ms: sotto 16 ms e 200 ms.

## Costruttore del `.mzidx` (`python3 tools/bench.py --indici`, Chromium Linux 4 CPU, 2026-10-11)
«python» = costruttore Python in Pyodide (`?pool=0`, il riferimento), «pool» = worker Rust (predefinito). Tempo = dall'invio dei file a tutti nell'elenco (l'indice di ciascuno è costruito nel frattempo); memoria = somma dei processi Chromium (massimo campionato ogni 0,25 s); `digest` = hash di TIC e 5 XIC di ogni file; riapertura = nuova pagina nello stesso profilo (le differenze di riapertura tra le due righe sono rumore: l'indice è lo stesso). File: gli esempi anonimi del sito (LR: Full Scan; HR: il ritaglio Orbitrap ripetuto fino a 140 MB). Un solo giro per scenario.

| scenario | constructor | files | file_mb | build_total_s | rss_peak_build_mb | first_draw_s | reopen_first_draw_s | rust_passes_max | digest |
|---|---|---|---|---|---|---|---|---|---|
| 20 Full Scan LR | python | 20 | 107.3 | 15.39 | 747.9 | 0.26 | 5.96 | 0 | 35255d6425e8 |
| 20 Full Scan LR | pool | 20 | 107.3 | 9.51 | 749.5 | 0.34 | 7.91 | 1 | 35255d6425e8 |
| 15 HR DDA (140 MB each) | python | 15 | 2101.0 | 299.63 | 1010.2 | 1.85 | 12.19 | 0 | 77a5810c63d1 |
| 15 HR DDA (140 MB each) | pool | 15 | 2101.0 | 252.59 | 1321.9 | 1.34 | 11.61 | 3 | 77a5810c63d1 |

- 20 Full Scan LR: tempo totale −38 %, memoria uguale, array identici. 15 HR DDA: tempo −16 %, memoria massima +31 %, array identici. Somma dei due scenari: −17 %.
- Decisione: il pool resta il costruttore predefinito perché la correttezza è provata (array identici) e la velocità non blocca (regola di Federico); è più veloce in entrambi gli scenari. Il collo di bottiglia che resta è il lavoro in serie di Python prima della costruzione (intestazioni, SHA-256 dell'intero file, tabella delle scansioni), non l'ordinamento dei picchi: spostarlo in Rust è il passo successivo.
- SIMD (`+simd128`), 20 Full Scan LR, pool, tre giri: tempo totale 8,83 / 10,41 / 8,13 s con SIMD (media 9,12 s), 7,74 / 10,05 / 10,09 s senza (media 9,29 s); somma dei tempi nel worker (`buildMs`) 9,21 contro 9,02 s. Nessuna differenza misurabile; `.wasm` 773 KB contro 776 KB.
- Il costruttore nativo del desktop (memmap2 + rayon) non è misurato qui: va misurato sul Mac con `python3 tools/bench.py --indici` dentro l'app.

## Pulizia del codice (motore Python, 5 file Full Scan sintetici, 3 giri; Chromium cloud senza WASM Rust)
| misura | prima | dopo |
|---|---|---|
| apertura (open_s) | 3.32 / 3.49 / 3.42 | 3.08 / 3.33 / 3.07 |
| TIC + 5 XIC (tic_xic_s) | 0.61 / 0.33 / 0.31 | 0.34 / 0.34 / 0.34 |
| RSS del browser, MB | 806 / 810 / 790 | 766 / 810 / 725 |
| JS degli script di `mzlab/web/` (senza vendor e teoria), byte | 1 124 713 | 1 122 273 (−2 440) |

Tempi invariati entro il rumore; `workflow.js` e `perf.js` passano a moduli ES (partono dopo gli script classici).

## Schermata smartphone (iPhone 390×844, touch, server di sviluppo, mediana di 5)
| | prima | dopo |
|---|---|---|
| primo disegno (FCP), ms | 92 | 88 |
| byte scaricati fino a rete ferma | 1 034 427 | 330 178 (comprende ~85 KB di Teoria scaldata) |

Sul telefono non partono più gli script di Dati/Disegno né la shell; test `e2e_telefono`.

## Teoria da telefono (`tests_e2e/e2e_teoria_mobile.py`, `e2e_indirizzi.py`)
Chromium Linux, 390×844 con tocco, server di sviluppo, un giro, dati stampati come `MEASURE`: capitolo 4 ≈ 45 kB e LCP ≈ 0,18 s, capitolo 20 ≈ 140 kB (le due raster ridotte da 1400 px, 88 + 57 kB, caricate «lazy»; i file grandi da 384 + 182 kB solo all'apertura della figura), LCP ≈ 0,07 s. Apertura a freddo di `#/teoria` 11 richieste ≈ 160 kB, senza Pyodide, WASM, `api/` né script di Dati o Disegno (verificato). Rifare la misura: lanciare i due script (stampano le righe `MEASURE`).

## Mirino della mappa (`python3 tools/bench_mirino.py [lr|hr] [--site]`, Chromium Linux, 2026-10-11)
Mappa a schermo intero con il mirino acceso. LR: un Full Scan sintetico di 1500 scansioni × 500 centroidi (750 000 picchi); HR: le scansioni Full Scan di un Orbitrap sintetico (poche, ~600 scansioni). «site» = sito costruito (Pyodide in un worker; `python_ms` = tempo in Python dalla intestazione `X-Py-Ms`, il resto è andata e ritorno e copie JS↔worker); «dev» = server di sviluppo. Prima = `origin/main` con la sola strumentazione `?perf`; dopo = questo pacchetto. Un giro; tempi in ms, mediana / p95.

**Prima: dove va il tempo di un punto del mirino (sito, Pyodide)**
| fase | LR | HR |
|---|---|---|
| (a) XIC: totale / Python | 6.5 / 3.5 (p95 10.3 / 6.3) | 3.8 / 1.4 |
| (b) scansione più vicina (`nearest_scan`) + spettro (`spectrum`): totale | 2.4 + 49.5 (p95 6 + 75) | 1.8 + 13.2 |
| (b) di cui Python | 1.8 + 42.1 | 1.2 + 11.1 |
| (c) disegno dei due pannelli | 0.9 (p95 1.7) | 0.9 (p95 1.2) |
| (d) JS↔worker (totale − Python) | 0.6 / 7.4 / 3.0 (nearest / spectrum / XIC) | 0.6 / 2.1 / 2.4 |
| (e) richieste per movimento, spazzata lenta (60 eventi a 100 ms) | 2.27 (136) | 2.18 (131) |
| (e) trascinamento continuo (120 eventi ogni ~30 ms) | 4 richieste in tutto, 1 disegno | 3 richieste, 1 disegno |
| attesa dal punto al disegno esatto, spazzata (mediana / p95) | 114 / 140 | 77 / 84 |

Cause: (1) il debounce di 60 ms riparte a ogni evento, quindi **durante il movimento i due pannelli non si aggiornano mai** (1 disegno in 3,5 s di trascinamento; tenendo premuta una freccia, 2 richieste in 40 pressioni) e dopo l'arresto passano ≥ 60 ms più la risposta; (2) per ogni punto tre richieste in due tempi (`nearest_scan`, poi `spectrum`, in parallelo l'XIC), anche se la scansione non è cambiata (la chiave della cache era l'RT del pixel); (3) `Item._binned` scorre **tutta** la tabella dei picchi (ordinata per m/z) per estrarre una scansione: 42 ms di Python su 750 000 picchi, proporzionale alla dimensione del file; (4) la cache `CACHE` di `explore.js` non ha limite (uno spettro per ogni scansione visitata, per sempre).

**Dopo**
| misura | prima | dopo |
|---|---|---|
| spettro di un punto: Python, LR (mediana) | 42.1 | 3.2 (`/api/spectra`: legge la scansione dal file) |
| spettro di un punto: totale, LR / HR | 49.5 / 13.2 | 4.1 / 2.2 |
| richieste, spazzata lenta, LR / HR | 136 / 131 | 38 / 36 |
| attesa dal punto al disegno esatto, spazzata, LR / HR (mediana) | 114 / 77 | 24 / 23 |
| attesa p95, spazzata, LR / HR | 140 / 84 | 29 / 30 |
| aggiornamenti esatti durante il trascinamento, LR / HR (120 eventi) | 1 / 1 | 68 / 90 (più 132 / 106 anteprime) |
| 40 frecce premute di seguito, LR: richieste / aggiornamenti esatti / attesa | 2 / 1 / 108 | 33 / 40 / 21 |
| fotogramma durante il trascinamento (mediana / p95 / max) | 16.7 / 16.8 / 16.8 | 16.7 / 33.4 / 50.0 (LR), 16.7 / 16.8 / 16.8 (HR) |
| lavoro del mirino per fotogramma (mediana / p95 / max), dev | 0.4–1.3 (solo il disegno al movimento) | 0.8 / 1.4 / 4.2 |
| attività lunghe (> 50 ms) durante il trascinamento | 0 | 0 |

Le richieste per movimento salgono nel trascinamento continuo (prima il mirino non chiedeva nulla finché si muoveva, quindi non mostrava nulla) e scendono per ogni aggiornamento mostrato: 3 richieste → ≤ 1 (spettro e XIC solo se cambiano), 0 sulle scansioni già visitate (cache), mai più di due insieme. Il risultato a mirino fermo è identico (`tests/test_scan_axis.py`: stessa scansione, RT, m/z, intensità e `mode` di `nearest_scan` + `spectrum`, LR e HR a due livelli; `e2e_mappa_mirino`: spettro e XIC finali uguali a `/api/nearest_scan` + `/api/spectrum` + `/api/xic`). Il massimo di 50 ms del fotogramma LR non viene dal codice del mirino (4.2 ms al massimo) ma dal compositore di Chromium senza GPU con la mappa a schermo intero.

**Cosa risolverà RUST_PRIMO e cosa no.** Già coperti dal motore Rust: l'XIC (`/api/xic`) e lo spettro di una scansione LR a centroidi senza fusione (`/api/spectra`, `rust-bridge.js`), cioè il costo (a) e (b) di Python, oggi 3-4 ms ma proporzionale al file nel vecchio percorso; il nuovo percorso del mirino li usa già con entrambi i motori. Restano: `/api/scan_axis` (una volta per file e livello, risposta Python; lista di RT in JSON), le copie JSON di array di numeri (7 ms su 500 picchi per lo spettro LR: il passaggio come `Float32Array` trasferibile è lavoro del motore, non della pagina), la cache e la coalescenza (solo JS), il disegno. Provato e scartato: una richiesta unica «XIC + spettro» (due andata-e-ritorno da 3-4 ms in parallelo non dominano più, e un endpoint unico escluderebbe l'XIC dal motore Rust); riduzione dell'XIC a min/max per colonna di pixel (il disegno costa 0.9 ms); letture di sole colonne dal `.mzidx` (lo spettro ora legge la scansione dal file, l'XIC usa già la ricerca per m/z).
