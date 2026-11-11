# Lettore Bruker timsTOF (`.d` con `analysis.tdf` o `analysis.tsf`)

`crates/mzlab-lettori/src/bruker/` (feature `bruker`, modulo `.wasm` `bruker`, tetto 524 288 B; misurato dopo `wasm_opt -Oz`: 416 514 B con `ruzstd`). Voci del registro: `bruker-tdf` (membri `x.d/analysis.tdf` + `analysis.tdf_bin`) e `bruker-tsf` (`analysis.tsf` + `analysis.tsf_bin`); un eventuale `-wal` accanto al database si legge (vince l'ultimo frame confermato).
- Le cartelle Bruker senza `analysis.tdf`/`.tsf` che il modulo non legge si riconoscono (`Forse`, `legacy_key` in `tdf_helpers.rs`) per dire quale sono: solo `analysis.baf` → `err.file.brukerBaf`, `analysis.yep` (trappole ioniche) → `err.file.brukerYep`, `ser` o `fid` direttamente nella cartella `.d` (FT-ICR) → `err.file.brukerFtms`, un `fid` accanto ad `acqu`/`acqus` fuori da una `.d` (MALDI FLEX) → `err.file.brukerFlex`; tutte dicono di esportare in mzML.
- La pagina dà le stesse chiavi (`BrukerD.legacyKey` in `agilent-d.js`; FLEX tra i file ignorati, `CartCarica.flexMaldi`). BAF, YEP, FT-ICR e FLEX restano illeggibili senza l'SDK.
- Un database senza il suo `_bin`, un `_bin` senza il suo database o un database che non è SQLite (copia vuota o tagliata) sono `Forse`: `apri` dà `err.file.brukerNoBin` (`name` = cartella, `file` = il file che manca) o l'errore SQLite con chiave; la pagina fa lo stesso conto (`BrukerD.missing` in `agilent-d.js`).
- Un frame dell'indice che nomina una riga di `MzCalibration` assente o inutilizzabile (`MzCal::validate`) fa rifiutare la corsa all'apertura con `err.file.brukerCalibration` (`frame` = `Frames.Id`), non al primo spettro.
- Un `.tdf`/`.tsf` o un `_bin` danneggiato, tagliato o con un layout non letto (pagina SQLite, blocco zstd) è `err.file.brukerDamaged`: il testo dell'avviso non mostra il dettaglio del lettore (`detail`, scritto dal CLI dopo la chiave).

## File del modulo
- `sqlite_min.rs`, `sqlite_schema.rs`, `sqlite_wal.rs`: lettore SQLite 3 in sola lettura senza motore SQL (nessun `rusqlite`). Pagine b-tree di tabella (5, 13) e di indice (2, 10, per le tabelle `WITHOUT ROWID`, con le colonne riordinate: la chiave primaria per prima), overflow, log WAL. Visita iterativa con ogni puntatore, lunghezza, profondità e numero di pagine passati da `Limiti`; una pagina già vista o un ciclo è un errore, non un giro infinito.
- Overflow SQLite: una catena che torna su una pagina, o una cella che dichiara più byte di quanti ne tengano le pagine del file, è `err.file.brukerDamaged` (`tests/robustness_sqlite_overflow.rs`), mai lo stesso valore ricopiato.
- `block.rs`: blocchi di `tdf_bin`. `zstd()` usa `ruzstd` solo dopo aver letto la finestra e la dimensione annunciate dall'intestazione del frame zstd (il tetto è `Limiti.byte`) e pretende la dimensione attesa esatta. Un blocco inizia con `u32` lunghezza, `u32` numero di scansioni (`ns`), poi i dati.
- `calib.rs`: calibrazione m/z. `tables.rs`: tabelle del database. `spectra.rs`: taglio degli spettri (`tratti`: gli intervalli di scansione uniti e passati agli offset per scansione `Picchi::inizio`; `somma`: unione per indice TOF, radix a 11 bit da 1024 punti). Banco: `examples/bruker_bench.rs` (`misure-formati.md`). `tsf.rs`: blocchi TSF. `header.rs`: `metodo/1` e testata mzML. `tdf.rs`: `ENTRY`, `BrukerReader`.

## Tabelle lette
`Frames`, `Precursors`, `PasefFrameMsMsInfo`, `DiaFrameMsMsWindows`, `DiaFrameMsMsInfo`, `PrmFrameMsMsInfo`, `PrmTargets`, `FrameMsMsInfo`, `MzCalibration`, `TimsCalibration`, `GlobalMetadata`, `Properties`, `PropertyDefinitions`, `GroupProperties`. Una tabella che manca non ha righe (non è un errore). Ogni chiave di `GlobalMetadata` finisce in `campi` (origine `bruker:global`); operatore, seriali, percorsi, cartelle, computer e utente sono marcati `identita`.

## Blocchi (`Frames.TimsId` = offset in `tdf_bin`)
- Compressione 2 (`TimsCompressionType`, zstd): dentro lo zstd, `ns + 2·np` parole `u32` scritte in 4 piani di byte (tutti i byte 0, poi i byte 1, ...). Le prime `ns` sono l'intestazione (la scansione `s` ha `parola[s+1]/2` picchi, l'ultima il resto di `NumPeaks`); poi coppie (delta di indice TOF, intensità). L'accumulatore parte da `u32::MAX`: il primo indice TOF di una scansione è delta−1.
- Compressione 1: `ns+1` offset `u32`, poi per ogni scansione un flusso LZF di parole `i32`: una parola negativa salta |v| indici TOF, una non negativa è un'intensità all'indice corrente, che poi avanza di 1. Il numero di scansioni è quello scritto nel blocco (la rampa intera, es. 985), non quello della tabella `Frames`.
- `*_bin` tagliato (copia interrotta): all'apertura `tagliati` (`tdf_helpers.rs`) segna i frame il cui blocco non finisce dentro il file (un blocco finisce dove inizia il successivo; si legge solo l'intestazione dell'ultimo che inizia nel file); i loro spettri escono dall'indice, `Lettore::missing` li conta e il resto si legge (TIC, BPC, mzML, `.mzidx`). `Reader::losses` dà dichiarati, letti e la causa `warn.file.brukerCutBin` (`tdf::CUT_BIN`), scritti in `conv/1` dell'mzML dal modulo e dal CLI (che esce con 3).
- TSF: `u32` dimensione del profilo zstd, `u32` dimensione delle linee zstd; le linee sono `NumPeaks` posizioni TOF `f64` poi `NumPeaks` intensità `f32`; il profilo (se non ci sono linee) ha `DigitizerNumSamples` campioni `u32`, tenuti solo se diversi da zero.

