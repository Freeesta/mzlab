# Misure su computer lenti (PERF-MACCHINE, passo 1)

Solo rapporto: nessuna soglia che fallisce. Serve a decidere dopo (soglie, profili leggero/normale/potente, lavoro a pezzi, tetto di memoria) dove la pagina si rompe per prima quando il computer è 4-6 volte più lento di un MacBook M1 Pro.

## Come si lancia
- **Comando**: `python3 tools/prepara_sito.py` (una volta: `site/` con il motore .wasm) e poi `python3 tests_e2e/misura_macchine.py --md OUT.md [--json OUT.json] [--reps 12] [--rates 1,4,6] [--cores 2,4]`. Non fa parte dei test della CI (il nome non inizia con `e2e_`), non fallisce mai su un numero.
- **Durata**: 4,6 minuti per le 6 configurazioni (1×, 4×, 6× × 2 e 4 core) con `--reps 12` su un container a 4 CPU (22 s a 1×, 46 s a 4×, 68 s a 6× per configurazione); la costruzione del sito la prima volta richiede alcuni minuti in più.
- **Come misura**: ogni configurazione è un browser nuovo (cache e archivio vuoti); `Emulation.setCPUThrottlingRate` come `e2e_cls.py`, `navigator.hardwareConcurrency` ridefinito da uno script di avvio. Il tempo parte prima dell'azione e si ferma dopo il disegno successivo al risultato, dentro la pagina (senza il ritardo di Playwright). Le fasi vengono da ganci che avvolgono `fetch`, `Response.json`, `draw()` e `drawMap()` senza toccare il prodotto: richiesta = unione delle chiamate `api/`, motore = somma di `X-Py-Ms`, JSON, disegno = somma delle durate di `draw()`. Le fasi si sovrappongono.
- **Dati**: solo i cinque file di esempio Full Scan (`mzlab/web/esempi/`, scaricati dal sito con «Prova con i file di esempio»), mai dati del laboratorio.

## Risultati (2026-10-25, main b57b9867755700d61b406b3888c25deba821d012)
Macchina: container cloud, 4 CPU, Chromium headless; `--reps 12` (mappe 2D e 3D: 4 ripetizioni; apertura: una sola). Celle `p50 / p95 ms`; per il trascinamento, intervallo tra due fotogrammi e fotogrammi persi (gli intervalli oltre 25 ms, conteggiati come 16,7 ms ciascuno) sui due passaggi andata e ritorno della linea. Ultima riga: picco dello heap JS / memoria del motore in MB.

| azione | 1× / 2 core | 1× / 4 core | 4× / 2 core | 4× / 4 core | 6× / 2 core | 6× / 4 core |
|---|---|---|---|---|---|---|
| apertura demo Full Scan: primo grafico | 429 | 345 | 878 | 823 | 1384 | 1420 |
| apertura demo Full Scan: pronto | 2535 | 2254 | 3082 | 3105 | 4609 | 5124 |
| trascina la linea: intervallo tra fotogrammi | 16,7 / 16,8 (0 persi) | 16,7 / 16,7 (0 persi) | 16,7 / 83,3 (314 persi) | 16,7 / 66,7 (265 persi) | 16,7 / 116,7 (604 persi) | 16,7 / 133,3 (675 persi) |
| trascina la linea: un `draw()` (cromatogramma o spettro) | 4,1 / 52,1 | 3,8 / 46,5 | 26,5 / 139,5 | 23,1 / 137,9 | 37,7 / 174,9 | 43,9 / 206,4 |
| nuovo pannello (cromatogramma) | 44,8 / 45,4 | 44,2 / 45,5 | 110,8 / 124,5 | 99,1 / 145,9 | 183,1 / 320,5 | 204 / 254,8 |
| cambio di file | 30,5 / 280 | 30 / 313,3 | 136,6 / 427,3 | 132,2 / 430,8 | 191,4 / 493,3 | 202,3 / 493,8 |
| mappa 2D | 62,3 / 199,9 | 56,3 / 186,6 | 208,9 / 647,1 | 202,6 / 546,7 | 392,3 / 1096 | 415,9 / 916 |
| mappa 3D | 1323 / 1390 | 1380 / 1503 | 2533 / 2573 | 2448 / 2709 | 3325 / 3638 | 3221 / 3267 |
| XIC da un clic | 61,7 / 124,5 | 61,1 / 119,5 | 245 / 401,4 | 244,1 / 289,7 | 457,3 / 555,7 | 381,8 / 467,8 |
| heap JS / motore (MB) | 61,8 / 32 | 59,1 / 32 | 68 / 32 | 66,4 / 32 | 67,8 / 32 | 65 / 32 |

