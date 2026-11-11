# File di prodotto a un livello MSn diverso da 2 (MS3)

Codice: `iteminfo.rs` (`msn_level`), `axisroutes.rs` (`ms2_transitions(level)`), `itemroutes.rs` (`item_method`), `explore.js` (`lv`, `getMs2Tr`), `titoli.js`, `pannelli.js`, `tabs.js`, `eventi.js`, `hr.js`. Prove: `crates/mzlab-core/tests/ms3_only.rs` (feature `sciex`, sulla coppia sintetica `ms3` di `tools/wiff_riferimento.py`), `tests/test_titoli.py`, `tests_e2e/e2e_ms3.py` (file di `ms3` in `tools/dati_sintetici_msn.py`).

- **Livello del file**: l'info di un file (`item_info`) ha `msn_level` = il livello più basso fra le sue scansioni di livello ≥ 2 (`null` se non ne ha); un file di sole MS3 ha `kind` `ms2` e `msn_level` 3. L'intervallo m/z del prodotto (`mz2_min`/`mz2_max`) viene dalle scansioni di quel livello.
- **Interfaccia**: la scheda dei prodotti usa `lv` = `msn_level` del file (2 se manca): cromatogramma del precursore, lista delle scansioni, spettro e tabella dei precursori (`/api/ms2tr?level=`, 2 se manca) chiedono la sezione di quel livello, mai una sezione MS2 che il file non ha. Il menu del livello dello spettro offre «MS3» per un file di sole MS3.
- **Titoli**: con `lv` 3 i titoli dicono «TIC · MS³», «TIC · MS³ (tutti i precursori)», «TIC · MS³ · precursore {prec}», «MS³ di {prec}» (le chiavi `panel.title.ticMs2`, `ticMs2All`, `ms2prec`, `productOf` hanno `{ms}` = «MS²» o «MS³»).
- **Precursore**: di una scansione MS3 è la seconda massa isolata (quella del lettore), come nell'mzML scritto; la prima è un userParam del lettore e non entra nei menu.
- **Metodo** (`/api/method`): i tempi dei cicli di un file di prodotto si contano sulle scansioni del suo livello più basso ≥ 2.
- **Fuori da qui**: un file con MS2 e MS3 insieme mostra il livello più basso (2); le librerie e l'identificazione restano su spettri MS2.