## m/z (`MzCalibration`, `ModelType` 1)
`t = DigitizerDelay + DigitizerTimebase·tof`. Con `k = 1e6/√C1`, `u = t − C0 + C2·C4`, `s = 2u / (k + √(k² + 4·C2·u))`, `m/z = s² − C4`. Nessuno spostamento di centratura. Correzione di temperatura per frame: `m/z · (1 − 1e-6·(dC1·(T1frame − T1cal) + dC2·(T2frame − T2cal)))`. Costanti non finite o `C1 ≤ 0` danno un errore con chiave, non numeri.

## Intensità
Sono conteggi grezzi (quelli del blocco). `SummedIntensities` e `MaxIntensity` della tabella `Frames` sono normalizzati a 100 ms: il TIC dell'indice è `SummedIntensities·AccumulationTime/100`. Indici TOF uguali nello stesso spettro si sommano (msconvert li lascia come punti separati: stesse somme).
- Cromatogrammi: TIC (MS1 da `Frames`, MS2 sommato dai punti alla prima chiamata di `indice`; probe, mzML, `conv/1` e prova SRM chiedono `indice_senza_tic`, senza decodificare frame: `crates/mzlab-lettori/tests/bruker_without_tic.rs`) e BPC, il massimo delle intensità a 32 bit di ogni spettro di rilievo; il lettore tiene il massimo di ogni spettro già decodificato (`massimi`, `traces.rs`), così il BPC dopo un giro intero (indice `.mzidx`, mzML) non decodifica di nuovo nessun frame. Prova: `bpc_equal_before_and_after_the_round_trip` (`tests/robustness_bruker.rs`).

