# Ingresso di mzFinder: un `.mzidx` per ogni file e la lista di trasformazioni in Rust
Stato attuale. Aree vicine: `mzfeat-wasm.md` (i `.wasm` e il pool), `mzidx.md` (formato).

## Indice di ogni file (`TP_Mine/files/finder-idx.js`)
- Il worker `tpmine-worker.js` tiene in `held` il Blob o l'ArrayBuffer di ogni file che la pagina gli manda (`put`). Prima di `classify` chiama `ensureIdx(nomi)`: per ogni file mandato senza `.mzidx` in OPFS `mzlab/idx/` (o con un indice che registra una dimensione diversa nel `meta.source.size`) lo costruisce.
- Il costruttore è quello del programma principale: `prepare_upload` di `wasm/mzlab_wasm.js` (tabella delle scansioni, il `.wasm` compilato una volta), il pool di `idxpool.js` / `idx-worker.js` (un file per worker, a fette dal Blob) e `ARCH.promote` (da `<nome>.mzidx.mzf.part` al nome vero). Nessun indicizzatore nuovo: i byte sono quelli che scriverebbe la scheda Dati (`tests_e2e/e2e_tpmine_ingresso.py` li confronta).
- Un campione di un file del produttore (`.raw`, `.wiff`, `.D`: in `files/` è un piccolo segnaposto, `lettura-nativa.md`) entra in mzFinder con «Reload from Data» tramite `nativeMzml` (`20_tpmine.js`): il suo mzML si scrive in quel momento (`rb.nativeToMzml`) e segue la via di un mzML qualsiasi.
- I file già indicizzati dalla scheda Dati (presi con «Reload from Data») passano senza lavoro. Se la costruzione fallisce (niente OPFS, disco pieno) `classify` ricade su Pyodide come prima.
- Il worker tiene ancora `puts` per Pyodide: servono al motore a bassa risoluzione finché esiste.

## Lista di trasformazioni dell'utente (`legacy/transf.rs`, `finder_transformations`)
- Il testo della lista (righe `nome;variazione[;nota]`, `#` commenti, `lit:` = livello 2) lo legge `crates/mzlab-feat/src/legacy/transf.rs` con le regole del Python (`chem.parse_transformations`, `hr/network.py::transformation_table`: un vettore uguale tiene il primo nome). `finder_transformations(regole, testo)` in `crates/mzlab-feat-wasm/src/finder.rs` fonde il testo nel `regole_tp.json` e restituisce il nuovo JSON; una riga sbagliata dà l'errore con il testo del Python.
- `tpmine-worker.js` (`runRust`) lo applica solo se il testo non è vuoto e passa le regole fuse a `hypotheses`: il testo di default non cambia le regole, quindi il Dig è identico. Parità: `tests/golden/finder_transf.json` (da `tools/finder_transf_riferimento.py`), `crates/mzlab-feat/tests/legacy_transf.rs`, `tests_e2e/e2e_tpmine_transf.py`.
