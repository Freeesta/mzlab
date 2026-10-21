# Pacchetto «passi di Tabui in mzFlow»: prompt DEV pronto
Obiettivo: far eseguire a `mzlab-cli flow run` (e a «Run on this computer») i passi che Tabui ha e mzFlow descrive soltanto. Stato di partenza e tabella completa: `flow-copertura-tabui.md`. Regole: calcolo solo in Rust sui motori già esistenti, ogni parametro del descrittore con il suo `--set ISTANZA.NOME=VALORE`, un registro (`flow::steps()`), esecutori in `EXECUTORS`, test di parità CLI = pagina, `flow_rule_tests.rs` verde, buchi nuovi in `BUCHI`. Una PR per passo, nell'ordine sotto.

## Prompt DEV (copialo in una chat nuova)
Sei il DEV «Flow Tabui». Zona: `crates/mzlab-cli` e i motori già in `crates/mzlab-feat`. Leggi `docs/agenti/cli.md`, `rust-flow.md`, `flow-copertura-tabui.md`. Porta in `EXECUTORS` (`crates/mzlab-cli/src/flow.rs`) i passi seguenti, uno per PR, con un file `flow_<nome>.rs` + `flow_<nome>_tests.rs` ciascuno (modello: `flow_network.rs` e `flow_ms2.rs`; `flow.rs` ≤ 500 righe, funzioni ≤ 80).

Per ogni passo: (1) registra il descrittore nel registro del CLI (`flow::steps()`; i descrittori Tabui stanno in `crates/mzlab-feat/src/steps/tabui*.rs`, esportati in `docs/schemi/steps-tabui-1.json`), (2) esecutore che legge gli ingressi come file e produce un file del tipo della porta d'uscita, (3) test: stesso risultato di Tabui sugli stessi dati sintetici (mai dati del laboratorio), (4) una riga nel file d'area. Non toccare `.github/workflows`, non alzare i tetti dei `.wasm`.

## Passi, nell'ordine
1. **`table.filter`** (FATTO, `flow_table.rs`) (ingressi `table`, `samples`; uscita `table`): motore `crates/mzlab-feat/src/filters.rs` (`apply`, `from_json`, `to_json`) con ruoli da `campioni/2` (`crates/mzlab-core/src/campioni/`); il più piccolo e sblocca i modelli «Blank-corrected table» e «Standards check».
2. **`tp.generate`** (FATTO, `flow_generate.rs`) (ingressi `parent`, `features`; uscita `hypotheses`): `crates/mzlab-feat/src/hypo.rs` (`generate(rules, inp, p)`, `HypoParams::default()`, `Registry::to_json`) e le regole in `hypo/rules.rs`, `hypo/sources.rs`; ingresso `parent` come campi nome/formula neutra/SMILES (ingresso testuale o file piccolo), parametri da `HypoParams`.
3. **`tp.link`** (FATTO come `tp.link@2`, `flow_link.rs`: descrittore cambiato, ingressi `table samples parent index hypotheses`, la costruzione di `tp/1` è dentro il passo) (ingressi `table`, `spectra`, `hypotheses`; uscita `tp`): abbinamento ipotesi ↔ feature in `crates/mzlab-feat/src/steps/tabui_link.rs` (descrittore) e nel codice che costruisce `tp/1` (`interp/`, `evid.rs`); l'uscita `tp/1` è l'ingresso già accettato da `tp.network` (`flow_network.rs`).
4. **`tp.filter`** (FATTO come `tp.filter@2`, `flow_refilter.rs`: ingressi `tp` e `hypotheses`, uscita `filtri/1`; sopra l'output di `tp.link`) (ingresso `table`; uscita `table`): filtri dei prodotti di `crates/mzlab-feat/src/interp/filtri.rs` (`filtri/1`), stessa forma di `table.filter` ma sulle righe `tp/1`.
5. **`tp.position`** (FATTO, `flow_position.rs`: `SiteParams`; buco: voti dell'albero MSn) (ingressi `tp`, `parent`; uscita `sites`): `posiz/1`, vedi `docs/agenti/posizioni.md` (`chem/posizioni.rs`); prova: classi compatibili con la MS2 identiche a quelle della pagina.
6. **`features.ms2_link`** (NON FATTO per scelta: incluso in `features.detect`, non separabile senza cambiare i byte; resta in `BUCHI` con il motivo) (ingressi `index`, `peaks`; uscita `peaks`): `crates/mzlab-feat/src/ms2.rs` (`link`) e, per la bassa risoluzione, `crates/mzlab-finder-lr-wasm/src/ms2link.rs`; parità con `tests/lr_ms2link.rs`.

## Quando finisce il pacchetto
- Stato: cinque passi su sei hanno l'esecutore nativo (`table.filter`, `tp.generate`, `tp.link@2`, `tp.filter@2`, `tp.position`); `features.ms2_link` resta senza esecutore proprio, con il motivo scritto in `BUCHI`. Il piano del modello «TP time series» (`tp.generate` → `tp.link` → `tp.filter` → `table.filter` → `tp.network`) gira con `flow run` (test di cache e di rigioco come in `flow_run_tests.rs`).
- `flow-copertura-tabui.md` e `cli.md` descrivono lo stato; l'elenco dei buchi qui sotto è quello che resta.

## Buchi per modulo (oggi)
- **Senza esecutore, Tabui**: `features.ms2_link` (incluso in `features.detect`, non separabile senza cambiare i byte), `structure.render`, `structure.reactivity`; `tp.position` non rifà i voti dell'albero MSn (le righe con `source: tree` tengono il loro `site`).
- **Senza esecutore** (`err.flow.step.native`): `import.raw`, `spectra.chimeric`, `spectra.insilico`, `rt.predict`, `annot.regulatory`, `annot.hazard` (il registro di pericolosità non è in Rust).
- **Bassa risoluzione**: lo Scavo è il passo `tp.dig_lr`; i cancelli di `tp/1` usano il Da quando la tabella è a risoluzione unitaria (`TpParams.ppm < 0`), quindi `tp.link@2` gira; `tp.generate` usa ancora le regole di HR (`regole_tp_2.json`), diverse dal Dig.
- **Mancano come passi**: albero/grafo MSn (`msngrafo/1`), centroidazione prima di `features.detect` (`lettori_centroidi.rs`), ricerca per nome del composto, avanzamento in diretta.
- **Sito**: nessun passo è eseguibile dal browser (manca la terza voce di `F.runners`); il sito costruisce il piano e dà il comando da copiare.
