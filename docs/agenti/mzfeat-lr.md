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
- Non ancora in LR: le prove L8 nominali (massa nominale, M+2 degli alogeni, MS2 EPI) e l'esportazione MGF con nota di risoluzione unitaria.
