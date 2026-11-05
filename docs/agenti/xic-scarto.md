# Scarto con segno nella finestra «Estrai ioni» (XIC)
- Un numero con segno in coda al testo di uno ione, `288.2+16` o `C9H10Cl2N2O-18`, si somma all'm/z dopo l'addotto come Da letterali (+16 = 16,000, non l'ossigeno). Un testo che finisce con un segno o con lettere (`+H`, `[M+H]+`) non ha scarto.
- Il lettore sta in Rust: `elements::split_offset`, usato da `elements::formula_mz` (`api/formula`), che aggiunge la chiave `offset`. Un m/z nudo con scarto risponde senza formula (`formula` vuota). Scarto illeggibile (`+1.2.3`, `+16x`, `288.2+`): `err.formula.offset`.
- Sito e app danno gli stessi byte: casi in `crates/mzlab-desktop/tests/parita_rotte.rs` (`lr_formula`); prove del lettore in `crates/mzlab-core/tests/scarto_mz.rs`.
- CLI: `mzlab-cli ion-mz TESTO [--adduct A] [--out PATH]` (`ion_mz_cmd.rs`) stampa il JSON di `api/formula`; scarto illeggibile = uscita 2.
- Interfaccia: `evalRow` di `openXic` (`pannelli.js`) mostra «base scarto Da → m/z candidato», mai un'identificazione; i testi sono `xic.shifted` e `err.formula.offset`. Prova: `tests_e2e/e2e_xic_scarto.py`.
- Il dialogo risolve solo m/z e formule: i nomi di composto non vi sono cercati.
