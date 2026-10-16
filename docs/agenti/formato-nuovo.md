# Derivati di formato più nuovo (`error.newerFormat`)
- Un `.mzidx` con versione di intestazione sopra `idxnuovo::VERSION_MAX` (o magic `MZIDX00N`, N > 1) e un `.mzfeat`/`.mzfeatf` con `version` o `feat_version` sopra quelli noti (o magic `MZFEAT0N`, N > 1) rispondono `error.newerFormat {kind, version}`; `error.badIndex`/`error.badMzfeat` restano per il file danneggiato.
- Il file non si apre con questa versione, non si ricostruisce e non si sovrascrive: `archivio.promote` rifiuta di sostituirlo (`manifesto.js`: `MASSIMI`, `nuovo`), `lrPut` non ne toglie l'indice, `lr_index` risponde con l'avviso (it/en) senza costruire; il sorgente resta caricato.
- Una versione minore si ricostruisce dal sorgente come sempre (`pulizia.js`, `vecchio`).
- I `.mzfeat`/`.mzfeatf` scritti da `feat-worker.js` (mzFinder, nascosto) non passano da `archivio.promote`: nessuna guardia in scrittura.
- Prova: `crates/mzlab-core/tests/formato_nuovo.rs`, `crates/mzlab-feat/tests/formato_nuovo.rs`, `tests_e2e/e2e_formato_nuovo.py`.
