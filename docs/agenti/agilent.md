# Lettore Agilent MassHunter `.D` (GC/MS)

Stato attuale. `crates/mzlab-core/src/agilent/`: Rust puro, nessun crate nuovo. Una cartella `.D` si legge dai suoi file: `AcqData/MSScan.bin` (indice, un record per scansione) e `AcqData/MSPeak.bin` (i punti), più i file piccoli (`MSTS.xml`, `Devices.xml`, `Contents.xml`, `MSTree2.bin`, `acqmeth.txt` nella cartella `.D`).

Il formato è stato ricavato da tre corse vere di un GC/MS a singolo quadrupolo 5977 (EI, positivo; una in scansione 50-500 m/z, due in SIM) e dallo schema `AcqData/MSScan.xsd` che Agilent mette nella cartella; NON c'è un mzML di riferimento Agilent, i controlli sono interni (sotto). Tutto ciò che non è nelle tre corse è elencato in «Non verificato».

## File di una cartella `.D` e a cosa servono
- `AcqData/MSScan.bin`: indice (formato sotto). `AcqData/MSPeak.bin`: punti. Bastano questi due per leggere tutto.
- `AcqData/MSTS.xml`: segmenti di tempo (`TimeSegment`: `StartTime`, `EndTime` in minuti, `NumOfScans`). `NumOfScans` conta per TEMPO: la scansione a cavallo tra due segmenti SIM può avere nel record il segmento precedente e `MSTS.xml` il successivo (differenza di 1 tra i due conteggi; il totale coincide).
- `AcqData/MSIndex.bin` (solo nelle corse SIM): per ogni segmento una breve intestazione (con il numero di scansioni del segmento) e i tempi di scansione (f64, minuti); verificato il primo segmento: tempi uguali a quelli dei record. Non serve al lettore.
- `AcqData/MSTree2.bin`: `u32 86, u32 5`, tre testi (`u8` lunghezza + byte: dispositivo, tecnica, strumento), quattro f64 (primo e ultimo tempo, m/z minimo e massimo), `00 01 01 00 00 00`, testo del tipo di scansione («Scan», «SelectedIon»), `u32 1`, testo della polarità («Positive»), `u32` numero di scansioni, poi i tempi e una struttura che non si legge. `info::tree` legge solo questa testa e dà `None` se la forma è diversa.
- `AcqData/Devices.xml` (nome, modello, seriale, firmware), `Contents.xml` (data, tecnica di separazione 2 = GC, software), `sample_info.xml` e `sequence.xml` (campione, operatore, vial: non letti), `acqmeth.txt` (UTF-16, il metodo; `method::parse`), `*.M/` (metodo di analisi dati: non letto), `data.ms` (formato ChemStation vecchio, «GC / MS Data File»: i dati di partenza da cui `IndexedDataConverter.log` ha generato `AcqData`; non letto), `checksum.xml`, `GC.ini`, `runstart.txt`, `PRE_POST.INI`, `TEMPBASE`.

## `MSScan.bin`
- Intestazione di 160 byte, uguale nelle tre corse; il `i32` a +88 è l'offset del primo record (160). Poi `(dimensione − 160) / 184` record di 184 byte, senza riempimenti, little-endian. Il numero di record = numero di scansioni (3868 e 3417 nelle corse; `MSTS.xml` dà gli stessi totali).
- Record, un campo per riga (offset, campo, tipo, cosa si vede):

| Offset | Campo | Tipo | Nelle corse |
|---|---|---|---|
| 0 | `ScanID` | i32 | 1, 2, 3… |
| 4 | `ScanMethodID` | i32 | 1 |
| 8 | `TimeSegmentID` | i32 | da 0 |
| 12 | `ScanTime` | f64 | minuti |
| 20 | `MSLevel` | i32 | 1 |
| 24 | `ScanType` | i32 | 1 scansione, 2 SIM |
| 28 / 36 / 44 | `TIC`, `BasePeakMZ`, `BasePeakValue` | f64 | |
| 52 | `CycleNumber` | i32 | = ScanID |
| 56 | `Status` | i32 | 0 |
| 60 | `IonMode` | i32 | 2 (EI) |
| 64 | `IonPolarity` | i32 | 0 (tutte positive) |
| 68 / 76 / 84 | `Fragmentor`, `CollisionEnergy`, `MzOfInterest` | f64 | 0 |
| 92 | `SamplingPeriod` | f64 | 0 |
| 100 / 108 | `MeasuredMassRangeMin`, `Max` | f64 | 0 e 1050: limite dello strumento, non i ioni del metodo |
| 116 | `Threshold` | f64 | 0 |
| 124 / 128 | `IsFragmentorDynamic`, `IsCollisionEnergyDynamic` | i32 | 0 |
| 132 | `SpectrumFormatID` | i32 | 1 |
| 136 | `SpectrumOffset` | i64 | dentro `MSPeak.bin` |
| 144 / 148 | `ByteCount`, `PointCount` | i32 | |
| 152 / 160 | `MinY`, `MaxY` | f64 | MinY è sempre il massimo f64 (valore iniziale) |
| 168 / 176 | `MinX`, `MaxX` | f64 | m/z ÷ 20 |
- `MinX`/`MaxX` valgono il primo e l'ultimo m/z divisi per 20: il lettore non li usa. `BasePeakMZ` può differire fino a 0.05 dal m/z del punto più alto (taratura), `BasePeakValue` di qualche decina di conteggi da `MaxY`: il lettore usa i punti.
- `ScanTime` è la fine della scansione: il gruppo SIM di una scansione è quello in corso al tempo della scansione precedente (la scansione stampata 8.5009 min porta ancora i ioni del gruppo che finisce a 8.5).

