# Modulo «aperti»: formati aperti (mzML con numpress, poi mzXML e netCDF)

Famiglia `aperti` (`crates/mzlab-lettori/src/aperti/`, feature `aperti`, tetto 393 216 B, `.wasm` in `wasm-lettori/aperti/`). Contratto dei lettori e caricamento a richiesta: `lettori-modulo.md`, `estendere.md`.

## mzML con MS-Numpress (`aperti/mzml_numpress.rs`)
- `numpress.rs` (`mzlab-core`, solo con `estesi`, fuori dal motore): decodifica `Np::{Linear, Pic, Slof}` (MS:1002312/13/14, e le forme con zlib 1002746/47/48 via `of_accession`); `decode(np, byte, atteso)` limita i valori a `atteso` (8 M se 0) e dà `err.file.numpress`; `decode_text` aggiunge base64 e zlib limitato (`error.memoryBudget` oltre `max_bytes`); `plain_base64` scrive f64 piani; i codificatori `encode_linear/pic/slof` servono ai test.
- Il motore resta com'è: `mzml.rs` rifiuta il numpress (`err.file.numpress`) e il suo `.wasm` non cambia (stesso SHA-256). Il modulo riscrive gli array di UN spettro/cromatogramma come f64 piani (`aperti/np_riscrivi.rs`: `riscrivi`) e li dà a `mzml::parse` / `Mzml::chromatogram`.
- `VOCE` `id "mzml-numpress"`, `.mzml`: `riconosci` vuole `<mzML` e una delle sei accession nei primi 64 KB (`Fiducia::Si`). `apri` fa un solo passaggio sugli offset (`scantable::read`), controlla `start < end ≤ dimensione` e `Limiti::voci_ok`; `scansione(c, k)` legge solo i byte di `k`.
- `cromatogrammi`/`cromatogramma`: come `mzmlhead.rs`, sui byte riscritti del cromatogramma. Lo scanner di tag, i parametri dello spettro e la lettura della testata originale stanno in `aperti/mzml_testata.rs`.
- `testata_mzml` = testata originale (strumento, seriale, componenti, sorgenti con SHA-1, software, data) con mzLab come convertitore; `metodo` ripete il documento `mzLab metodo/1` se il file lo porta, altrimenti `{}`.
- Pagina: `LETTORI_FAM.aperti` (`lettori-famiglie.js`) e `LETTORI_MOD.variante` (`lettori-modulo.js`: guarda i primi 64 KB di un `.mzML`) fanno arrivare al modulo, da `moduloWants` (`shell.js`), gli mzML che portano numpress; gli altri vanno al motore.
- Prove: `crates/mzlab-lettori/tests/aperti_numpress.rs` (file sintetici; costruttori in `tests/comune/`), `aperti_numpress_ostili.rs` (casi ostili e dati veri se c'è `MZLAB_DATI`), unit in `numpress/tests.rs`, `tests_e2e/e2e_lettori_modulo.py` (mzML numpress e gemello zlib).
- Limiti noti: i cvParam dello spettro che la riscrittura non conosce diventano userParam «ACC nome» nell'mzML riscritto; `dataProcessing`, `referenceableParamGroup` e `scanSettings` non passano (`grezzi-non-estratti.md`).
