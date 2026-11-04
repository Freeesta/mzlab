# Flow: dai passi di mzmine ai passi di mzLab

Tabella di lavoro del Flow grafico (`docs/agenti/flow-grafica.md`). Dice cosa esiste oggi, senza promettere: **step** = ha un descrittore `step/1` nella palette; **motore** = il calcolo esiste in Rust (`crates/mzlab-feat`, binario `untargeted`) ma non ha ancora un descrittore, quindi il Flow non lo mette in catena; **previsto** = nel descrittore è un algoritmo `planned` o una riga dello studio, senza esecutore; **nessuno** = non c'è. Nessuna riga inventa un algoritmo.

| Passo di mzmine | In mzLab | Stato |
|---|---|---|
| Raw data import | `import.raw` (file RAW, WIFF, D...) e `convert.mzml` | step |
| Mass detection: centroid, exact mass | `features.detect` legge i centroidi dell'indice; nessun rilevatore di massa separato | nessuno |
| Mass detection: local maxima | `untargeted/localmax.rs` (da `LocalMaxMassDetector` 4.10.90), fuori dalla pipeline | motore |
| Mass detection: factor of lowest signal | `features.detect`, algoritmo `adap_localmin`, parametro `noise_factor` (soglia per scansione, `untargeted/pipeline_stream.rs`); 0 = soglia assoluta `floor` | step |
| Mass detection: wavelet, auto, recursive threshold | nessuna implementazione | nessuno |
| Scan index per accesso a finestre | `index.mzidx` | step |
| Chromatogram building (ADAP / ion chromatogram builder) | `features.detect`, algoritmo `masstrace` (HR e LR) o `adap_localmin` (ADAP di `untargeted/adap.rs`, ponte `untargeted/flow.rs`: `floor`, `min_height`, `min_points` comuni, tolleranza `ppm`/`abs_da`) | step |
| Feature detection alternativa | `features.detect`: `centwave` (onde Ricker sulle tracce, `centwave.rs`) disponibile; `threshold` | `threshold` previsto |
| Parametri dai dati | `features.estimate` (rumore, tolleranza m/z, larghezza del picco, tempo morto) | step |
| Smoothing | Savitzky-Golay in `features.detect` `adap_localmin` (`smooth_window`, 0 = spento; solo la forma su cui cerca i minimi, altezza e area dal grezzo); gaussiana in `untargeted/smooth.rs`; LOESS no | step (SG), motore (gauss), nessuno (LOESS) |
| Baseline | `untargeted/arpls.rs`, opzionale | motore |
| Feature resolving: local minimum | `features.detect` `adap_localmin` (`untargeted/resolve.rs`, da `MinimumSearchFeatureResolver`: `chrom_threshold`, `search_rt`, `min_ratio`, `duration`; il taglio di `resolve.rs` non si applica); con `masstrace` il taglio di `resolve.rs` | step |
| Feature resolving: ADAP wavelets, Savitzky-Golay, baseline resolver, noise amplitude | nessuna implementazione | nessuno |
| Deisotoping: isotope grouper | `isotopes.rs` dentro `features.detect` (gruppo isotopico, mai scarta); `untargeted/isotopes.rs` | motore |
| Deisotoping: isotope finder 4.10 | `untargeted/isofinder.rs` | motore |
| Alignment: join aligner | `features.align`, algoritmo `join` (`untargeted/join.rs`, ponte `untargeted/flow_align.rs`: `mz_weight`, `rt_weight`, `rt_tol`, tolleranza `ppm`/`abs_da`; righe per ione qualunque `mode`, solo in memoria, quindi una serie che non ci sta è rifiutata); `groups` di default (modalità «per ione» e «per famiglia») | step, motore |
| Alignment: RANSAC, correzione dell'RT tra campioni | `features.align`, parametro `rt_warp` (`piecewise`, `loess`, `ransac`; `crates/mzlab-feat/src/rtwarp.rs`): ancore da un allineamento grossolano (`rt_coarse_gap`), curva per file, RT riportato a quello comune prima del raggruppamento; `off` di default | step |
| Gap filling (stesso RT e m/z) | `features.gapfill`, algoritmo `integrate` di default; algoritmo `peak_finder` (`untargeted/gapfill.rs`, ponte `untargeted/flow_align.rs`: `intensity_tol`, tolleranza `ppm`/`abs_da`, `rt_tol`, `min_points`, `floor` comuni; `max_gap`, `long_trace` e la mappa RT non si applicano) | step, motore |
| Duplicate feature filter | `table.filter`, parametri `dup_ppm` e `dup_rt` (regola `duplicati`, `filters.rs`): tiene la riga più forte entro la finestra; l'allineamento scarta già i lobi FT | step |
| Feature list rows filter | `table.filter` (campioni minimi, area, altezza, rapporto al bianco, MS2 richiesta) | step |
| Blank subtraction | `table.filter`, parametro `blank_ratio`; `bianco.js` nella vista Dati HR | step |
| Normalization | `table.filter`, parametro `normalize` | step |
| Ion mobility | nessuna elaborazione (il lettore Bruker legge i dati) | nessuno |
| Adduct, complex, fragment search | famiglie di ioni in `features.align` («per famiglia»), `ionfamily` | motore |
| Correlation grouping (metaCorrelate), ion identity networking | nessun passo; i legami di famiglia sono in `features.align` | nessuno |
| MS2 scan pairing | `features.ms2_link` | step |
| MS2 spectral library search | `spectra.match` (entropia spettrale, coseno, coseno modificato) | step |
| Local compound database search | `annot.regulatory` copre solo liste e screening di sospetti, non una banca locale qualsiasi | parziale (step) |
| Lipid annotation | nessuno | nessuno |
| Formula prediction | `formula.ms1`, `formula.ms2` | step |
| SIRIUS / CSI:FingerID | nessuno; non si chiama alcun servizio esterno | nessuno |
| In silico fragmentation | `spectra.insilico` | step |
| Identification level | `annot.level` (cascata dei livelli di Schymanski) | step |
| Export: CSV | `export.table_csv` | step |
| Export: GNPS FBMN/IIN, SIRIUS, mzTab-M | solo le esportazioni dell'interfaccia (`attributio-export.md`), nessun passo | nessuno |