## `MSPeak.bin`
- 68 byte di intestazione (`03 01` e zeri), poi i blocchi, uno per scansione, uno dopo l'altro: il primo è a 68 e l'ultimo finisce all'ultimo byte del file (nessun byte senza proprietario).
- Formato 1: `PointCount` valori m/z poi `PointCount` intensità, tutti f64 (16 byte per punto, `ByteCount = 16 × PointCount`). I valori sono precisi come un f32 (i 4 byte bassi sono zero). I punti sono CENTROIDI già filtrati (la scansione 50-500 ne ha da 14 a 300; una scansione SIM ne ha uno per ione del gruppo: 3 o 4) in ordine crescente di m/z. Nessuna compressione.
- Somma delle intensità = `TIC` del record entro 0.2 % (il TIC è un intero, le intensità sono tarate).

## Controlli fatti sulle tre corse (`crates/mzlab-core/tests/agilent_parity.rs`, saltano senza i dati)
Scansioni numerate 1…N, tempi crescenti, blocchi che coprono `MSPeak.bin` da 68 alla fine, totale di `MSTS.xml` = record, scansioni per segmento contate per tempo = `MSTS.xml`, testa di `MSTree2.bin` (conteggio, «Positive»), somma delle intensità = TIC, massimo = picco base, m/z crescenti, ioni di ogni scansione SIM = ioni del gruppo del metodo (`acqmeth.txt`), intervallo m/z della scansione dentro `Low Mass`/`High Mass`, mzML scritto e riletto uguale. Prove sintetiche (senza dati): ciclo completo, scansioni vuote, MS2 con precursore, errori.

## Come diventa `Run`/mzML
- `AgilentD::open(nome, &Files)` (record, segmenti, dispositivo, contenuti, albero, metodo), `run(&mut ReadAt)`, `visit`, `to_mzml_progress`. Uno spettro per scansione con punti (le vuote si saltano), `id = scan=<ScanID>`, centroide.
- Livello = `MSLevel`, precursore = `MzOfInterest` solo se livello ≥ 2 e > 0, polarità: codice 0 → +1, 1 → −1 (non visto), altro → 0; filtro «EI Scan» o «EI SIM 95.1,108.1,135.1» (EI se `Contents.xml` dice GC). TIC = `TIC` dei record di livello 1. Gli errori sono `err.file.notAgilentD` (manca `MSScan.bin`) e `err.file.agilent` con `detail` (dimensione del record, formato dello spettro, spettro fuori da `MSPeak.bin`, valore non numerico): mai un'ipotesi.

## Non verificato (nessun file vero)
- LC/MS e LC-QTOF MassHunter usano gli stessi nomi di file. Lo schema `MSScan.xsd` prevede campi opzionali (`UncompressedByteCount`, più `SpectrumParamValues` per scansione, `MeasuredNoise`, dati dipendenti): se un record ha un'altra dimensione o un altro `SpectrumFormatID` o i byte per punto non sono 16, il lettore si ferma con `err.file.agilent` e i numeri trovati. Non so se in quei file i profili stiano in `MSProfile.bin`, se siano compressi, o con quale precisione (f32/f64).
- Alta risoluzione, MS/MS (livello, `MzOfInterest`, `CollisionEnergy`, `Fragmentor`), polarità negativa (codice 1) e mobilità ionica: i campi esistono nello schema e si leggono come sopra, ma nessuna delle tre corse li usa; le mappe codice → significato (`ScanType` ≠ 1, 2; `IonMode`) sono sconosciute.
- Un `.D` vecchio senza `AcqData` (solo `data.ms`) non si apre ancora.
- Non so come MassHunter scriva MRM triplo quadrupolo (una scansione per transizione o un record con più punti): il TIC si legge, i cromatogrammi SRM no.
