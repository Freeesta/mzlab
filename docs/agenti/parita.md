# Parità delle tre facce: funzione × faccia × prova
Regola (`CLAUDE.md`): ogni funzione esiste nel sito (`.wasm` + OPFS) e nell'app (desktop nativo o `mzlab-cli`), con una prova byte per byte. Qui lo stato di ogni classe di funzioni; lo studio con le priorità è `docs/studi/parita-browser.md`, i formati `docs/studi/formati-matrice.md`, i passi della CLI `cli-passi.md`.

## Guardie automatiche
- **Operazioni del motore della pagina**: `tests/test_parita_operazioni.py` legge ogni op di `mzlab/web/rust-worker.js` e pretende un ramo nell'app (`shim/desktop-worker.js` o un braccio `"op" =>` di `crates/mzlab-desktop/src/*.rs`); le op del solo sito stanno in `SOLO_SITO` con il motivo, e la lista non può nominare op che l'app serve o che il sito non ha.
- **Comando copiabile, rotte comuni, conversione, validazione, canali**: prove sotto, una per riga.
- **Determinismo**: `HashMap`/`HashSet` di std hanno un seme casuale per processo nel nativo e chiavi fisse in `wasm32-unknown-unknown`: un ordine di iterazione che arriva a un'uscita darebbe byte diversi fra due corse dell'app e fra app e sito. Nessuna mappa del motore lo fa (verifica su tutti i crate fuori da `mzlab-lib`: solo ricerche, conteggi, min/max; `serde_json` senza `preserve_order`, quindi `Value` è ordinato).
- **Fili e orari**: nessuna somma di f64 dipende dall'ordine dei fili (i lavori paralleli scrivono per indice), nessun orario entra in un'uscita salvo `ms`/`heap_peak_bytes` di `prov/1` (`cli.md`, «Riproducibilità»).

