# Numeri (`mzlab/web/numeri.js`)
Caricato prima di `grafici.js`; lo usano `picchi.js`, `liste.js`, `banco.js`, `conversione.js`. Prova: `tests/test_perf_web10.py`, `e2e_spettro_grande`.

- `maxOf(a, init)`, `minOf(a, init)`, `maxAbs(a, init)`: estremi di un array (anche tipizzato) senza spread; un NaN dà NaN come `Math.max`; `init` vale per l'array vuoto. `Math.max(...a)` su ~125 000 elementi dà RangeError: nei file nuovi si usano questi.
- `lowerIdx(a, v)`: primo indice con `a[i] >= v` in un array crescente.
- `spanOf(a, lo, hi)`: intervallo `[i0, i1)` che contiene ogni indice con `lo <= a[i] <= hi`: ricerca binaria se `a` è crescente (controllo una volta per array, in una WeakMap), altrimenti tutto l'array. I cicli chiamanti tengono i propri test esatti, quindi c'è un solo percorso e i pixel non cambiano.
- `b64F32(str, n)`: i float32 di un payload base64 delle rotte mappa (`getMap` in `grafici.js`); un payload non base64, di lunghezza non multipla di 4 o con un numero di valori diverso da `n` (nrt×nmz) lancia un `Error` chiaro invece di un RangeError.
