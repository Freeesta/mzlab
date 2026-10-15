# Lettore Bruker timsTOF (`.d` con `analysis.tdf` o `analysis.tsf`)

`crates/mzlab-lettori/src/bruker/` (feature `bruker`, modulo `.wasm` `bruker`, tetto 524 288 B; misurato dopo `wasm_opt -Oz`: 416 514 B con `ruzstd`). Voci del registro: `bruker-tdf` (membri `x.d/analysis.tdf` + `analysis.tdf_bin`) e `bruker-tsf` (`analysis.tsf` + `analysis.tsf_bin`); un eventuale `-wal` accanto al database si legge (vince l'ultimo frame confermato). Una cartella con solo `analysis.baf` si riconosce (`Forse`) per dire `err.file.brukerBaf` («esporta in mzML con Bruker DataAnalysis o msconvert»): il BAF resta impossibile senza l'SDK.

## File del modulo
- `sqlite_min.rs`, `sqlite_schema.rs`, `sqlite_wal.rs`: lettore SQLite 3 in sola lettura senza motore SQL (nessun `rusqlite`). Pagine b-tree di tabella (5, 13) e di indice (2, 10, per le tabelle `WITHOUT ROWID`, con le colonne riordinate: la chiave primaria per prima), overflow, log WAL. Visita iterativa con ogni puntatore, lunghezza, profondità e numero di pagine passati da `Limiti`; una pagina già vista o un ciclo è un errore, non un giro infinito.
- `blocco.rs`: blocchi di `tdf_bin`. `zstd()` usa `ruzstd` solo dopo aver letto la finestra e la dimensione annunciate dall'intestazione del frame zstd (il tetto è `Limiti.byte`) e pretende la dimensione attesa esatta. Un blocco inizia con `u32` lunghezza, `u32` numero di scansioni (`ns`), poi i dati.
- `calib.rs`: calibrazione m/z. `tabelle.rs`: tabelle del database. `spettri.rs`: taglio degli spettri. `tsf.rs`: blocchi TSF. `testata.rs`: `metodo/1` e testata mzML. `tdf.rs`: `VOCE`, `LettoreBruker`.

## Tabelle lette
`Frames`, `Precursors`, `PasefFrameMsMsInfo`, `DiaFrameMsMsWindows`, `DiaFrameMsMsInfo`, `PrmFrameMsMsInfo`, `PrmTargets`, `FrameMsMsInfo`, `MzCalibration`, `TimsCalibration`, `GlobalMetadata`, `Properties`, `PropertyDefinitions`, `GroupProperties`. Una tabella che manca non ha righe (non è un errore). Ogni chiave di `GlobalMetadata` finisce in `campi` (origine `bruker:global`); operatore, seriali, percorsi, cartelle, computer e utente sono marcati `identita`.

## Blocchi (`Frames.TimsId` = offset in `tdf_bin`)
- Compressione 2 (`TimsCompressionType`, zstd): dentro lo zstd, `ns + 2·np` parole `u32` scritte in 4 piani di byte (tutti i byte 0, poi i byte 1, ...). Le prime `ns` sono l'intestazione (la scansione `s` ha `parola[s+1]/2` picchi, l'ultima il resto di `NumPeaks`); poi coppie (delta di indice TOF, intensità). L'accumulatore parte da `u32::MAX`: il primo indice TOF di una scansione è delta−1.
- Compressione 1: `ns+1` offset `u32`, poi per ogni scansione un flusso LZF di parole `i32`: una parola negativa salta |v| indici TOF, una non negativa è un'intensità all'indice corrente, che poi avanza di 1. Il numero di scansioni è quello scritto nel blocco (la rampa intera, es. 985), non quello della tabella `Frames`.
- TSF: `u32` dimensione del profilo zstd, `u32` dimensione delle linee zstd; le linee sono `NumPeaks` posizioni TOF `f64` poi `NumPeaks` intensità `f32`; il profilo (se non ci sono linee) ha `DigitizerNumSamples` campioni `u32`, tenuti solo se diversi da zero.

