# Flow: dai passi di mzmine ai passi di mzLab

Tabella di lavoro del Flow grafico (`docs/agenti/flow-grafica.md`). Dice cosa esiste oggi, senza promettere: **step** = ha un descrittore `step/1` nella palette; **motore** = il calcolo esiste in Rust (`crates/mzlab-feat`, binario `untargeted`) ma non ha ancora un descrittore, quindi il Flow non lo mette in catena; **previsto** = nel descrittore è un algoritmo `planned` o una riga dello studio, senza esecutore; **nessuno** = non c'è. Nessuna riga inventa un algoritmo.

| Passo di mzmine | In mzLab | Stato |
|---|---|---|
| Raw data import | `import.raw` (file RAW, WIFF, D...) e `convert.mzml` | step |
| Mass detection: centroid, exact mass | `features.detect` legge i centroidi dell'indice; nessun rilevatore di massa separato | nessuno |
| Mass detection: local maxima | `untargeted/localmax.rs` (da `LocalMaxMassDetector` 4.10.90), fuori dalla pipeline | motore |
| Mass detection: wavelet, auto, recursive threshold, factor of lowest signal | nessuna implementazione | nessuno |
| Scan index per accesso a finestre | `index.mzidx` | step |
| Chromatogram building (ADAP / ion chromatogram builder) | `features.detect`, algoritmo `masstrace` (HR e LR); ADAP in `untargeted/adap.rs` | step (`masstrace`), motore (ADAP) |
| Feature detection alternativa | `features.detect`: `centwave` e `threshold` | previsto |
| Parametri dai dati | `features.estimate` (rumore, tolleranza m/z, larghezza del picco, tempo morto) | step |
| Smoothing | `untargeted/smooth.rs` (gaussiana, Savitzky-Golay); LOESS no | motore (SG, gauss), nessuno (LOESS) |
| Baseline | `untargeted/arpls.rs`, opzionale | motore |
| Feature resolving: local minimum | taglio di `resolve.rs` dentro `features.detect`; `untargeted/resolve.rs` (da `MinimumSearchFeatureResolver`) | motore |
| Feature resolving: ADAP wavelets, Savitzky-Golay, baseline resolver, noise amplitude | nessuna implementazione | nessuno |
| Deisotoping: isotope grouper | `isotopes.rs` dentro `features.detect` (gruppo isotopico, mai scarta); `untargeted/isotopes.rs` | motore |
| Deisotoping: isotope finder 4.10 | `untargeted/isofinder.rs` | motore |
| Alignment: join aligner | `features.align`, modalità «per ione» e «per famiglia»; `untargeted/join.rs` | step, motore |
| Alignment: RANSAC | nessuna implementazione | nessuno |
| Gap filling (stesso RT e m/z) | `features.gapfill`; `untargeted/gapfill.rs` | step, motore |
| Duplicate feature filter | nessuno; l'allineamento scarta i lobi FT | nessuno |
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