## Le 5 azioni peggiori al 6× (4 core, p50) e dove va il tempo
Fasi mediane in ms (si sovrappongono): richiesta / motore / JSON / disegno; tra parentesi lo stesso a 1×.
1. **Apertura, pronto**: 5124 ms (2254 a 1×); il primo grafico è a 1420 ms. Non scomposta dai ganci (include la lettura dei 5 file e il disegno dei pannelli).
2. **Mappa 3D**: 3221 ms (1380); richiesta 271 (96), JSON 70 (6), il resto è il disegno WebGL. Qui il WebGL è software (container senza GPU): il numero è pessimista e va riconfermato su un portatile vero.
3. **Mappa 2D**: 416 ms con p95 916 (56 / 187); disegno 559 (57), nessuna richiesta nel conteggio: il tempo è tutto nel disegno dell'immagine.
4. **XIC da un clic**: 382 ms (61); richiesta 139 (29), motore 322 (69), disegno 267 (40). Il motore costa 4,6 volte di più a 6×: il rallentamento CPU di Chromium rallenta anche il worker del motore.
5. **Nuovo pannello 204 ms e cambio di file 202 ms** (44 e 30): disegno 69 e 129 (8 e 12); nessuna richiesta.
Trascinare la linea è l'azione che si vede di più: in circa 11 s di trascinamento si perdono 265-314 fotogrammi a 4× e 604-675 a 6× (0 a 1×); a 6× un `draw()` costa 38-44 ms in mediana e 175-206 al p95 (4 a 1×), con 118 long task su 18 s a 6×.

## Cosa dicono i numeri
- Con il rallentamento le azioni costano da 2,3 a 7 volte di più (6×: XIC 6,2, cambio di file 6,7, mappa 2D 7,4, nuovo pannello 4,6, mappa 3D 2,3, `draw()` del trascinamento 9-12). Tutte superano i 16 ms già a 1× tranne il `draw()` del trascinamento; mappa 2D, XIC e cambio di file sono sopra i 130 ms già a 4×.
- Il numero di core dichiarato (2 contro 4) non cambia nulla qui: i valori differiscono dentro il rumore. La macchina di misura ha 4 CPU e non limita i thread veri; la differenza reale di core va misurata su un computer vero (o limitando le CPU del processo con `taskset`).
- La memoria non è il problema in questo scenario (heap JS ~65 MB, motore 32 MB, a prescindere dal rallentamento). `measureUserAgentSpecificMemory` non è disponibile senza isolamento cross-origin: la colonna del sito resta `performance.memory`.
- Il p95 del cambio di file a 1× (280-313 ms contro un p50 di 30) è il primo cambio, a freddo: con 3 ripetizioni sole diventa il p50.
- Limiti: un solo giro, una sola macchina, `n` piccolo per le mappe e per l'apertura; i p95 sono di orientamento. Il confronto vale solo fra celle dello stesso giro.

## Apertura «pronto» a 6×: causa misurata (PERF-MACCHINE 2a)
- **Banco**: `--open-reps N` ripete solo l'apertura in browser nuovi (una sola è rumorosa); `--patch FILE.js` aggiunge uno script di avvio per provare un'idea (A/B) senza toccare il prodotto.
- **Dove va il tempo** (profilo CPU di Chromium, 6×, 4 core): il primo grafico è a 1,4 s; poi la pagina rifà un giro per ogni file aggiunto (`api/session`, `explore`, TIC, spettro, `metodo`, `method_warnings`: 5 giri, ciascuno 0,4-0,8 s a 6×) fino a 4,5-5 s. Il primo giro dura 3,4 s: il TIC del file 0 costa 1031 ms di motore contro 93 a 1× (11×, più dei 6× del rallentamento: avvio a freddo del motore) e `metodo` 1,1-1,2 s. Il thread principale ha 21 long task nel periodo; nativo (`(program)`) 3,1 s, GC 0,56 s, JS circa 2 s.
- **Strade provate, nessun guadagno oltre il rumore (±5 %)**: memoria di `namesWidth`, cache del TIC e degli spettri dei file già aperti, un solo disegno per fotogramma della lista file; numeri in `ottimizzazioni-tentate.md`. Il percorso critico è il motore (un giro per file in serie), non il thread principale.
- **Aprire con meno giri** (ridisegno di `openRest` ogni 3 volte la durata dell'ultimo): nessun guadagno (4881 contro 4884 ms), vedi `ottimizzazioni-tentate.md`. Anche l'avvio del motore prima del clic è già così (parte al caricamento della pagina) e 10 s di quiete prima del clic non cambiano il pronto (4529 contro 4458 ms): il TIC del file 0 a freddo (1031 ms contro 93 a 1×) non è l'avvio ma il primo lavoro del motore; vedi `ottimizzazioni-tentate.md`.

