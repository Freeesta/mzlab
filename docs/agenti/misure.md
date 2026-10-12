# Misure dei motori (`python3 tools/bench.py --memoria --scrivi`)
Chromium Linux, sito costruito con il motore Rust, 2026-10-10, commit eb6c6f1, stessa macchina, un giro. «open» = caricamento + primo TIC disegnato; «TIC+XIC» = TIC e 5 XIC di ogni file. Con `?motore=rust` il motore Python resta caricato e legge comunque i file (Rust risponde solo a TIC, XIC e scansioni): la memoria è la somma dei due.
Memoria dopo l'apertura dei file e le richieste di TIC e XIC: heap WASM = somma delle `WebAssembly.Memory` del worker; IndexedDB = byte dei file salvati; heap JS della pagina da `performance.memory` (nei worker non c'è); RSS = somma dei processi Chromium. Rifare la misura: `python3 tools/build_site.py`, costruire il Rust come in `pages.yml`, poi `python3 tools/bench.py --memoria --scrivi` (file LR: `MZLAB_MZML`; Chromium: `MZLAB_CHROMIUM`).

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

## Librerie spettrali (`python3 tools/bench.py --librerie --gb --scrivi`, Chromium Linux con profilo persistente, 2026-10-11, commit 5d49ecf)
«js» = percorso JavaScript di prima (`?libjs=1`, solo fino a 10^5 spettri: oltre non è pratico), «wasm» = motore nuovo. Tempi misurati dalla pagina (comprendono il passaggio dei messaggi al worker); identità e analoghi = mediana di 30 e 10 richieste a caldo, «cold» = la prima; «identify_all» = 5000 MS2 (400 per MassBank, che ne ha meno). Memoria = picco RSS di tutto il browser meno la base prima dell'importazione. Spettri sintetici (8-40 picchi uniformi: comprimono poco, il peso su disco è quindi un caso sfavorevole).
Bersagli: identità < 5 ms a caldo, analoghi < 100 ms su 10^6, MGF da 1 GB < 90 s con ≤ 200 MB. Non raggiunti nel browser: identità 8-12 ms (la maggior parte è il giro dei messaggi e del JSON; il calcolo nativo è sotto il millisecondo), analoghi 136 ms su 10^6, MGF da 1 GB 130 s e 331 MB; il costruttore nativo (stesso codice) importa lo stesso MGF in 74 s con 110 MB. Il peso su disco della MassBank è 8,0 MB contro 15,7 MB di MSP (i due indici Flash non sono compressi).

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
«Prima» = il sito costruito dal commit precedente al punto 2 (tabella dei picchi in memoria, file in IndexedDB/WORKERFS), «dopo» = sito con `.mzidx` in OPFS; stessi file, stessa macchina, un giro. File: sintetici (nessun dato del laboratorio nel contenitore): 20 Full Scan LR di 2,4 MB; 15 file HR DDA di 78 MB ottenuti ripetendo 20 volte il file Orbitrap sintetico (`inflate`: ~15.800 scansioni e 2 M di picchi in MS1 ciascuno, più scansioni e meno picchi per MB di un file vero).
Apertura = invio dei file, lettura, scrittura in OPFS e costruzione dell'indice di ognuno; primo disegno = clic su «Apri» fino al TIC; riapertura = nuova pagina nello stesso profilo, fino al primo disegno. Tempi di risposta su 3 × N spettri (`api/scanbin`), 5 m/z × 2 giri di XIC su tutti i file insieme, 2N richieste di TIC (cambio file). La mappa è `api/map` di ogni file sulla griglia di sessione.

