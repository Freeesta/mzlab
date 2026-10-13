# mzFinder bassa risoluzione in Rust

## Verdetto ISF delle righe LR (`crates/mzlab-feat/src/legacy/lrisf/`)
- Funzioni pure con parità ≤ 1e-9 a `TP_Mine/py/tpmine/isf.py` (riferimento `tools/finder_lrisf_riferimento.py`, dati d'oro `tests/golden/finder_lrisf.json`, test `crates/mzlab-feat/tests/legacy_lrisf.rs`; `legacy_lrisf_eval.rs` prova `isf_eval` su un indice sintetico). Il verdetto (probabilità) vive solo qui, mai in `mzlab-core`. I parametri del modello (`isf_model.MODEL`) sono costanti in `isf.rs`; `calibrate`, `fit_logistic` e `families` (MCR-ALS) non si portano.
- Interfaccia per il Dig LR: `isf_eval(samples: &mut [IsfSample], parent_mz, ions: &[f64], formula_p: Option<&str>) -> Vec<Option<IsfVerdict>>` = `classify_ions(..., fam=False)`, un esito per ione nell'ordine dato, `None` se l'ione non si analizza (mai panico). 
- `IsfSample` = `mzlab_core::origin::Sample` (etichetta, tempo in minuti, indice `.mzidx` e sezione full scan): è ciò che la pipeline ha per ogni file. 
- `IsfVerdict` = `row.isf` (`p_inherited`, `isf_ness`, `probs`, `doubtful`, `reasons`, `explanation`, `calibrated`); `row.insource` lo calcola il chiamante (`probs.isf ≥ 0.5` e ione più leggero).
- `classify_report(report, formula_p, ramp) -> Option<Value>`: il JSON completo di `isf.classify_report` (con `evidence`, `notes`, `features`); contributi MS2 e rampa DP con peso euristico, `calibrated: false`. `isf_split.rs` = ruoli per differenza di massa (più pesante del progenitore non è un frammento), `isf_text.rs` = testi in inglese e formati numerici di Python.

## Albero LR «supposto» e confronto (`lrisf/tree.rs`, `compare.rs`)
- `lr_tree(ms2: Option<&ParentMs2>, mz_x, ion_formula: Option<&str>, smiles: Option<&str>) -> String` = `fragtree.lr`: sopra `fragtree::two_level` (stesso codice dell'HR) con `ppm = 0`, tolleranza `LR_TOL = 0.5` Da, `supposed: true`, `mode: "lr"`. 
- `ParentMs2` = frammenti (m/z, intensità: i più forti, ≥ 5 %, al massimo 12), numero di scansioni e m/z del precursore (0 o assente → `mz_x`). `ion_formula` è la formula dello ione [M+H]+ (`None` per altri addotti o senza formula: allora le alternative sono le sole perdite comuni). 
- Senza MS2 del progenitore: `{"ok": false, "mode": "lr", "note": "no MS2 of the parent in the files", "nodes": []}`. 
- `resolve_smiles(campo, testo neutro)` dà lo SMILES come nel Python (il testo neutro vale come SMILES se non è una semplice formula alfanumerica).
- `compare_lr(tree: &Value, tp_mz: &[f64], dm: Option<f64>) -> String` = `compare_lr` + `_tally`: un frammento del prodotto entro 0.5 Da dal nodo è `same`, entro 0.5 Da dal nodo più `dm` è `shifted`; senza frammenti `{"ok": false, "note": "no MS2 of the product", "marks": {}}`.
- Dati d'oro: chiavi `tree` e `compare` di `tests/golden/finder_lrisf.json` (il riferimento chiama `fragtree.lr` con un esperimento finto). Un elemento fuori dall'elenco HR fa sollevare il Python; il Rust disegna l'albero senza formule.
- Dimensioni di prova (wasm32, `opt-level z`, lto, senza `wasm-opt`; base con solo il lettore JSON 88 KB): `classify_report` +40 KB, `lr_tree` + `compare_lr` +112 KB (grafo chimico e spazio delle formule), `isf_eval` con `origin_report` di `mzlab-core` +145 KB; tutto insieme 378 KB.
