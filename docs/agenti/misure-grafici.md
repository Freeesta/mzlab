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

Un file da 10^6 punti (stessa macchina): draw della vista intera 11 ms, dello zoom 0,9 ms; rotella, trascinamento e passaggio del mouse a 16,7 ms per fotogramma senza attività lunghe; ridimensionare ×10 = 15 chiamate a `draw()` (fotogrammi fino a 50 ms).

## Memoria su cicli ripetuti, un file LR sintetico (`main` `1913013` → `corrInfoWire` in `pannelli.js` chiude il vecchio ascoltatore con `abort(0)`; prova `e2e_memoria_zoom`)
| 30 cicli zoom → zoom intero dopo 5 di riscaldamento, dopo GC | prima | dopo |
|---|---|---|
| nodi DOM | 5004 → 18 084 (+436 per ciclo) | 2410 → 2410 |
| ascoltatori JS | 1213 → 3043 (+61 per ciclo) | 845 → 845 |

Ogni zoom ricostruisce l'intestazione del pannello; `abort()` senza argomento crea una `DOMException` la cui pila trattiene la NodeList dell'intestazione, e il segnale interrotto resta raggiungibile da `AbortSignal.any` sul segnale del pannello: ogni intestazione vecchia restava in memoria, senza limite. Con una ragione senza pila (`0`) nodi e ascoltatori restano fermi (sonda a 50 cicli: 2797 nodi al ciclo 20 e al 40).

Aprire, usare e chiudere 50 XIC trattiene invece ~280 nodi per ciclo nelle voci di annullamento (`storia.js`), che hanno un tetto (100 voci): si ferma a ~50 cicli, la memoria JS resta ~9 MB; nessuna modifica.
