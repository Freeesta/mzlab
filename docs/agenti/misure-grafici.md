# Misure dei pannelli dei grafici (vista Dati)

Costo del disegno dei cromatogrammi su tracce sintetiche dense: `python3 tools/bench_grafici.py [--rate 1|4] [--only big,many]` (sito costruito con il motore, `tools/prepara_sito.py`; Chromium headless senza GPU, cioè tela 2D software, 1500×1000, 4 CPU; file in `/tmp/mzlab_bench_grafici`). Una misura, non un test.

- **Insiemi**: `big` = un file SRM con 4 transizioni da 10^6 punti (20 min), una mostrata; `many` = 20 file SRM da 10^5 punti, tutti spuntati e sovrapposti (riempimento «Area» acceso, il predefinito dei pannelli MRM). Rumore sulla linea di base, un picco gaussiano per transizione e 6 punte di un solo punto.
- **Colonne**: «draw» = tempo di `draw()` (mediana di 8, ms) sulla vista intera e su uno zoom della sola linea di base (9,45-9,62 min); gesti = chiamate a `draw()`, intervalli tra i fotogrammi (mediana / p95 / massimo, ms) e attività lunghe (> 50 ms: numero / la più lunga).
- **Dove va il tempo**: con la tela software il tratto più largo di 1 px di una linea fitta e frastagliata (rumore) costa ~70 volte una linea di 1 px (prova a parte: 20 linee da 825 vertici, 694 ms contro 10 ms); il riempimento trasparente sotto 20 tracce ~45 ms. La lettura della tela per la legenda (`legende.js`) porta quel lavoro sul thread principale; una lettura costa come quattro.

## 20 file da 10^5 punti, CPU 1x (`main` `0be060b32ecb1ba7c12be9a593b9d741df1af6f0` → linea di 1 px oltre mezzo punto per pixel, `decima.js` `hairline`)
| azione | prima | dopo |
|---|---|---|
| draw, vista intera, ms | 39.7 | 43.4 |
| draw, zoom sulla linea di base, ms | 230.0 | 56.8 |
| Ctrl+rotella ×30: fotogrammi p95 / max, ms; attività lunghe | 50.0 / 300.0; 4 (300) | 50.0 / 50.0; 4 (64) |
| Maiusc+trascina ×60 (con zoom): fotogrammi p95 / max, ms; attività lunghe | 233.3 / 250.0; 60 (253) | 66.7 / 83.3; 60 (88) |
| ridimensiona la finestra ×10: chiamate a `draw()`; fotogrammi mediana / max, ms | 15; 116.7 / 183.3 | 15; 125.0 / 183.3 |

## Vista a cascata, 20 file da 10^5 punti, CPU 1x (`main` `622c60ccc384e94cb663950740383717ee8af69e` → `grafici.js` disegna ogni traccia della cascata con `DECIMA.rawLine`; prova `e2e_cascata_densa`)
| azione | prima | dopo |
|---|---|---|
| draw, vista a cascata, ms | 2903.3 | 88.4 |
| vertici per traccia (tela di 1184 px, 60 000 punti) | 60 000 (riempimento e linea) | 1057 |

Prima ogni punto entrava due volte nel percorso (riempimento sotto la traccia e linea da 1,25 px); ora una colonna di pixel ha un vertice (il massimo della colonna, come nella vista sovrapposta) e la linea fitta è di 1 px (`hairline`). Il picco più alto di ogni traccia resta la cima della sua linea.

## Zoom verticale sulla linea di base, CPU 1x (`main` `2c4704f975e51dadaae010b537d45b509be23feb` → `grafici.js` passa ogni linea per `DECIMA.clipY`; prova `e2e_zoom_y_denso`)
| azione | prima | dopo |
|---|---|---|
| draw, zoom y 0-300, 20 file da 10^5 punti, ms | 326.5 | 148.6 |
| draw, zoom y 0-300, un file da 10^6 punti, ms | 28.0 | 28.8 |
| cima delle linee sopra la tela (4 file da 60 000 punti, zoom y 0-300), px | ~110 000 | ≤ 9 |

Con lo zoom verticale i picchi (10^5) finivano ~10^5 px sopra la tela e la tela software pagava tutta la lunghezza delle linee (profilo: `drawImage` della legenda, che svuota il disegno in sospeso, 300 ms). Ora il tratto fuori dal grafico corre su un bordo 9 px oltre il ritaglio, tagliato esattamente dove lo attraversa: la parte visibile è la stessa (differenze solo di antialiasing). Il resto del tempo è la banda di rumore ingrandita a tutta altezza.

Un file da 10^6 punti (stessa macchina): draw della vista intera 11 ms, dello zoom 0,9 ms; rotella, trascinamento e passaggio del mouse a 16,7 ms per fotogramma senza attività lunghe; ridimensionare ×10 = 15 chiamate a `draw()` (fotogrammi fino a 50 ms).