## m/z (`MzCalibration`, `ModelType` 1)
`t = DigitizerDelay + DigitizerTimebase·tof`. Con `k = 1e6/√C1`, `u = t − C0 + C2·C4`, `s = 2u / (k + √(k² + 4·C2·u))`, `m/z = s² − C4`. Nessuno spostamento di centratura. Correzione di temperatura per frame: `m/z · (1 − 1e-6·(dC1·(T1frame − T1cal) + dC2·(T2frame − T2cal)))`. Costanti non finite o `C1 ≤ 0` danno un errore con chiave, non numeri.

## Intensità
Sono conteggi grezzi (quelli del blocco). `SummedIntensities` e `MaxIntensity` della tabella `Frames` sono normalizzati a 100 ms: il TIC dell'indice è `SummedIntensities·AccumulationTime/100`. Indici TOF uguali nello stesso spettro si sommano (msconvert li lascia come punti separati: stesse somme).

## Spettri
- `MsMsType` 0 (MS1): un frame è uno spettro, mobilità sommata.
- 8 (DDA-PASEF): uno spettro per (frame, precursore), righe della stessa coppia unite, scansioni sommate, con isolamento (`IsolationMz`, `IsolationWidth`) e CE; ordine: prima scansione della finestra (come msconvert).
- 9 (diaPASEF): uno spettro per finestra del gruppo (`DiaFrameMsMsInfo` → `DiaFrameMsMsWindows`) e per ciclo.
- 10 (PRM-PASEF): uno spettro per riga di `PrmFrameMsMsInfo`.
- Altri tipi: uno spettro per frame, livello 2 se esiste `FrameMsMsInfo`.
- Gli intervalli di scansione sono semiaperti [inizio, fine), da 0. Le stringhe del filtro non contengono «SRM», «MRM» né «CRM» (per non essere prese per tracce di un triplo quadrupolo).
- La mobilità per punto non entra nel motore. In `campi` di ogni spettro stanno l'intervallo di scansioni TIMS e, per TDF, l'avviso `meta.nonDecodificato` (vedi sotto).

## Metodo e testata
`metodo/1` da `GlobalMetadata`, `Properties` e dalle calibrazioni (`MzCalibration`, `TimsCalibration`); `testata_mzml` con `Header::run_params` = `("mzLab metodo/1", documento)`, strumento, seriale, software, `startTimeStamp` e SHA-1 di `analysis.tdf` (solo TDF).

## Prove
- `cargo test -p mzlab-lettori --features bruker`: unitarie (varint, colonne, calibrazione, TSF) e `tests/robustezza_bruker.rs` (SQLite e `tdf_bin` costruiti nel test: WAL confermato, non confermato e con somma errata, `WITHOUT ROWID`, ciclo di pagine, varint lunghi, zstd falso e con finestra enorme, conteggi di picchi bugiardi, troncamenti a ogni passo, BAF, cartella Agilent).
- `tests/bruker_dati.rs` (dati veri con `MZLAB_DATI`, si salta senza): confronto con l'mzML di msconvert (`combineIonMobilitySpectra`) su una corsa DDA-PASEF e una MRM con compressione 1: differenza massima 0,07 ppm sull'm/z, stesse somme di intensità; una cartella diaPASEF pulita; le cartelle sintetiche di timsrust.
- Pagina: `tests/test_bruker_d.py` (node): una cartella `.d` con `analysis.tdf` va alla famiglia `bruker`, una `.D` con `AcqData` all'`agilent`; in `lettori-famiglie.js` la riga `bruker` sta prima di `agilent` e decide dai membri (`analysis.tdf`, `analysis.tsf`, `analysis.baf`), mai dall'estensione. `agilent-d.js` (`BrukerD`) manda la cartella al modulo con `LETTORI_MOD.stand`.

## Non verificato
- La compressione 2 non ha un mzML di riferimento: è controllata solo contro `Precursors.LargestPeakMz` (pochi ppm) e leggendo cartelle vere (DDA, MRM, diaPASEF con WAL).
- TSF: m/z uguale a quello del `.mgf` di Bruker su un campione; mobilità non verificata.
- Mobilità inversa 1/K0 (`TimsCalibration`, `ModelType` 2): la formula non è nota né ricavabile dai file, quindi non si calcola; si scrivono gli intervalli di scansione (DEV-10 userà i punti con mobilità).
- Il BAF non si legge (serve l'SDK Bruker).
