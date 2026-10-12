# Lettore Agilent MassHunter `.D` (GC/MS)

Stato attuale. `crates/mzlab-core/src/agilent/`: Rust puro, nessun crate nuovo. Una cartella `.D` si legge dai suoi file: `AcqData/MSScan.bin` (indice, un record per scansione) e `AcqData/MSPeak.bin` (i punti), più i file piccoli (`MSTS.xml`, `Devices.xml`, `Contents.xml`, `MSTree2.bin`, `acqmeth.txt` nella cartella `.D`).

Il formato è stato ricavato da tre corse vere di un GC/MS a singolo quadrupolo 5977 (EI, positivo; una in scansione 50-500 m/z, due in SIM) e dallo schema `AcqData/MSScan.xsd` che Agilent mette nella cartella; Riferimento: gli mzML che ProteoWizard msconvert ha fatto dalle stesse tre corse (64 bit, zlib, senza peak picking, SIM come spettri; repository dati, `GC-MS/mzML/`), più controlli interni (sotto). Tutto ciò che non è nelle tre corse è elencato in «Non verificato».

## File di una cartella `.D` e a cosa servono
- `AcqData/MSScan.bin`: indice (formato sotto). `AcqData/MSPeak.bin`: punti. Bastano questi due per leggere tutto.
- `AcqData/MSTS.xml`: segmenti di tempo (`TimeSegment`: `StartTime`, `EndTime` in minuti, `NumOfScans`). `NumOfScans` conta per TEMPO: la scansione a cavallo tra due segmenti SIM può avere nel record il segmento precedente e `MSTS.xml` il successivo (differenza di 1 tra i due conteggi; il totale coincide).
- `AcqData/MSIndex.bin` (solo nelle corse SIM): per ogni segmento una breve intestazione (con il numero di scansioni del segmento) e i tempi di scansione (f64, minuti); verificato il primo segmento: tempi uguali a quelli dei record. Non serve al lettore.
- `AcqData/MSTree2.bin`: `u32 86, u32 5`, tre testi (`u8` lunghezza + byte: dispositivo, tecnica, strumento), quattro f64 (primo e ultimo tempo, m/z minimo e massimo), `00 01 01 00 00 00`, testo del tipo di scansione («Scan», «SelectedIon»), `u32 1`, testo della polarità («Positive»), `u32` numero di scansioni, poi i tempi e una struttura che non si legge. `info::tree` legge solo questa testa e dà `None` se la forma è diversa.
- `AcqData/Devices.xml` (nome, modello, seriale, firmware), `Contents.xml` (data, tecnica di separazione 2 = GC, software), `sample_info.xml` e `sequence.xml` (campione, operatore, vial: non letti), `acqmeth.txt` (UTF-16, il metodo; `method::parse`), `*.M/` (metodo di analisi dati: non letto), `data.ms` (formato ChemStation vecchio, «GC / MS Data File»: i dati di partenza da cui `IndexedDataConverter.log` ha generato `AcqData`; non letto), `checksum.xml`, `GC.ini`, `runstart.txt`, `PRE_POST.INI`, `TEMPBASE`.

## Tutto ciò che contiene un `.D` e se lo estraggo
Regola: nessuna informazione dei file grezzi si butta; ciò che non so mappare si conserva come nome/valore con il file di origine (`AgilentD::metadata`, `meta::collect`; il chiamante passa i file piccoli in `Files::others`).