## HR, 120 tracce da 50 000 scansioni, CPU 1x (`tools/bench_grafici.py --only hr`; `main` `28c73e8b1aa0932cd786d7b44eaf71d4a8518269` → massimi e minimi a blocchi in `decima.js`, Ctrl+rotella con `drawSoon`)
Un file Orbitrap sintetico di 50 000 scansioni aperto 6 volte, tutti spuntati; un pannello XIC di 20 ioni = 120 tracce.
| azione | prima | dopo |
|---|---|---|
| draw, vista intera, ms | 105.5 | 58.4 |
| draw, zoom y 0-300, ms | 244.9 | 101.3 |
| Ctrl+rotella ×30: fotogrammi p95 / max, ms; attività lunghe | 116.7 / 183.3; 15 (192) | 50.0 / 66.7; 14 (77) |
| ridimensiona ×10: fotogrammi mediana / max, ms; attività lunghe | 166.7 / 200.0; 6 (190) | 83.3 / 100.0; 6 (94) |

Prima la scala y di ogni traccia scorreva due volte tutti i 50 000 punti a ogni disegno (`amax`/`amin`, 70 ms su 105) e i triangoli dello zoom verticale chiamavano una funzione per ogni punto di ogni traccia anche fuori dalla finestra (55 ms). Ora i blocchi da 128 punti (`rangeExtent`, `runsAbove`) danno gli stessi valori leggendo ~1/60 dei punti; `clipY` non crea più due array per segmento (36 ms); gli scatti di rotella di un fotogramma fanno un disegno.

Il resto è `m4` (~27 ms, ~38 punti per colonna: i blocchi non aiutano) e la rasterizzazione delle 120 linee. Il cromatogramma TIC, la mappa e il passaggio del mouse erano già a 16,7 ms per fotogramma; aprire i 20 XIC: sezione seguente.

## Aprire 20 XIC su 6 file HR da 50 000 scansioni, CPU 1x (`main` `2499d61b` → `rtArr` in `cache-dati.js`; 4 aperture, cache delle risposte svuotata tra l'una e l'altra)
| azione | prima | dopo |
|---|---|---|
| aprire il pannello XIC di 20 ioni (120 tracce), ms | 1015-1225 | 558-688 |
| attività più lunga, ms | 445-585 | 103-193 |

I tempi di scansione di un file sono gli stessi per ogni ione: `xicBin` (`explore.js`) copiava in un Array gli stessi 50 000 rt per ognuno dei 20 ioni, e il garbage collector pagava ~250 ms. Ora un rt uguale bit per bit all'ultimo dello stesso file (confronto delle parole a 32 bit, ~10 volte meno di una copia) restituisce lo stesso Array; `isSorted` di `decima.js`, in cache per array, lo scorre una volta. Lo smoothing in un `Float64Array` non cambia il tempo: non fatto.

## Memoria su cicli ripetuti, un file LR sintetico (`main` `1913013` → `corrInfoWire` in `pannelli.js` chiude il vecchio ascoltatore con `abort(0)`; prova `e2e_memoria_zoom`)
| 30 cicli zoom → zoom intero dopo 5 di riscaldamento, dopo GC | prima | dopo |
|---|---|---|
| nodi DOM | 5004 → 18 084 (+436 per ciclo) | 2410 → 2410 |
| ascoltatori JS | 1213 → 3043 (+61 per ciclo) | 845 → 845 |

Ogni zoom ricostruisce l'intestazione del pannello; `abort()` senza argomento crea una `DOMException` la cui pila trattiene la NodeList dell'intestazione, e il segnale interrotto resta raggiungibile da `AbortSignal.any` sul segnale del pannello: ogni intestazione vecchia restava in memoria, senza limite. Con una ragione senza pila (`0`) nodi e ascoltatori restano fermi (sonda a 50 cicli: 2797 nodi al ciclo 20 e al 40).

Aprire, usare e chiudere 50 XIC trattiene invece ~280 nodi per ciclo nelle voci di annullamento (`storia.js`), che hanno un tetto (100 voci): si ferma a ~50 cicli, la memoria JS resta ~9 MB; nessuna modifica.

## Tabella HR di 100 000 righe nella «Statistica», CPU 1x (`tools/bench_tabella_hr.py`; `main` `3f613c149d136538f016a75257d92c341975859a` → grafico KMD denso `kmd-nuvola.js`, tabella «Dati» a blocchi, `contain:paint` sui riquadri delle tabelle; prova `e2e_kmd_denso`)
| azione | prima | dopo |
|---|---|---|
| tabella caricata e pannello dipinto: fotogramma più lungo, ms | 4211 | 423 |
| grafico di Kendrick: elementi SVG | 500 008 | 178 |
| clic su un punto (XIC, profilo, pannello), ms; attività lunghe | 9164; 2 (6987) | 99; 1 (85) |
| tabella «Dati» del grafico, ms; righe scritte | 10 595; 100 000 | 173; 1000 |

Dove andava il tempo: un elemento focalizzabile per punto (5 nodi ciascuno), il pannello ridipinto per intero a ogni clic (`PROFILO.show` riapriva la scheda già aperta) e il riquadro `overflow-x:auto` della tabella del riferimento (500 righe), che senza `contain:paint` il browser ristratificava a ogni fotogramma della pagina (`PaintArtifactCompositor::Update`, ~125 ms).
