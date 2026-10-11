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
