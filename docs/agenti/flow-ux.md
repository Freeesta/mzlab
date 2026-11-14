# Flow: carte, viste e nomi del modello «MSn infusione»

Stato attuale di nomi, gruppi e carte di vista del Flow web. Testi del Flow solo in inglese; logica in Rust, JS collega.

## Carte e viste

- **Nomi e gruppi** (`flow-names.js`): `F.NAMES` dà un nome inglese a ogni passo (l'id non si mostra mai); `F.GROUPS` sono gruppi di Simple fuori dalla base (Export = `msn.library_export` + `msn.table_export`); `F.INFUSION_STEPS` sono i passi che leggono l'infusione di Data senza `file:` nel piano e ricevono l'arco da Raw files (`F.canvasModel`, `flow-fix.js`).
- **Carta di vista** (`F.STEP_VIEWS`): una carta `kind: "view"` (chiave `view:<istanza>`) accanto al passo, fuori dal piano (`mzflow/2` invariato); è spenta finché `S.results` non ha il risultato del suo tipo, poi apre `F.openResult` con il fuoco nella vista. Con ≤ 5 carte `F.layout` usa W 250, H 88.
- Dati dei file nei nodi (`flow-samples.js`): `F.fileNature()` legge da `E.files` polarità (se tutti uguali) e risoluzione (`prof1.hr`); `F.defaults` mostra come predefiniti `polarity` e `nominal` dei file (`F.fromFiles`), `F.addStep` scrive il preset che combacia con la risoluzione (`inheritPreset`), `F.runPlanFromData` scrive nel piano da eseguire ciò che il piano non ha scelto (`inheritNature`); la scelta a mano vince. Prova: `tests/flow_natura_file.js`.
- **Piani con passi di versione vecchia**: `F.migrated` (`flow-inputs.js`, nel nucleo) manda alla rotta `flow/line` (`F.lineCall`) ogni piano letto da file o proposto dall'assistente che nomina una versione più vecchia; il motore (`flow::migrate`) la porta alla corrente, la pagina non ha un elenco proprio. Prove: `flow_command_tests.rs` (rotta = riga di comando, byte per byte), `tests/test_flow_grafico.py`, `tests_e2e/e2e_flow_migrate.py`.

## Grafica del Flow (regole della tela)
- **Tela**: scritta da noi in JS semplice (DOM per i nodi, SVG per i fili, mai `<canvas>`, che perde tastiera e lettori di schermo); nessuna libreria di nodi e nessuna riga copiata da strumenti di nodi, neanche MIT. Compatibilità, conversioni e ordine li decide il controllo del piano (`flow::check` in Rust, `flow-check.js` uguale sui casi di prova), la tela colora la risposta; una conversione mancante si propone con un clic, mai inserita da sola.
- **Porte** (`F.TYPES` di `flow/lazy/flow-registry.js`): ogni tipo di dato = colore Okabe-Ito da variabile CSS + forma della famiglia + lettera, mai il solo colore; i file salvati portano i tipi, non i colori, quindi cambiarli è facile. Tutto funziona anche senza trascinare (clic-clic, tastiera, elenco dei passi). Obiettivo di una catena (`F.GOAL_STEPS` di `flow-plan.js`, facile da cambiare): il passo che gradua un'identificazione, `annot.level`.

## Aperto: un Flow più semplice per i TP
- **Non costruito**: cinque carte da sinistra a destra (Parent → Data → Features → Candidate TPs → Evidence & decisions, più Export), la lista dei campioni non più carta ma «Advanced» dentro Data; il modello «Find transformation products» che sceglie `tp.dig_lr` sui file a risoluzione unitaria (usa `tp.link`, solo HR); i tempi letti dai nomi e non confermati come avvisi, non errori.
- **Domande aperte**: nomi delle carte; avvisi o errori (gravità di `valida` in Rust); `tp.filter`, `tp.network`, `tp.position` accesi nel modello HR; carta Evidence nel mondo LR; file del rapporto.
