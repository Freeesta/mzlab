# Ingresso di mzFinder: un `.mzidx` per ogni file e la lista di trasformazioni in Rust
Stato attuale. Aree vicine: `mzfeat-wasm.md` (i `.wasm` e il pool), `mzidx.md` (formato).

## Indice di ogni file (`TP_Mine/files/finder-idx.js`)
- Il worker `tpmine-worker.js` tiene in `held` il Blob o l'ArrayBuffer di ogni file che la pagina gli manda (`put`). Prima di `classify` chiama `ensureIdx(nomi)`: per ogni file mandato senza `.mzidx` in OPFS `mzlab/idx/` (o con un indice che registra una dimensione diversa nel `meta.source.size`) lo costruisce.
- Il costruttore è quello del programma principale: `prepare_upload` di `wasm/mzlab_wasm.js` (tabella delle scansioni, il `.wasm` compilato una volta), il pool di `idxpool.js` / `idx-worker.js` (un file per worker, a fette dal Blob) e `ARCH.promote` (da `<nome>.mzidx.mzf.part` al nome vero). Nessun indicizzatore nuovo: i byte sono quelli che scriverebbe la scheda Dati (`tests_e2e/e2e_tpmine_ingresso.py` li confronta).
- I file già indicizzati dalla scheda Dati (presi con «Reload from Data») passano senza lavoro. Se la costruzione fallisce (niente OPFS, disco pieno) `classify` ricade su Pyodide come prima.
- Il worker tiene ancora `puts` per Pyodide: servono al motore a bassa risoluzione finché esiste.
