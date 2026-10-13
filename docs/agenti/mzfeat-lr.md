# mzlab-feat a bassa risoluzione (full scan, risoluzione unitaria)

La pipeline di `crates/mzlab-feat` gira anche sui dati LR (SCIEX 3200 QTRAP, Q1 full scan, MS2 EPI) con una ricetta propria; i test usano solo dati sintetici (`tests/lr_fullscan.rs`).

## Scelta del profilo
- `Recipe::for_index(&mut Index)` (`recipe_lr.rs`): se `Index::hr1()` è vero la ricetta è quella predefinita (HR, invariata), altrimenti `Recipe::lr(difetto, LR_FLOOR, LR_MIN_HEIGHT)`. Il crate wasm la sceglie da solo (`run::recipe_for`) in rilevamento, anteprima, allineamento e ipotesi; `build_evidence*` usa la ricetta passata.
- Asse di massa: `MassAxis::{Ppm, Da{defect}}` (`axis.rs`). Una tolleranza negativa vale «Da»: `half(mz, tol)` è `-tol` se `tol < 0`, altrimenti `mz·tol·1e-6`. Costanti: `LR_TOL` 0,35 Da, `LR_TOUCH` 0,1, `LR_TRACE` 0,3, `LR_STITCH` 0,2.
- In Da i canali sono larghi 1 Da, con coordinata `mz − difetto + 0,5` e due griglie sfalsate di 0,5. Il difetto di massa è la moda circolare delle parti frazionarie dei picchi forti (`estimate_defect`); l'anteprima lo riporta in `"defect"` (solo in Da).

## Cosa cambia rispetto a HR
- Isotopi: `+1` e un solo `+2`, carica 1; `M+1/M` non è definito (`NaN`) nelle sintesi di riga (`rowmeta::compute_in` con finestra in Da).
- MS2: senza `parent` lo scan MS2 si lega alla MS1 precedente per RT; purezza con la tolleranza in Da.
- Famiglie: a massa nominale un addotto o una frammentazione in sorgente senza MS2 né sul leggero né sul pesante resta `Undetermined` (`Rules::nominal()`).
- Allineamento, riempimento dei buchi, mappa RT e qualità usano `half`; `fourier_artefacts` non si applica in Da.
- Ipotesi (`HypoParams.nominal`): si fondono per massa nominale, addotto e formula; le alternative compaiono come `formula (etichetta)`.
- Mirato (`TargetParams::lr`): finestre di 0,35 Da; `TargetParams.mz_offset` è l'errore di calibrazione misurato sul progenitore (`target::offset::estimate`, mediana osservato − calcolato, limite ±0,6 Da, finestra RT 0,3 min) e spostato su tutte le ipotesi.

## Livelli
- Profilo `lr` di `livelli/1` (nascosto): mai sopra il 3 (`tetto`: tutti i cancelli superati danno 3), 4 se la sola formula nominale spiega lo ione, altrimenti 5. Il profilo non ha `punti`: la tabella `PointsTable::bundled()` legge solo il profilo predefinito.

## Prova
- `cargo test -p mzlab-feat --test lr_fullscan`: serie sintetica (5 tempi + bianco, 270 scansioni Q1, rumore deterministico, spostamento di calibrazione +0,30 Da) costruita in Rust con `MzmlWriter` → `upload::prepare` → `mzidx`.
- Prove L8: `PointsTable::with_unit()` (scelta in `run::evidence_to` dal testo della ricetta nel meta della tabella, `recipe::is_unit_text`) mette le finestre di m/z in Da (`PointsTable::window`, `match_hypotheses_in`, `RowIdent::compute_in`, `parent_row`, `counts_of_n`), toglie la prova di massa (una massa nominale non è un'evidenza) e porta il livello massimo con MS2 a 3 (`MAX_LEVEL_UNIT`). Restano in ppm le viste che non passano da qui: `tp/` (vista dei prodotti), `evid/arcs.rs` e `evid/producers.rs` (non usati nel flusso del sito).
- Esportazione: `export_ms::mgf_unit` (formato `mgf_unit` di `ms2_file`) scrive lo stesso MGF con la riga `# unit resolution, tolerance 0.5 Da`; si sceglie in modo esplicito e non esiste una versione SIRIUS.
- Dimensione dei `.wasm` (grezzi, prima di `wasm-opt`): detect +14 KB e proofs +24 KB (circa +1-2 %) rispetto a prima del profilo unitario, dentro i tetti di `tools/build_feat_wasm.sh`.

## MRM (`crates/mzlab-feat/src/mrm.rs`)
- Un file MRM non ha scansioni: l'unità è la transizione. `read_srm` legge dall'mzML (non dal `.mzidx`) i cromatogrammi `SRM` con Q1 e Q3; `detect` cerca i picchi di ognuno con `peakq::detect_prominence` (lo stesso resolver delle tracce, soglia dal rumore del cromatogramma e `MrmParams.min_height`). Niente isotopi, MS2 né famiglie. `mz` della riga è il Q1 e `q3` il Q3: colonna opzionale di `FileFeatures` e di `Aligned` (assente nei file di ioni, quindi i byte HR non cambiano), conservata in `.mzfeatf` e `.mzfeat` solo se non vuota.
- `align` raggruppa per transizione (Q1, Q3 al centesimo) e per RT (`MrmParams.rt_gap`, un solo picco per file in ogni riga); le righe stanno in ordine (Q1, Q3, RT). `fill_file` integra le celle senza picco sul cromatogramma della transizione (soglia 3 volte il rumore); `file_bytes` e `table_bytes` scrivono `.mzfeatf` e `.mzfeat` con la ricetta unitaria (`recipe`).
- Ponte wasm: `detect_mrm_file` e `align_mrm_series` (leggono l'mzML con `read(k, offset, length)`) nella feature `mrm` di `mzlab-feat-wasm`, un terzo `.wasm` (`wasm-feat/mrm/`, voce `mrm` del manifest) caricato solo per una serie MRM: il lettore dei cromatogrammi dell'mzML sta lì e non nel `detect`. Dopo `wasm-opt -Oz`: detect 419 KB, proofs 458 KB, mrm 244 KB (il tetto è 512 000 B per ogni `.wasm`, alzato da 460 800 per lasciare spazio a nuove funzioni: la dieta dei `.wasm` è un pacchetto a parte).
- `qualifier_ratios`: per ogni Q1 con più transizioni, il rapporto delle aree di ciascuna contro quella con l'area mediana maggiore, per file (misura, nessun verdetto). `csv_calibration`: tabella lunga (transizione, campione, tipo, concentrazione, unità, RT, area, altezza, stato) dei file `calibration` e `std`, pronta per la retta, che mzLab non calcola.
- Non fatto qui: il livello (tetto 3 con standard confrontato in RT, altrimenti 4) si applica nell'interfaccia; precursor ion e neutral loss non si leggono.
