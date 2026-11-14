# Flow: carte, viste e nomi del modello «MSn infusione»

Stato attuale di nomi, gruppi e carte di vista del Flow web. Testi del Flow solo in inglese; logica in Rust, JS collega.

## Carte e viste

- **Nomi e gruppi** (`flow-names.js`): `F.NAMES` dà un nome inglese a ogni passo (l'id non si mostra mai); `F.GROUPS` sono gruppi di Simple fuori dalla base (Export = `msn.library_export` + `msn.table_export`); `F.INFUSION_STEPS` sono i passi che leggono l'infusione di Data senza `file:` nel piano e ricevono l'arco da Raw files (`F.canvasModel`, `flow-fix.js`).
- **Carta di vista** (`F.STEP_VIEWS`): una carta `kind: "view"` (chiave `view:<istanza>`) accanto al passo, fuori dal piano (`mzflow/2` invariato); è spenta finché `S.results` non ha il risultato del suo tipo, poi apre `F.openResult` con il fuoco nella vista. Con ≤ 5 carte `F.layout` usa W 250, H 88.
- Dati dei file nei nodi (`flow-samples.js`): `F.fileNature()` legge da `E.files` polarità (se tutti uguali) e risoluzione (`prof1.hr`); `F.defaults` mostra come predefiniti `polarity` e `nominal` dei file (`F.fromFiles`), `F.addStep` scrive il preset che combacia con la risoluzione (`inheritPreset`), `F.runPlanFromData` scrive nel piano da eseguire ciò che il piano non ha scelto (`inheritNature`); la scelta a mano vince. Prova: `tests/flow_natura_file.js`.
- **Piani con passi di versione vecchia**: `F.migrated` (`flow-inputs.js`, nel nucleo) manda alla rotta `flow/line` (`F.lineCall`) ogni piano letto da file o proposto dall'assistente che nomina una versione più vecchia; il motore (`flow::migrate`) la porta alla corrente, la pagina non ha un elenco proprio. Prove: `flow_command_tests.rs` (rotta = riga di comando, byte per byte), `tests/test_flow_grafico.py`, `tests_e2e/e2e_flow_migrate.py`.
