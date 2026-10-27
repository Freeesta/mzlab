# Parità sito ↔ app: cosa c'è da una parte sola

Regola: ogni funzione esiste sia nel sito (`.wasm` + OPFS) sia nell'app installata (desktop Tauri o `mzlab-cli` nativo), con un test di parità byte per byte; se una metà è impossibile lo si dichiara. Questo studio elenca le disparità di oggi (base: `origin/main` `afc144c`). I formati degli strumenti sono già in `formati-matrice.md`: qui solo una riga di sintesi per famiglia.

## Come è stato fatto
- Passi del Flow: `mzlab-cli steps --json` (48 passi, campo `executable_native` e `runs_in.wasm` del descrittore) contro gli esecutori del sito `F.liveJobs` (`flow/lazy/flow-live-jobs.js:240`); un passo senza esecutore nel sito finisce `blocked` per `platform.computer` (`flow/lazy/flow-live.js:145`).
- Motore: le 59 operazioni di `mzlab/web/rust-worker.js` contro quelle che il desktop serve (`crates/mzlab-desktop/shim/desktop-worker.js`, `src/rpc.rs`, `rpc_lr.rs`, `rpc_nat.rs`); un'operazione sconosciuta dà `rpc.unknown` (`crates/mzlab-desktop/src/rpc.rs:212`).
- Sottocomandi: `mzlab-cli --help` (`crates/mzlab-cli/src/lib.rs:389`), comandi Tauri di `crates/mzlab-desktop/src/main.rs`.
- Test di parità: solo quelli che confrontano `.wasm` (via Node o pagina) con il nativo. I `crates/mzlab-core/tests/*_parity.rs` confrontano Rust con il riferimento Python, non il sito con l'app.
- «Non verificato» = il codice suggerisce la disparità ma nessuna prova l'ha osservata.

## Legenda
- **App/CLI**, **Sito**: sì / no / parziale. «pannello» = la funzione c'è nel sito in un pannello (Tabui, mzFinder, Attributio) ma non come passo del Flow.
- **Perché**: *limite* = limite tecnico vero; *da fare* = non ancora fatto; *scelta* = decisione di progetto.
- **Diff.**: S (≤ 1 giorno, solo collegamento), M (qualche giorno), L (settimane o decisione).
- **Prio**: 1 = subito, 2 = presto, 3 = quando serve, — = nessuna azione (n/a o dichiarato impossibile).

## Tabella