## Spettri
- `MsMsType` 0 (MS1): un frame è uno spettro, mobilità sommata.
- 8 (DDA-PASEF): uno spettro per (frame, precursore), righe della stessa coppia unite, scansioni sommate, con isolamento (`IsolationMz`, `IsolationWidth`) e CE; ordine: prima scansione della finestra (come msconvert). Nel `<precursor>` dell'mzML lo ione selezionato è `MonoisotopicMz` (o `LargestPeakMz`), il centro della finestra `IsolationMz` e la carica `Charge` di `Precursors` (una carica 0 o vuota non si scrive); prova `tests/bruker_precursor.rs`.
- 9 (diaPASEF): uno spettro per finestra del gruppo (`DiaFrameMsMsInfo` → `DiaFrameMsMsWindows`) e per ciclo.
- 10 (PRM-PASEF): uno spettro per riga di `PrmFrameMsMsInfo`.
- Altri tipi: uno spettro per frame, livello 2 se esiste `FrameMsMsInfo`.
- Gli intervalli di scansione sono semiaperti [inizio, fine), da 0. Le stringhe del filtro non contengono «SRM», «MRM» né «CRM» (per non essere prese per tracce di un triplo quadrupolo).
- La mobilità per punto non entra nel motore. In `campi` di ogni spettro stanno l'intervallo di scansioni TIMS e, per TDF, l'avviso `meta.nonDecodificato` (vedi sotto).

## Metodo e testata
`metodo/1` da `GlobalMetadata`, `Properties` e dalle calibrazioni (`MzCalibration`, `TimsCalibration`); `testata_mzml` con `Header::run_params` = `("mzLab metodo/1", documento)`, strumento, seriale, software, `startTimeStamp` e SHA-1 di `analysis.tdf` (solo TDF); componenti quadrupolo + time-of-flight + rivelatore a microcanali (il file non li nomina), da cui la tabella delle scansioni legge l'alta risoluzione (`hr1`, prova `a_timstof_is_high_resolution`).

## Prove
- `cargo test -p mzlab-lettori --features bruker`: unitarie (varint, colonne, calibrazione, TSF) e `tests/robustness_bruker.rs` (SQLite e `tdf_bin` costruiti nel test: WAL confermato, non confermato e con somma errata, `WITHOUT ROWID`, ciclo di pagine, varint lunghi, zstd falso e con finestra enorme, conteggi di picchi bugiardi, troncamenti a ogni passo, `tdf_bin` tagliato dentro un frame, BAF, cartella Agilent); `tests/test_lettori_parita.py` (`test_bruker_con_tdf_bin_tagliato_uguale_al_cli`): stesso mzML dal modulo e dal CLI per una cartella tagliata.
- `tests/bruker_data.rs` (dati veri con `MZLAB_DATI`, si salta senza): confronto con l'mzML di msconvert (`combineIonMobilitySpectra`) su una corsa DDA-PASEF e una MRM con compressione 1: differenza massima 0,07 ppm sull'm/z, stesse somme di intensità; una cartella diaPASEF pulita; le cartelle sintetiche di timsrust.
- Pagina: `tests/test_bruker_d.py` (node): una cartella `.d` con `analysis.tdf` va alla famiglia `bruker`, una `.D` con `AcqData` all'`agilent`; in `lettori-famiglie.js` la riga `bruker` sta prima di `agilent` e decide dai membri (`analysis.tdf`, `analysis.tsf`, `analysis.baf`), mai dall'estensione. `agilent-d.js` (`BrukerD`) manda la cartella al modulo con `LETTORI_MOD.stand`.

## Non verificato
- La compressione 2 non ha un mzML di riferimento: è controllata solo contro `Precursors.LargestPeakMz` (pochi ppm) e leggendo cartelle vere (DDA, MRM, diaPASEF con WAL).
- TSF: m/z uguale a quello del `.mgf` di Bruker su un campione; mobilità non verificata.
- Mobilità inversa 1/K0 (`TimsCalibration`, `ModelType` 2): la formula non è nota né ricavabile dai file, quindi non si calcola; si scrivono gli intervalli di scansione (DEV-10 userà i punti con mobilità).
- Il BAF non si legge (serve l'SDK Bruker).
- TIC dell'indice: il fattore `AccumulationTime/100` presuppone `SummedIntensities` normalizzato a 100 ms in ogni versione dello schema; non è verificato su tutte le versioni (nessun file sintetico né documento lo decide), quindi la regola resta questa.
