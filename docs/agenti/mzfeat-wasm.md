# mzFinder nel browser: `crates/mzlab-feat-wasm` e `TP_Mine/files/feat-worker.js`

Lo stato attuale del ponte WASM della pipeline L1-L9 di `mzlab-feat` (formati e livelli: `mzfeat.md`).

## Il crate
- `mzlab-feat-wasm` (cdylib + rlib) espone con wasm-bindgen: `detect_file(read, size, name, recipe)` → byte `.mzfeatf`; `align_series(files, read, sizes, recipe)` → byte `.mzfeat`; `run_hypotheses(files, read, sizes, rules, parent, parent_rt, recipe)` → oggetto `Hypotheses` (`registry_json`, `tried`, `found`); `build_evidence(table, hypo, minutes, parent_mz, parent_rt, recipe)` → `.mzfeat` con la sezione `evid`; `version()`.
- `read(k, offset, len)` è una funzione JS che torna un `Uint8Array` (nel worker: handle sincrono OPFS); `NaN` = nessun valore per `parent_rt` e per i minuti (controllo). `recipe` è JSON `{"polarity":1|-1|null,"mode":"ion"|"family"}`. Errori: JSON `{error_key, params}` (`error.badRecipe` e le chiavi di `mzlab-feat`).
- Due `.wasm` separati dalle feature: `detect` (L1-L5, 324 KB dopo `wasm-opt -Oz`) e `proofs` (L6-L9, 349 KB); il limite di CI è 400 KB l'uno (`tools/build_feat_wasm.sh`, che scrive anche `wasm-feat/manifest.json` con gli SHA-256). Nel grafo WASM non compaiono `memmap2`, `zstd-sys`, `libz-sys`, `cc` (controllo in `rust.yml`).
- `mzlab-feat/src/sortc.rs`: ordinamenti stabili compatti usati dai moduli che girano nel `.wasm` (gli `slice::sort*` istanziati per ogni tipo pesavano 5-10 KB l'uno); non toccano `evid*`, `hypo*`, `kinetic*`, `suspects`.
- `src/run.rs` contiene la pipeline generica su `ReadAt`, condivisa con l'esempio nativo `examples/nativo.rs` (riferimento: stampa `nome sha256` di ogni file). Lacuna nota: gli archi delle famiglie non stanno nel `.mzfeatf`, quindi `build_evidence` li passa vuoti.

## Il worker (`TP_Mine/files/feat-worker.js`)
- Un file, due ruoli. Lato pagina `createFeatPool({workerUrl, wasm:{feat,proofs}, size, notify, onProgress})`: `detectAll(files, recipe)`, `align(stored, paths, recipe)`, `proofs({...})` (ipotesi + evidenze nello stesso worker), `fs.write/read/exists/remove` (worker d'archivio, scrive `nome.part` poi `move()`), `cancel()`, `resume()`.
- Supervisione (come `idxpool.js`): un worker che muore (ucciso o trappola del `.wasm`) viene ricreato, il lavoro riparte una volta con `notify("tpmine.feat.restarted")`; alla seconda morte il lavoro è rifiutato con `error.featWorker`. Con `sha256` nel manifest il `.wasm` scaricato è verificato (`error.featWasmHash`).
- I file `.mzidx` si leggono da OPFS (`mzlab/idx/<nome>.mzidx`) con handle sincroni aperti per operazione; un handle di un worker appena morto è atteso fino a 2 s.
- Chiavi i18n da aggiungere nei cataloghi: `tpmine.feat.restarted`, `tpmine.feat.cancelled`, `error.featWorker`, `error.featWasmHash`, `error.featNoHypotheses`, `error.featFailed`, `error.badRecipe`.

## Prove
- `tests/test_feat_wasm.py` (+ `tests/feat_wasm_node.js`): `.mzfeatf`, `.mzfeat` ed `evid` del WASM hanno lo stesso SHA-256 del nativo sulla serie sintetica `piccolo`.
- `tests_e2e/e2e_feat_worker.py`: costruisce i `.wasm`, pool su OPFS, worker ucciso e trappola, archivio senza `.part`, dimensioni.