**20 Full Scan LR (sintetici, 2,4 MB l'uno)**

| misura | dopo |
|---|---|
| apertura (invio + lettura + indice), s | 8.65 |
| primo disegno, s | 1.38 |
| TIC + 5 XIC di ogni file, s | 1.47 |
| mappa di tutti i file, s | 1.1 |
| riapertura fino al primo disegno, s | 8.16 |
| mappa di tutti i file dopo la riapertura, s | 1.19 |
| spettro (mediana / max), ms | 2.5 / 9.6 |
| XIC su tutti i file (mediana / max), ms | 45 / 66.1 |
| cambio file, TIC (mediana / max), ms | 5.7 / 14.1 |
| heap WASM dopo il lavoro, MB | 155.5 |
| heap WASM dopo la riapertura, MB | 129.6 |
| RSS massima del browser, MB | 831.3 |

**15 HR DDA da 78 MB (sintetici)**

| misura | dopo |
|---|---|
| apertura (invio + lettura + indice), s | 166.62 |
| primo disegno, s | 12.98 |
| TIC + 5 XIC di ogni file, s | 6.04 |
| mappa di tutti i file, s | 5.64 |
| riapertura fino al primo disegno, s | 24.27 |
| mappa di tutti i file dopo la riapertura, s | 7.49 |
| spettro (mediana / max), ms | 7.5 / 21.8 |
| XIC su tutti i file (mediana / max), ms | 500.3 / 551.7 |
| cambio file, TIC (mediana / max), ms | 73.9 / 109.7 |
| heap WASM dopo il lavoro, MB | 570.9 |
| heap WASM dopo la riapertura, MB | 570.9 |
| RSS massima del browser, MB | 1355.3 |

## Costruttore del `.mzidx` (`python3 tools/bench.py --indici`, Chromium Linux 4 CPU, 2026-10-11)
«python» = costruttore Python in Pyodide (`?pool=0`, il riferimento), «pool» = worker Rust (predefinito). Tempo = dall'invio dei file a tutti nell'elenco (l'indice di ciascuno è costruito nel frattempo); memoria = somma dei processi Chromium (massimo campionato ogni 0,25 s); `digest` = hash di TIC e 5 XIC di ogni file; riapertura = nuova pagina nello stesso profilo (le differenze di riapertura tra le due righe sono rumore: l'indice è lo stesso). File: gli esempi anonimi del sito (LR: Full Scan; HR: il ritaglio Orbitrap ripetuto fino a 140 MB). Un solo giro per scenario.

| scenario | constructor | files | file_mb | build_total_s | rss_peak_build_mb | first_draw_s | reopen_first_draw_s | rust_passes_max | digest |
|---|---|---|---|---|---|---|---|---|---|
| 20 Full Scan LR | python | 20 | 107.3 | 15.39 | 747.9 | 0.26 | 5.96 | 0 | 35255d6425e8 |
| 20 Full Scan LR | pool | 20 | 107.3 | 9.51 | 749.5 | 0.34 | 7.91 | 1 | 35255d6425e8 |
| 15 HR DDA (140 MB each) | python | 15 | 2101.0 | 299.63 | 1010.2 | 1.85 | 12.19 | 0 | 77a5810c63d1 |
| 15 HR DDA (140 MB each) | pool | 15 | 2101.0 | 252.59 | 1321.9 | 1.34 | 11.61 | 3 | 77a5810c63d1 |

## Pulizia del codice (motore Python, 5 file Full Scan sintetici, 3 giri; Chromium cloud senza WASM Rust)
| misura | dopo |
|---|---|
| apertura (open_s) | 3.08 / 3.33 / 3.07 |
| TIC + 5 XIC (tic_xic_s) | 0.34 / 0.34 / 0.34 |
| RSS del browser, MB | 766 / 810 / 725 |
| JS degli script di `mzlab/web/` (senza vendor e teoria), byte | 1 122 273 (−2 440) |

## Schermata smartphone (iPhone 390×844, touch, server di sviluppo, mediana di 5)
| | dopo |
|---|---|
| primo disegno (FCP), ms | 88 |
| byte scaricati fino a rete ferma | 330 178 (comprende ~85 KB di Teoria scaldata) |

## Rust legge l'indice (`python3 tools/bench.py --files 6 --json`, Chromium cloud con WASM, 6 Full Scan sintetici, un giro, commit di partenza e commit del lavoro)
Prima = `?motore=rust` con il motore che apre l'mzML per TIC e XIC; dopo = lo stesso motore che risponde dall'indice `.mzidx` (TIC, XIC, filtri, elenco scansioni, scansione più vicina) e apre l'mzML solo per le scansioni singole.

| misura | dopo (Rust) | Python (stesso giro, dopo) |
|---|---|---|
| avvio (pagina + motore), s | 5.38 | 4.37 |
| apertura dei file fino al primo TIC, s | 2.27 | 2.24 |
| TIC + 5 XIC di ogni file, s | 0.11 | 0.35 |
| RSS del browser, MB | 1033 | 1073 |

## Teoria da telefono (`tests_e2e/e2e_teoria_mobile.py`, `e2e_indirizzi.py`)
Chromium Linux, 390×844 con tocco, server di sviluppo, un giro, dati stampati come `MEASURE`: capitolo 4 ≈ 45 kB e LCP ≈ 0,18 s, capitolo 20 ≈ 140 kB (le due raster ridotte da 1400 px, 88 + 57 kB, caricate «lazy»; i file grandi da 384 + 182 kB solo all'apertura della figura), LCP ≈ 0,07 s. Apertura a freddo di `#/teoria` 11 richieste ≈ 160 kB, senza Pyodide, WASM, `api/` né script di Dati o Disegno (verificato). Rifare la misura: lanciare i due script (stampano le righe `MEASURE`).

## Mirino della mappa (`python3 tools/bench_mirino.py [lr|hr] [--site]`, Chromium Linux, 2026-10-11)
Mappa a schermo intero con il mirino acceso. LR: un Full Scan sintetico di 1500 scansioni × 500 centroidi (750 000 picchi); HR: le scansioni Full Scan di un Orbitrap sintetico (poche, ~600 scansioni). «site» = sito costruito (Pyodide in un worker; `python_ms` = tempo in Python dalla intestazione `X-Py-Ms`, il resto è andata e ritorno e copie JS↔worker); «dev» = server di sviluppo. Prima = `origin/main` con la sola strumentazione `?perf`; dopo = questo pacchetto. Un giro; tempi in ms, mediana / p95.

| misura | dopo |
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

## Pulizia finale, fase A (commit di partenza 2d50415; Chromium cloud, dati sintetici, senza WASM Rust)
| Misura | Dopo |
|---|---|
| `golden.py --controlla` | uguale |
| `verifica.py` completa | 137 OK (6 controlli in più: mappa, soglie, nuovi test di main), e2e16 e e2e25 NOTO |
| `draw()` per 30 eventi resize (`tools/misura_redraw.py`, 2 pannelli) | 2 |
| `prototipi/` | cancellata |
| `bench.py --velocita` / `--mzidx` | non eseguiti |

| Area | Righe dopo | KB dopo |
|---|---|---|
| web (pagina) | 25707 | 1946 |
| web teoria | 3858 | 334 |
| python mzlab | 8272 | 424 |
| rust crates | 22765 | 744 |
| tools | 6256 | 358 |
| tests | 22863 | 1524 |
| TP_Mine | 12885 | 770 |

## Flusso Rust senza Pyodide (`tests_e2e/e2e_lr.py`, riga «MISURA»)
Container cloud (Chromium 1194; su un runner GitHub Actions prima della correzione: Rust 0.46 s / 6.0 s, classico 2.47 s / 0.67 s), 2 file Full Scan sintetici (`tools/dati_sintetici.py`), un giro. «avvio→pagina pronta» = dall'apertura dell'indirizzo alla schermata di partenza; «apertura» = dal caricamento dei due file al primo TIC disegnato.

| flusso | avvio→pagina pronta (s) | apertura (s) |
|---|---|---|
| Rust (predefinito) | 0.46 | 1.3 |
| classico (`?motore=python`) | 2.47 | 1.2 |

Stesso container prima della correzione: Rust 6.5 s di apertura, classico 1.2 s. L'apertura costava la tabella delle scansioni (`prepare_upload`, 4.5 s a file).
Senza `indexList` (i file sintetici) lo scorrimento rileggeva 1 MB per ogni spettro e ogni lettura sincrona di `Blob` in OPFS costa circa 1 ms; ora `idxread::Windowed` serve le letture piccole da una finestra di 1 MB e lo scorrimento tiene il suo buffer (688 letture da 1 MB → 5). 
Resta ~0.7 s per la costruzione dell'indice (avvio del worker, 0.2 s di calcolo a file, in parallelo). Con file veri (centinaia di MB, di norma con `indexList`) la misura non è stata fatta.

## Famiglie di ioni per file (`cargo run --release -p mzlab-feat --example famiglie -- FILE.mzidx [--truth verita.json]`, 4 CPU, solo `analyse_families`)
Serie sintetica d'identità (9672 feature in `t05min`): 0,03 s, famiglie giuste 98,9 % (basi con la famiglia giusta 181/183), ISF 12/13, addotti 31/31, falsi ISF dichiarati ISF 0/13, NH4 lasciati aperti 8/8. Tre file Orbitrap da circa 80 MB (indice 54-56 MB): 141 208 feature → 119 317 famiglie in 0,76 s (il file con più rumore), 77 197 → 65 645 in 0,32 s, 79 560 → 67 594 in 0,32 s. Il tempo è per due terzi nella lettura delle tracce (`traces::gather` scorre quasi tutta la tabella dei picchi) e nelle finestre di m/z dei candidati.


## «Da dove viene questo ione?» (`api/origin`, `cargo test --release -p mzlab-core --test origin_parity` e `tools/origin_riferimento.py`, nativo, dati LR veri e sintetici, un report con 200 bootstrap, 400 spostamenti, 25 candidati)
| Motore | Tempo per rapporto (ms) |
|---|---|
| Python (numpy, `ionfamily.origin_report`) | 320-365 |
| Rust (`origin::origin_report`) | 80-90 |

Il tempo comprende ROI, tracce, somiglianza di profilo con p-value di permutazione, rapporto con bootstrap, cinetica, ruoli e candidati. Nel browser (WebAssembly) non misurato qui. Parità: stessi indici e conteggi, numeri uguali entro 1e-9, p-value Monte Carlo identici (stesso `default_rng`).

## Mappa RT×m/z e punti segnati (`api/map`, `mapsnap`, `mappunti`, `mappunti3d`; `cargo test --release -p mzlab-core --test map_parity -- --nocapture` e `tools/mappa_riferimento.py`, nativo, tre file Full Scan veri)
| Richiesta | Python (ms) | Rust (ms) |
|---|---|---|
| mappa intera (prima richiesta, a freddo) | 216 | 12 |
| mappa intera con sfondo (mediana lungo la corsa) | 20 | 22 |
| differenza con `ref` e `norm=max` | 6 (mappe già in memoria) | 12 |
| regione con zoom 120 × 100 | 3,6 | 2,7 |
| aggancio, misure dei punti, punti del 3D (300 punti) | 0,3-5 | 0,1-6 |

Nel browser (sintetici, due file di 2,4 MB, prima richiesta di ogni rotta, `e2e_lr`): mappa intera 54 ms in Rust contro 46 in Pyodide già avviato, con sfondo 102 contro 73, regione 36 contro 24, aggancio 7 contro 2, punti 26 contro 3, punti 3D 22 contro 37. Il guadagno non è la velocità della singola richiesta (Pyodide ha già la tabella in memoria) ma che Pyodide non parte: avvio della pagina 1,3 s contro 5,0 s. Parità: celle e punti uguali bit a bit; dei punti 3D tagliati a `max` si confrontano gli insiemi per intensità (numpy sceglie a caso fra intensità uguali).

## Formule compatibili (`api/composition`, `cargo test --release -p mzlab-core --test composition_route_parity -- --nocapture` e `tools/composizione_rotta_riferimento.py`, nativo) e XIC da formula (`api/subxic`, `--test subxic_parity` e `tools/subxic_riferimento.py`, due file Orbitrap veri di 80 MB)
| Richiesta | Python (ms) | Rust (ms) |
|---|---|---|
| composizione con intervalli di elementi (C 0-20, H 0-40, N 0-5, O 0-8, S 0-2) | 1-7 | 0,1-2,5 |
| composizione con gli intervalli di base | 20-44 | 5-7 |
| composizione con picchi isotopici e frammenti | 1-5 | 0,5-2,4 |
| XIC da formula, 24 000 sottoformule a ±5 ppm | 540-660 | 100-220 |
| XIC da formula, 26 000 sottoformule a ±10 ppm | 700-1240 | 140-280 |
| XIC da formula, 480 sottoformule | 9-21 | 9-45 (il tempo è quello di caricare la tabella dei picchi) |

Parità: stessi candidati nello stesso ordine e numeri entro 1e-9 per `composition`; righe, ordine e numeri identici per `subxic` (anche sui file veri). Nel browser non misurato: la pagina HR passa a Python alla prima richiesta di spettro, quindi `subxic` ci arriva solo da un motore già avviato; `composition` non ha bisogno di file.

## Spettri in profilo (`api/spectrum` e `api/spectra` di un file LR in profilo; `cargo test --release -p mzlab-core --test profile_parity -- --nocapture` e `tools/profilo_riferimento.py`, nativo; `e2e_lr` nel browser, file sintetico di 400 scansioni)
| Richiesta | Python (ms) | Rust (ms) | Browser: Pyodide già avviato / Rust (ms) |
|---|---|---|---|
| spettro di una finestra di 14 scansioni | 7-11 | 4-9 | 84 / 38 |
| spettro medio di tutta la corsa | 15-20 | 4-5 | 58 / 27 |
| 11 scansioni singole | 14-20 | 8-9 | 38 / 31 |
| 60 scansioni singole | 92 | 29-30 | 220 / 85 |
| una scansione singola | 1,6-2,7 | 4-5 (una lettura dell'indice intero) | – |

Parità: stessi picchi, stessa linea con gli zeri rimessi, numeri entro 1e-9; i punti sono sommati sulla griglia `rint(m/z / passo)` con il passo letto da venti scansioni a metà corsa, come `Item._step`.

## Rilevatori di picchi e misure di qualità (`peakq.rs` contro `peaks::detect_peaks`, nativo, release, media di 20 giri, traccia sintetica di 15 min con tre picchi gaussiani e rumore)
| Punti della traccia | CWT (ms) | prominenza + minimo locale (ms) | misure di qualità (µs per picco) |
|---|---|---|---|
| 2000 | 5,9 | 0,20 | 2,3 |
| 20 000 | 54 | 9,3 | 20 |

La CWT resta il metodo predefinito e non cambia (parità di bordi e aree con Python in `golden.rs`); la modalità nuova costa un decimo e separa i picchi doppi dove la CWT li unisce (prove sintetiche in `peakq.rs`: doppio parzialmente risolto, spalla, coda, solo rumore).

## Spettro HR e tabella DDA (`cargo test -p mzlab-core --test hr_parity --release -- --nocapture`, 2026-10-12, stessa macchina, un giro)
Due file Orbitrap DDA sintetici (`tools/hr_riferimento.py`, ~15.000 picchi MS1 per scansione); «rust» = `Routes::handle`/`spectrum_exact` nativo dall'indice, «python» = `api.dispatch` nativo (senza il costo di Pyodide). Parità ≤1e-9 relativa su tutti i valori. Tempi in ms per richiesta:

| richiesta | rust | python |
|---|---|---|
| `api/dda` | 0,7 | 1,4 |
| `api/spectrum` MS1, finestra di 12 s (circa 20 scansioni) | 2,0 | 2,9 |
| `api/spectrum` MS1, tutto il file | 59 | 99 |
| `api/spectrum` MS2, tutto il file | 0,9 | 1,7 |
| `api/spectrum` MS2 con precursore | 0,2 | 0,5 |
| `api/spectrum` MS1, una sola scansione (legge l'mzML) | 1,5 | 1,7 |

## Mappe HR (`cargo test -p mzlab-core --test map_parity --release -- --nocapture`, 2026-10-12, stessa macchina, un giro, primo uso di ogni file)
Tre file Orbitrap DDA reali (~3900 scansioni MS1, ~290 000 picchi in 100–130 m/z), `tools/mappa_riferimento.py`; ms nativi dall'indice contro `dispatch` di Python con le tabelle già in memoria. Mappe identiche bit a bit; Rust paga la lettura delle colonne dall'indice alla prima richiesta.

| richiesta | rust | python |
|---|---|---|
| `api/map` intera, file 1 | 126 | 126 |
| `api/map` intera con `bg=1`, file 3 | 286 | 683 |
| `api/map` regione 250–360 m/z, 120 × 1000 | 16 | 12 |
| `api/map` regione, `mzscale=log` con `ref` | 19 | 12 |
| `api/mappunti3d` regione, 300 punti | 21 | 8 |