| File | Contenuto | Estratto? |
|---|---|---|
| `AcqData/MSScan.bin` | indice, un record per scansione | tutti i campi: i mappati in `ScanRec`, gli altri (stato, periodo di campionamento, intervallo di massa misurato, soglia, `MinX`/`MaxX`…) in `ScanRec::extra` con i nomi in `AgilentD::extra_names`; l'intestazione di 160 byte (a parte l'offset a +88) non è decodificata |
| `AcqData/MSPeak.bin` | punti (m/z, intensità) | sì; i 68 byte di intestazione non sono decodificati |
| `AcqData/MSScan.xsd` | schema dei record | sì, determina il layout; conservato come testo |
| `AcqData/MSTS.xml` | segmenti di tempo | sì (`segments`, e tutti i campi in `metadata`) |
| `AcqData/MSIndex.bin` | tempi di scansione per segmento | no (sono uguali ai tempi dei record; verificato sul primo segmento) |
| `AcqData/MSTree2.bin` | strumento, tipo di scansione, polarità in testo, numero di scansioni, tempi, poi una struttura sconosciuta | solo la testa (`tree`); il resto non è decodificato |
| `AcqData/Devices.xml`, `Contents.xml` | modello, seriale, firmware, driver, data, tecnica, software | sì, tutti i campi in `metadata` |
| `AcqData/sample_info.xml`, `sequence.xml` | campione, vial, volume, operatore, metodo, percorsi | sì, in `metadata` (nome/valore e attributi) |
| `AcqData-CumulativeAuditTrail.xml`, `checksum.xml` | traccia di audit (utente, computer, data, commento), hash | sì, in `metadata` |
| `acqmeth.txt` | metodo (forno, ALS, ingresso, colonna, linea di trasferimento, MS, gruppi SIM, tune) | testo intero + modo, ritardo del solvente, intervallo, gruppi SIM con ioni e dwell + parametri di tune (`tune NOME`) |
| `GC.ini`, `PRE_POST.INI` | stato del GC a fine corsa (temperature, flussi, colonne), tune, fattori, iniezione, hardware | sì, `[sezione] chiave = valore` in `metadata` |
| `runstart.txt`, `IndexedDataConverter.log` | variabili di inizio corsa, log del convertitore | testo intero in `metadata` |
| `TEMPBASE` | vuoto | no |
| `data.ms` | formato ChemStation vecchio, dati di partenza della conversione | non letto (formato non decodificato) |
| `AcqData/*.M/` e `*.m/` (copia del metodo di acquisizione, `DaMethod/Quant`, `Unknowns`) | RTP, monitor, tracce di audit, quantificazione (RT, ioni, livelli, curva) | XML sì (`metadata`), binari e testi non decodificati con SHA-1; i `.m` fuori dalla corsa non sono letti |
| TIC e picco base | TIC del record per scansione (= TIC dell'mzML) e `BasePeakMZ`/`BasePeakValue` | TIC sì (cromatogramma); il picco base è nel record, non scritto in un cromatogramma |

## `MSScan.bin`
- Il layout del record si legge da `AcqData/MSScan.xsd` della cartella (`Layout::from_xsd`: gli `xs:element` di `ScanRecordType` in ordine, quelli in commento esclusi, `SpectrumParamValues` = gli elementi di `SpectrumParamsType`; tipi int/long/double/short/byte/float). Per le tre corse l'xsd dà esattamente i 184 byte e gli offset della tabella (verificato dal test). Senza xsd vale `Layout::builtin` (la stessa tabella).
- Un tipo ignoto, un campo obbligatorio mancante o una dimensione che non divide il file sono errori con i numeri (e, se `MSTS.xml` annuncia N scansioni, la dimensione di record che il file avrebbe).
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

## Parità con msconvert (`crates/mzlab-core/tests/agilent_msconvert.rs`, salta senza i dati)
- Sulle tre corse: stesso numero di spettri (3417, 3417, 3868), stessi punti per spettro, intensità uguali (scarto 0), tempo entro 1e-9 min (msconvert scrive 12 decimali), livello 1, polarità positiva, TIC (cromatogramma) uguale punto per punto, TIC e picco base per scansione uguali a quelli del record.
- Differenze note: m/z fino a 1.5e-5 (msconvert li scrive come decimali brevi, il file ha valori f32); nativeID `scanId=N` in msconvert, `scan=N` qui perché il programma ricava il numero di scansione da `scan=`; msconvert aggiunge il termine «SIM spectrum», le finestre di scansione e un cromatogramma SIC per ione (`Q1` = m/z del metodo meno 0.05, 7 nelle corse SIM): qui no, gli XIC li costruisce il programma dagli spettri.

## Audit di perdita (`tests/agilent_audit.rs`, `tests/agilent_audit/`)
- Inventario del grezzo fatto senza il codice del lettore: elenco dei file, valori di testo e di attributo degli XML (scansione di testo semplice), valori degli `.ini`, campi del layout dei record. Ogni voce deve stare in `AgilentD::metadata` (nome o valore, origine = percorso nella cartella), in `ScanRec` o `extra_names`, oppure in `scarti.txt` con il motivo. I file che non si decodificano (`MSTree2.bin`, intestazione di `MSScan.bin`, formati ignoti) entrano con «undecoded bytes» e «undecoded sha1».
- `noti.txt` (solo nomi di file e conteggi, i nomi dei metodi sono sostituiti da `*.M`): se un file noto dà meno voci di prima il test fallisce; si riscrive con `AUDIT_WRITE=1`. Prova sintetica sempre attiva: un campo mai visto si conserva.
- Il primo giro ha trovato quattro perdite vere, corrette: XML in UTF-16, entità (`&lt;` arriva a parte nel lettore XML), testo misto con figli, nome di visualizzazione dei campi di `sample_info.xml`.
- Le cartelle `*.M` dentro `AcqData` (copia del metodo di acquisizione: `7890Method.bin`, `Acq.RTP*.xml`, tracce di audit, `quantitative.xml` con ioni e livelli di calibrazione) sono lette come file piccoli (`Files::others`): XML a nome/valore, il resto dichiarato con SHA-1.

## Cosa scrive nell'mzML oltre agli spettri
- Intestazione: `startTimeStamp` = `AcquiredTime` di `Contents.xml` come scritto (con il suo fuso, senza frazioni di secondo); msconvert lo scrive 2 ore avanti (16:00:45+01:00 diventa 17:00:45Z), qui no. Seriale (MS:1000529) da `Devices.xml`; software di acquisizione MS:1000678 con la versione di `Contents.xml` (msconvert scrive 8.0, il file dice 10.0); `sampleList` e userParam «sample type» da `sequence.xml`; per ogni file letto formato MassHunter, nativeID format e SHA-1 (`MSPeak.bin` si calcola prima di convertire, in blocchi da 1 MB); `fileContent` con centroide, TIC e BPC.
- Per spettro: «SIM spectrum» (MS:1000582) nelle corse SIM, m/z minimo e massimo osservati, finestra di scansione (quella del metodo, `Low Mass`/`High Mass`, per le scansioni; primo e ultimo ione per SIM). Cromatogrammi: TIC e BPC (dai record). Non scritti: i SIC per ione (dati derivati: li costruisce il programma) e il titolo dello spettro.
- Metodo GC (`AcqMethod::gc`, `method::parse_gc`): forno (iniziale, attesa, rampe, equilibrazione), iniettore (modo, riscaldatore, pressione, flusso totale), colonna (descrizione con il seriale: campo di identità, dimensioni, flusso, modo, velocità, holdup), linea di trasferimento, volume iniettato. L'audit con msconvert è `tests/agilent_cv.rs`: ogni accessione CV di msconvert sta anche nel nostro mzML, salvo le eccezioni elencate nel test con il motivo.

## Come diventa `Run`/mzML
- `AgilentD::open(nome, &Files)` (record, segmenti, dispositivo, contenuti, albero, metodo), `run(&mut ReadAt)`, `visit`, `to_mzml_progress`. Uno spettro per scansione con punti (le vuote si saltano), `id = scan=<ScanID>`, centroide.
- Livello = `MSLevel`, precursore = `MzOfInterest` solo se livello ≥ 2 e > 0, polarità: codice 0 → +1, 1 → −1 (non visto), altro → 0; filtro «EI Scan» o «EI SIM 95.1,108.1,135.1» (EI se `Contents.xml` dice GC). TIC = `TIC` dei record di livello 1. Gli errori sono `err.file.notAgilentD` (manca `MSScan.bin`) e `err.file.agilent` con `detail` (dimensione del record, formato dello spettro, spettro fuori da `MSPeak.bin`, valore non numerico): mai un'ipotesi.

## Non verificato (nessun file vero)
- LC/MS e LC-QTOF MassHunter usano gli stessi nomi di file. Se il loro `MSScan.xsd` ha altri campi (per esempio `UncompressedByteCount`) il layout li segue; se un record ha più `SpectrumParamValues`, se `SpectrumFormatID` non è 1, se i byte per punto non sono 16 o se `UncompressedByteCount` differisce da `ByteCount` (spettri compressi) il lettore si ferma con `err.file.agilent` e i numeri trovati. Non so dove stiano i profili (forse un altro file), con quale precisione né con quale compressione: serve un file vero.
- Alta risoluzione, MS/MS (`MSLevel`, `MzOfInterest`, `CollisionEnergy`, `Fragmentor`), polarità negativa (codice 1) e mobilità ionica: i campi esistono nello schema e si leggono come sopra, ma nessuna delle tre corse li usa; i significati dei codici (`ScanType` ≠ 1, 2; `IonMode`; `IonPolarity` ≠ 0) sono sconosciuti.
- Un `.D` vecchio senza `AcqData` (solo `data.ms`) non si apre ancora.
- Non so come MassHunter scriva MRM triplo quadrupolo (una scansione per transizione o un record con più punti): il TIC si legge, i cromatogrammi SRM no.