| Funzione | App/CLI | Sito | Test di parità | Perché manca | Diff. | Prio |
|---|---|---|---|---|---|---|
| **Flow: passi della catena** | | | | | | |
| `features.detect` / `align` / `gapfill` | sì | sì | sì: `e2e_flow_web.py` (SHA-256 di `series.mzfeat` = esempio `nativo`), `test_feat_wasm.py` | — | — | — |
| `index.mzidx` | sì | sì (riusa l'indice di Dati) | parziale: `mzidx_parity.rs` è Rust↔Python; nessun confronto `.mzidx` sito↔app | da fare (test) | S | 2 |
| `spectra.centroid` | sì | parziale: file del produttore sì (`lr_index_cen`); mzML in profilo `blocked` (`flow-live-jobs.js:35`) | no | da fare: `mzml2cen` esiste già nel worker (`rust-worker.js:409`, `rust-centroidi.js:16`) ma il Flow non lo usa | S | 1 |
| `features.estimate` | sì | no (la ricetta stimata c'è dentro `recipe_for`, non come passo) | no | da fare | S | 3 |
| `features.table` (catena di `mzlab-cli features`) | sì | no come passo; sì come catena detect+align+gapfill | indiretto (`e2e_flow_web.py`) | scelta: alias della CLI | S | 3 |
| `import.raw`, `convert.mzml` | sì | no come passo; sì da Dati («Scarica .mzML», moduli lettori) | no | da fare | M | 2 |
| `features.ms2_link` | no | no | — | scelta: dentro `features.detect` (`flow_rule_tests.rs:13`, `BUCHI`) | — | — |
| **Flow: tabella** | | | | | | |
| `table.filter`, `table.drift`, `table.plausible` | sì | pannello per plausibilità (`.wasm` `table`: `table_plausible`, `table_check`) e deriva (`drift_series`, `feat-worker.js:411`); filtro non verificato | `test_feat_table_wasm.py` per `check.json`; passi no | da fare: nessun esecutore in `F.liveJobs` | S | 1 |
| `table.stats`, `table.pca`, `table.kmd` | sì | pannello (motore: `stat_post`, `kmd_post`; vista KMD/PCA) | `stat_route_parity.rs` è Rust↔Python | da fare | M | 2 |
| `export.table_csv`, `export.inclusion`, `export.mztab` | sì | pannello (`export_table`, inclusione in mzFinder); mzTab del passo no | no | da fare | M | 1 |
| `table.match`, `meta.compare` | sì | no | no | da fare | M | 3 |
| **Flow: identificazione (Attributio)** | | | | | | |
| `formula.ms1` | sì | pannello (`untargeted_formula_id`) | `untargeted_wasm.rs` + `test_feat_wasm_untargeted.py` (detect/align), formula no | da fare | M | 2 |
| `formula.ms2`, `spectra.chimeric`, `spectra.insilico`, `spectra.ms2` | sì | pannello (`.wasm` `attributio-all`: `ms2_to`) | `parita_attributio_ms2.json` (non verificato se sito↔app) | da fare | M | 2 |
| `rt.predict`, `annot.level` | sì | pannello (`rt_descriptors`, `annotate_to`) | `test_attributio_descr_wasm.py` (descrittori a 1e-9) | da fare | M | 2 |
| `spectra.match` | sì (tutti gli algoritmi) | parziale: solo `tree` (`flow-live-jobs.js:142`); libreria spettrale nel pannello (`mzlab-lib-wasm`) | `test_librerie_fdr_wasm.py`; `e2e_flow_msn.py` per `tree` | da fare | M | 2 |
| `spectra.lookup` | sì | no | no | da fare | M | 3 |
| `annot.regulatory`, `annot.hazard` | no | no | — | da fare in entrambi (solo descrittore, `BUCHI`) | M | 3 |
| **Flow: TP e MSn** | | | | | | |
| `tp.generate`, `tp.link@2`, `tp.filter@2`, `tp.position`, `tp.network`, `tp.review`, `tp.dig_lr` | sì | sì | sì: `e2e_flow_tp_web.py`, `e2e_flow_tp_link.py`, `e2e_flow_tp_post.py`, `e2e_flow_tp_review.py`, `e2e_flow_dig_lr.py` contro `flow run` | — (avviso `warn.flow.link.noT0` non riportato dal sito) | S | 3 |
| Modello «Bentazone / TP» (`flow-names.js:98`) | sì | sì (tutti i suoi passi hanno esecutore) | indiretto (passi sopra) | — | — | — |
| `spectra.msn_graph`, `msn.library_export`, `msn.table_export` | sì | sì | sì: `test_msngrafo_wasm.py`, `e2e_flow_msn.py`, `e2e_msngrafo_nativo.py` | — | — | — |
| `msn.figure` | sì | no come passo (figura nella scheda) | `msn_figura_parita.js` (non verificato su quale coppia) | da fare | S | 3 |
| `structure.render`, `structure.reactivity` | sì | pannello (`feat-worker.js`, `.wasm` `chem` `reactivity`) | `crates/mzlab-desktop/tests/reattivita.rs` (app ↔ funzione del `.wasm`) | da fare come passo | S | 3 |
| `structure.draw` (Ketcher) | no | sì | — | limite: editor JS in iframe; un disegno non è un calcolo | — | — |
| `chrom.uv_peaks`, `quant.calibrate` | sì | no | no | da fare | M | 3 |
| **Flow: registro e controllo** | | | | | | |
| `prov/1` e `corsa/1` di una corsa | sì (`flow/run`, `flow/live finish`, `flow report`) | no: `live.run` non scrive `prov/1` (`flow-live.js:303`); il rapporto si apre solo da file | `test_corsa_schema.py` (solo nativo) | da fare: lo scheduler nel `.wasm` non espone `finish` | M | 1 |
| Controllo del piano (`flow check`) | sì (Rust `check.rs`) | sì ma in JS (`flow/lazy/flow-check.js:2`) | `test_flow_grafico.py` (casi JS↔Rust) | da fare: viola «Tutto in Rust»; va nel motore come `flow_line` | M | 2 |
| Liste dell'utente (`flow/liste`, `liste/1`) | sì (app) | no: campo di testo (`flow-live.md`, «Liste dell'utente») | `e2e_flow_liste.py` (rotta finta) | da fare: `mzlab_feat::liste::route` non è nel `.wasm` | S | 2 |
| Riuso prima di Run (`flow/reuse`) | sì (app) | equivalente diverso (chiavi in OPFS) | no | scelta (due cache) | — | 3 |
| Comando copiabile (`api/flow/command`) | CLI sì; app non verificato (`lr_flow_command` assente dall'rpc) | sì | `flow_command_tests.rs` (solo nativo) | da fare (verifica) | S | 3 |
| **Motore della pagina nell'app** | | | | | | |
| Grafo MSn dalla vista Dati (`api/msn_grafo`, op `msngrafo`) | CLI sì; pagina dell'app: op assente da `rpc*.rs` (non verificato) | sì | sito↔CLI sì (`test_msngrafo_wasm.py`); app no | da fare | S | 1 |
| Indice a centroidi (`lr_index_cen`), copia a centroidi (`mzml2cen`), SRM per mzFinder LR (`natsrm`) | pagina dell'app: op assenti (non verificato) | sì (`rust-worker.js:316`, `:409`, `:427`) | no | da fare | M | 1 |
| Rotte comuni (TIC, XIC, spettri, MRM, mappe, DAD, statistica…) | sì | sì | parziale: `crates/mzlab-desktop/tests/dad.rs` (DAD: stesse stringhe del `.wasm`); nessun test generale rotta per rotta | da fare (test) | M | 1 |
| mzFinder HR (`feat/*`, `finder/*`, untargeted) | sì | sì | sì: `crates/mzlab-desktop/tests/feat.rs`, `feat_untargeted.rs` | — | — | — |
| **Lettura e conversione** | | | | | | |
| Lettura diretta senza mzML intermedio | sì (8 famiglie) | parziale: motore (3200, Agilent GC/MS, mzML) e Thermo; 6 famiglie via mzML in OPFS | — | da fare (`formati-matrice.md`, punto 6) | L | 2 |
| `.wiff` → mzML con DAD (`opz.dad`) | sì | no (DAD solo come mappa/spettro) | no | da fare; tetto del modulo da misurare | M | 3 |
| Conversione a mzML (stesso `mzmlwrite`) | sì (coda, `core − 1` fili) | sì (su richiesta, in sequenza) | **no**: nessun test confronta il `to_mzml` dei moduli `.wasm` con `mzlab-cli convert` (`test_lettori_wasm.py` controlla solo i tetti) | da fare (test) | M | 1 |
| mzML con numpress in ingresso | non verificato nell'app (il core lo rifiuta) | sì (modulo `aperti`) | no | da fare (prova) | S | 2 |
| `mzlab-cli validate` (mzML: struttura, PSI-MS, indice, SHA-1) | sì | no | — | da fare | M | 3 |
| `mzlab-cli canali` | sì | sì (`api/canali`) | non verificato | da fare (test) | S | 3 |
| **Risorse** | | | | | | |
| File oltre 4 GiB di lavoro | sì (mmap, nessun tetto) | no: tetto 4 GiB per istanza, budget ≤ 3 072 MB e spill (`memoria.md:3`) | — | limite (memory64 assente in Safari, `wasm-potenziale.md:77`) | — | — |
| Thread dentro un file | sì (`rayon`) | no: pool di worker a messaggi (`wasm-potenziale.md:14`) | stessi byte provati dai test sopra | limite: niente COOP/COEP su GitHub Pages e niente `atomics` in Rust stabile (`wasm-potenziale.md:75`) | — | — |
| Cartella dei dati ricordata | sì (`data_choose_dir`, `cartella.rs`) | parziale: Chromium (`showDirectoryPicker`); Firefox/Safari una volta sola (`dati-cartella.md`) | tassonomia comune (`kindOf` = `cartella.rs`), non verificato da un test | limite del browser | — | — |
| **Librerie, assistente, tutor** | | | | | | |
| Scaricare librerie dal catalogo | sì (`lib_download`, `lib_download_external`) | parziale: link e «Load file…» (`librerie.md:30`) | — | limite: CSP `connect-src 'self'` e CORS delle fonti | — | — |
| Ricerca in libreria, FDR | sì | sì (`mzlab-lib-wasm`) | sì: `test_librerie_fdr_wasm.py` | — | — | — |
| Assistente con fornitori nel cloud | sì (app) | parziale: solo modello locale e `replay` (`llm.md:36`) | `test_llm_csp.py` | scelta (CSP) | — | — |
| `llm tools` / `replay` / `context` / `eval`, `tutor eval`, `msn-rules`, `explain-missing` | sì (CLI) | parziale: `context_build` e `strumenti/1` sì; gli altri no | `llm_replay_tests.rs` ecc. (solo nativo) | scelta: strumenti di sviluppo | — | 3 |
| Tutor della Teoria (`mzlab-tutor`) | solo libreria e CLI | no (nessun `.wasm`; logica pensata per un Worker) | — | da fare / decisione | L | 3 |
| Libro (`mzlab-libro`) | sì | sì | sì: `test_libro_wasm.py` | — | — | — |
| **Solo app per natura** | | | | | | |
| Aggiornamento della parte web, avviso di versione, app predefinita, avvio freddo | sì | n/a (il sito si aggiorna da sé) | — | scelta | — | — |
| Scheda per telefono (`telefono.js`) | n/a | sì | — | scelta | — | — |

## Conti
- **Passi del Flow** (48 nel registro): 44 eseguibili da `flow run`, 16 con un esecutore nel sito. **28 passi solo app/CLI**: 4 di catena e import, 11 di tabella ed esportazione, 8 di identificazione, 5 di struttura, MSn e quantificazione. Circa metà (colonna «pannello») ha già la funzione Rust nel `.wasm` di un pannello: manca l'esecutore del Flow e la prova. 3 passi non girano da nessuna parte (`features.ms2_link`, `annot.regulatory`, `annot.hazard`); `structure.draw` gira solo nel sito.
- **Motore della pagina**: 13 operazioni del worker del sito senza ramo nell'app; 5 contano (`msngrafo`, `lr_index_cen`, `mzml2cen`, `natsrm`, `lr_flow_command`), le altre sono del solo `.wasm` (`budget`, `down`, `lr_flow_sched` sostituito da `flow/live/*`, …).
- **Solo sito**: `structure.draw`, scheda telefono e, nell'app non verificati, i 5 rami sopra e il numpress in ingresso.
- **Impossibili oggi (dichiarati)**: thread dentro un file, più di 4 GiB, download diretto delle librerie, fornitori LLM nel cloud dal sito (scelta), aggiornamento dell'app.
- **Incoerenza da sistemare**: il descrittore `runs_in.wasm` nomina un modulo per passi che il sito non esegue (es. `table.filter` → `table`): «dove gira» va letto da un solo registro.

## I 10 passi più utili
1. **Parità della conversione** — test che fa scrivere l'mzML a ogni modulo `.wasm` dei lettori (Node, file sintetici di `tests/comune/`) e a `mzlab-cli convert` e confronta lo SHA-256. Chat: *Lettori/formati*.
2. **Rami mancanti nell'app** — aprire nell'app un'infusione MSn, un mzML in profilo e un campione SRM del produttore per mzFinder LR; se `rpc.unknown`, aggiungere `msngrafo`, `lr_index_cen`, `mzml2cen`, `natsrm` a `rpc*.rs` con le stesse funzioni del core, e prova in `crates/mzlab-desktop/tests/`. Chat: *Desktop*.
3. **Parità delle rotte sito ↔ app** — un test che manda le stesse richieste al motore `.wasm` (Node) e a `rpc` nativo su un file d'esempio e confronta le risposte rotta per rotta (sul modello di `tests/dad.rs`). Chat: *Desktop*.
4. **`prov/1` nel Flow del sito** — operazione `finish` nello scheduler del motore (`flow_sched`) che dà lo stesso `prov/1` di `flow/live finish`, scritto in OPFS, e il rapporto `corsa/1` aperto senza file. Chat: *Flow web*.
5. **Passi di tabella nel sito** — esecutori in `F.liveJobs` per `table.filter`, `table.drift`, `table.plausible`, `export.table_csv`, `export.inclusion`, `export.mztab` sopra il `.wasm` `table`, con e2e uguale a `flow run`. Chat: *Flow web (Tabui)*.
6. **Centroidi di un mzML in profilo nel Flow** — l'esecutore `centroid` usa `mzml2cen` invece di bloccare (`flow-live-jobs.js:35`), stessa uscita di `spectra.centroid` nativo. Chat: *Flow web*.
7. **Passi di Attributio nel sito** — `formula.ms1/ms2`, `spectra.chimeric`, `spectra.insilico`, `spectra.ms2`, `rt.predict`, `annot.level` e gli altri algoritmi di `spectra.match` come esecutori sopra `attributio-all`/`untargeted`/`lib`, con prova byte per byte. Chat: *Attributio*.
8. **Controllo del piano e liste in Rust** — sostituire `flow-check.js` con `flow::check` nel motore (come `flow_line`) e servire `flow/liste` dal `.wasm`; `test_flow_grafico.py` resta come prova. Chat: *Flow*.
9. **Lettura diretta nel sito** — Waters, Agilent LC/MS, Bruker, Shimadzu, ANDI, mzXML aperti per il motore come Thermo (`lettore-esterno.js`), senza mzML in OPFS; prima misurare tetti e tempi. Chat: *Lettori/formati*.
10. **Il resto dei passi e un solo «dove gira»** — `table.stats/pca/kmd`, `structure.*`, `msn.figure`, `chrom.uv_peaks`, `quant.calibrate`, `meta.compare`, `table.match`, `spectra.lookup`, `features.estimate`, `import.raw`/`convert.mzml` nel sito; `runs_in` del descrittore allineato agli esecutori veri, con un test che li confronta. Chat: *Flow web* (passi) e *Coordinatore* (decisione su `annot.regulatory/hazard`, tutor nel sito).