## Motore della pagina (op di `rust-worker.js`)
| Funzione | Sito | App | CLI | Prova |
|---|---|---|---|---|
| Rotte comuni (TIC, XIC, spettri, MRM, mappe, canali, info, statistica) | `.wasm` motore | `rpc.rs` | `channels`, `sim-table` | `crates/mzlab-desktop/tests/parita_rotte.rs` (37 passi, `.wasm` con `MZLAB_WASM_PKG` = app), `test_canali_wasm.py` |
| Grafo MSn (`msngrafo`), centroidi (`lr_index_cen`, `mzml2cen`), SRM per mzFinder (`natsrm`) | sì | `rpc_msn.rs` | `msn-graph` | `parita_msn.rs` (app = core), `test_msngrafo_wasm.py` (`.wasm` = CLI) |
| Registro dello strumento di un `.raw` (`natregistro`, `/api/registro`) | modulo `thermo` (`registroDi`) | `rpc_nat.rs` | — (nessun comando; il `metodo/1` della CLI elenca le serie) | `crates/mzlab-desktop/tests/natregistro.rs` (app = lettore del registro + `mzlab_lettori::json::cromatogramma_json`, la funzione che il modulo chiama); il `.raw` sintetico non ha status log: la prova passa per il TIC e per l'errore di una serie assente |
| Massa del fondo (`masscal`) | sì | `rpc.rs` | — | `parita_masscal.rs` |
| Comando copiabile, riga corta (`lr_flow_command`, `flow/line`) | `.wasm` `rotte` | `rpc_lr.rs`, `flow/line` | `flow run` | `parita_flow_command.rs` |
| Validazione mzML (`api/mzml_valida`) | modulo `valida` | `lr_valida` | `validate` | `test_valida_wasm.py`, `crates/mzlab-desktop/tests/rpc.rs` |
| Solo sito per natura | `module`, `policy`, `budget`, `down`, `natsame`, `lr_gunzip`, `lr_cen_have`, `lr_cen_adopt`; `lr_flow_line`/`lr_flow_sched` (l'app usa `flow/line`, `flow/live/*`) | — | — | `SOLO_SITO` |
| «Annulla» del caricamento (`lr_cancel`) | ferma copie e indici | manca: l'indice in costruzione finisce (i file sono mappati, niente copia); le uscite non cambiano | — | da fare, in `SOLO_SITO` come buco |

## Passi del Flow (`mzlab-cli steps`, esecutori del sito in `F.liveJobs`)
| Passi | Sito | App/CLI | Prova |
|---|---|---|---|
| `index.mzidx`, `features.detect/align/gapfill`, `spectra.centroid` | sì | sì | `e2e_flow_web.py`, `e2e_flow_centroid_web.py`, `test_feat_wasm.py` |
| `table.filter`, `table.drift`, `features.group`, `export.table_csv`, `export.mztab`, `export.inclusion`, `export.annotations`, `chrom.transitions`, `qc.run_order`, `report.methods` | sì | sì | `e2e_flow_web_export.py` (filtro ed esportazioni), `e2e_flow_web_drift.py`, `e2e_flow_web_group.py`, `e2e_flow_neg_lr.py`, `e2e_flow_export_annot.py`, `e2e_flow_transitions.py`, `e2e_flow_qcreport.py` |
| `spectra.ms2`, `spectra.match`, `table.match`, `formula.ms2`, `annot.level`, `qc.mass_accuracy`, `screen.suspects`, `annot.regulatory`, `spectra.insilico` | sì | sì | `e2e_flow_web_ident.py`, `e2e_flow_web_ident2.py`, `e2e_flow_qc_mass.py`, `e2e_flow_suspects.py`, `e2e_normative_screen.py`, `e2e_flow_insilico.py` |
| `tp.*` (7), `spectra.msn_graph`, `msn.library_export`, `msn.table_export` | sì | sì | `e2e_flow_tp_*.py`, `e2e_flow_dig_lr.py`, `e2e_flow_msn.py` |
| `import.raw`, `convert.mzml`, `features.estimate`, `features.table`, `table.stats`, `table.pca`, `table.kmd`, `table.plausible`, `spectra.chimeric`, `formula.ms1`, `rt.predict`, `structure.render`, `structure.reactivity`, `msn.figure`, `meta.compare`, `spectra.lookup`, `chrom.uv_peaks`, `quant.calibrate` | no come passo (molti nel `.wasm` di un pannello) | sì | da fare (area Flow, `parita-browser.md` punti 5, 7, 10) |
| `features.ms2_link`, `annot.hazard` | no | no (solo descrittore) | — |
| `structure.draw` | sì (editor) | no | limite: un disegno non è un calcolo |

## Lettura, conversione, esportazione
| Funzione | Sito | App/CLI | Prova |
|---|---|---|---|
| Conversione a mzML (8 famiglie) | moduli `wasm-lettori/*` (`to_mzml`) | `convert`, `nat2mzml` | `test_lettori_parita.py` (SHA-256 uguale, dati sintetici) |
| mzML con Numpress in ingresso | modulo `aperti` | percorso nativo | `numpress_mzml.rs` |
| Lettura diretta senza mzML (6 famiglie) | via mzML in OPFS | diretta | limite di oggi (`formati-matrice.md`, punto 6) |
| File oltre 4 GiB, fili dentro un file | no | sì | limite: memory64 e `atomics` (`wasm-potenziale.md`) |
| SRM: energia di collisione e dwell dell'id (`ce=`, `dwell=`) nell'mzML | ogni modulo lettori | `convert`, `nat2mzml` | `mzmlwrite/tests.rs` (`an_srm_id_with_energy_and_dwell_writes_them_in_every_build`): `srm_windows` senza `cfg(feature = "sciex")` |
| Prodotto ionico: energia e finestra di isolamento del precursore nell'`.mzidx` di un `.wiff` | motore (senza `sciex` né `precursori`) | `flow run` (`index.mzidx`) | `test_wiff_energia_parita.py` (byte per byte, `energia.wiff` sintetico); `mzmlwrite/tests.rs` (`a_plain_precursor_keeps_its_energy_and_window_in_every_build`) |
| Feature da un `.raw` e dal suo mzML | — | `features` | limite: stesso risultato solo per acquisizioni a centroidi; un `.raw` a profilo usa i centroidi del produttore, il suo mzML a profilo la centroidatura di mzLab (area di `features` e del lettore Thermo) |

## Verifiche su dati reali (fuori dal repository)
- `.mzidx` di 6 mzML, canali (7 ingressi), `sim-table` di un `.wiff` (7 viste, 21 campioni): sito e CLI uguali byte per byte; differenze trovate e chiuse: TIC del `.wiff` (f32/f64), energia SRM e del precursore, nome della sorgente dell'indice nativo.
