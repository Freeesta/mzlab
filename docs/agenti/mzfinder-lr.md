# mzFinder bassa risoluzione in Rust

## Verdetto ISF delle righe LR (`crates/mzlab-feat/src/legacy/lrisf/`)
- Funzioni pure con parità ≤ 1e-9 a `TP_Mine/py/tpmine/isf.py` (riferimento `tools/finder_lrisf_riferimento.py`, dati d'oro `tests/golden/finder_lrisf.json`, test `crates/mzlab-feat/tests/legacy_lrisf.rs`; `legacy_lrisf_eval.rs` prova `isf_eval` su un indice sintetico). Il verdetto (probabilità) vive solo qui, mai in `mzlab-core`. I parametri del modello (`isf_model.MODEL`) sono costanti in `isf.rs`; `calibrate`, `fit_logistic` e `families` (MCR-ALS) non si portano.
- Interfaccia per il Dig LR: `isf_eval(samples: &mut [IsfSample], parent_mz, ions: &[f64], formula_p: Option<&str>) -> Vec<Option<IsfVerdict>>` = `classify_ions(..., fam=False)`, un esito per ione nell'ordine dato, `None` se l'ione non si analizza (mai panico).
- `IsfSample` = `mzlab_core::origin::Sample` (etichetta, tempo in minuti, indice `.mzidx` e sezione full scan): è ciò che la pipeline ha per ogni file.
- `IsfVerdict` = `row.isf` (`p_inherited`, `isf_ness`, `probs`, `doubtful`, `reasons`, `explanation`, `calibrated`); `row.insource` lo calcola il chiamante (`probs.isf ≥ 0.5` e ione più leggero).
- `classify_report(report, formula_p, ramp) -> Option<Value>`: il JSON completo di `isf.classify_report` (con `evidence`, `notes`, `features`); contributi MS2 e rampa DP con peso euristico, `calibrated: false`. `isf_split.rs` = ruoli per differenza di massa (più pesante del progenitore non è un frammento), `isf_text.rs` = testi in inglese e formati numerici di Python.
