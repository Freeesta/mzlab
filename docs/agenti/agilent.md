# Lettore Agilent MassHunter `.D` (GC/MS)

`crates/mzlab-core/src/agilent/`: Rust puro, nessun crate nuovo. Una cartella `.D` si legge dai suoi file: `AcqData/MSScan.bin` (indice, un record per scansione) e `AcqData/MSPeak.bin` (i punti), più i file piccoli (`MSTS.xml`, `Devices.xml`, `Contents.xml`, `MSTree2.bin`, `acqmeth.txt` nella cartella `.D`).

Il formato è stato ricavato da tre corse vere di un GC/MS a singolo quadrupolo 5977 (EI, positivo; una in scansione 50-500 m/z, due in SIM) e dallo schema `AcqData/MSScan.xsd` che Agilent mette nella cartella; Riferimento: gli mzML che ProteoWizard msconvert ha fatto dalle stesse tre corse (64 bit, zlib, senza peak picking, SIM come spettri; repository dati, `GC-MS/mzML/`), più controlli interni (sotto). Tutto ciò che non è nelle tre corse è elencato in «Non verificato».

## File di una cartella `.D` e a cosa servono
- `AcqData/MSScan.bin`: indice (formato sotto). `AcqData/MSPeak.bin`: punti. Bastano questi due per leggere tutto.
- `AcqData/MSTS.xml`: segmenti di tempo (`TimeSegment`: `StartTime`, `EndTime` in minuti, `NumOfScans`). `NumOfScans` conta per TEMPO: la scansione a cavallo tra due segmenti SIM può avere nel record il segmento precedente e `MSTS.xml` il successivo (differenza di 1 tra i due conteggi; il totale coincide).
- `AcqData/MSIndex.bin` (solo nelle corse SIM): per ogni segmento una breve intestazione (con il numero di scansioni del segmento) e i tempi di scansione (f64, minuti); verificato il primo segmento: tempi uguali a quelli dei record. Non serve al lettore.
- `AcqData/MSTree2.bin`: `u32 86, u32 5`, tre testi (`u8` lunghezza + byte: dispositivo, tecnica, strumento), quattro f64 (primo e ultimo tempo, m/z minimo e massimo), `00 01 01 00 00 00`, testo del tipo di scansione («Scan», «SelectedIon»), `u32 1`, testo della polarità («Positive»), `u32` numero di scansioni, poi i tempi. `info::tree` legge solo la testa e dà `None` se la forma è diversa.
- `MSTree2.bin` dopo i tempi: il TIC di ogni scansione come `f32` (uguale ai record in tutte e tre le corse; provato in `tests/agilent_tree.rs`), poi un blocco per ione: `01 00 00 00 00 00 00 00 00 00 00 00` (nel primo blocco; i blocchi seguenti iniziano con `01` e un indice), un `u32`, l'ione (m/z, f64), `u32` 1, quattro `u32` a zero, il numero di valori e i valori come `f32`.
- Nella corsa SIM dei dati (3 ioni, 3417 scansioni) i valori di un ione sono le sue intensità nelle 1142 scansioni che lo contengono (uguali a `MSPeak.bin`); nella corsa in scansione completa (ione 51.0, 216 valori) NON sono le intensità di `MSPeak.bin` (1639.9 contro 1453 nella quarta scansione): è un dato calcolato dal grezzo con una regola non nota.
- Dopo i blocchi degli ioni c'è una lunga coda di `u32` crescenti (fino a 4.8 MB) non decodificata. Niente di tutto questo è dato originale in più: sono copie e derivati dei record e di `MSPeak.bin`.
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
| `AcqData/MSTree2.bin` | strumento, tipo di scansione, polarità in testo, numero di scansioni, tempi, TIC `f32`, blocchi per ione, coda sconosciuta | solo la testa (`tree`); TIC e primo blocco di ione controllati nel test, il resto non è decodificato (dichiarato con byte e SHA-1) |
| `AcqData/Devices.xml`, `Contents.xml` | modello, seriale, firmware, driver, data, tecnica, software | sì, tutti i campi in `metadata` |
| `AcqData/sample_info.xml`, `sequence.xml` | campione, vial, volume, operatore, metodo, percorsi | sì, in `metadata` (nome/valore e attributi) |
| `AcqData-CumulativeAuditTrail.xml`, `checksum.xml` | traccia di audit (utente, computer, data, commento), hash | sì, in `metadata` |
| `acqmeth.txt` | metodo (forno, ALS, ingresso, colonna, linea di trasferimento, MS, gruppi SIM, tune) | testo intero + modo, ritardo del solvente, intervallo, gruppi SIM con ioni e dwell + parametri di tune (`tune NOME`) |
| `GC.ini`, `PRE_POST.INI` | stato del GC a fine corsa (temperature, flussi, colonne), tune, fattori, iniezione, hardware | sì, `[sezione] chiave = valore` in `metadata` |
| `runstart.txt`, `IndexedDataConverter.log` | variabili di inizio corsa, log del convertitore | testo intero in `metadata` |
| `TEMPBASE` | vuoto | no |
| `data.ms` | file ChemStation (big-endian) da cui MassHunter ha convertito `AcqData`; formato sotto | letto (`datams.rs`): stringhe dell'intestazione, scansioni, tempi, TIC; intestazione numerica e tabella dopo i record dichiarate con byte e SHA-1; solo se manca `AcqData` fornisce gli spettri |
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

## `data.ms`
- Intestazione: tipo `01 32`; stringhe Pascal (lunghezza + testo) a 0x04 (tipo di file), 0x18 (campione), 0x94 (operatore), 0xB2 (data), 0xD0 (strumento), 0xE4 (metodo); `u16` a 0x10A = inizio dei record in parole meno 2; `u32` a 0x116 = numero di scansioni (big-endian). Il resto dell'intestazione non è decodificato.
- Record: `u16` lunghezza in parole, `u32` tempo in ms, `u16` 12 o 34, `u16` 1, `u16` 0, `u16` n picchi, la coppia del picco base, n coppie `(u16 m/z×20, u16 abbondanza)` dal m/z più alto al più basso, poi `u16` 5, `u16` 4, `u16` 0 e `u32` TIC. Abbondanza: 2 bit di esponente e, 14 di mantissa, valore = mantissa × 8^e. Lunghezza in parole = (24 + 4 × (n + 1)) / 2.
- Dopo l'ultimo record segue una tabella (circa 20 % del file) non decodificata: è dichiarata con byte e SHA-1.


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
- Il lettore XML dei file piccoli gestisce XML in UTF-16, entità (`&lt;` arriva a parte), testo misto con figli e il nome di visualizzazione dei campi di `sample_info.xml`: l'audit lo verifica.
- Le cartelle `*.M` dentro `AcqData` (copia del metodo di acquisizione: `7890Method.bin`, `Acq.RTP*.xml`, tracce di audit, `quantitative.xml` con ioni e livelli di calibrazione) sono lette come file piccoli (`Files::others`): XML a nome/valore, il resto dichiarato con SHA-1.

## Cosa scrive nell'mzML oltre agli spettri
- Intestazione: `startTimeStamp` = `AcquiredTime` di `Contents.xml` come scritto (con il suo fuso, senza frazioni di secondo); msconvert lo scrive 2 ore avanti (16:00:45+01:00 diventa 17:00:45Z), qui no. Seriale (MS:1000529) da `Devices.xml`; software di acquisizione MS:1000678 con la versione di `Contents.xml` (msconvert scrive 8.0, il file dice 10.0); `sampleList` e userParam «sample type» da `sequence.xml`; per ogni file letto formato MassHunter, nativeID format e SHA-1 (`MSPeak.bin` si calcola prima di convertire, in blocchi da 1 MB); `fileContent` con centroide, TIC e BPC.
- Per spettro: «SIM spectrum» (MS:1000582) nelle corse SIM, m/z minimo e massimo osservati, finestra di scansione (quella del metodo, `Low Mass`/`High Mass`, per le scansioni; primo e ultimo ione per SIM). Cromatogrammi: TIC e BPC (dai record).
- SIC: nelle corse con scansioni SIM, un SIC (MS:1000627, id `SIC <m/z>`) per ogni ione, costruito dai punti delle scansioni SIM (ioni distinti a 0.01 m/z; due punti dello stesso ione nella stessa scansione si sommano). Il SIC è un dato derivato: msconvert non lo scrive, e non c'è una corsa SIM vera con cui confrontarlo (verificato solo su dati sintetici).
- Non scritto: il titolo dello spettro.
- Metodo GC (`AcqMethod::gc`, `method::parse_gc`): forno (iniziale, attesa, rampe, equilibrazione), iniettore (modo, riscaldatore, pressione, flusso totale), colonna (descrizione con il seriale: campo di identità, dimensioni, flusso, modo, velocità, holdup), linea di trasferimento, volume iniettato. L'audit con msconvert è `tests/agilent_cv.rs`: ogni accessione CV di msconvert sta anche nel nostro mzML, salvo le eccezioni elencate nel test con il motivo.

## `metodo/1` (`src/agilent/doc.rs`, `tests/agilent_metodo.rs`)
- `AgilentD::metodo(&Source)` dà il documento `metodo/1` della corsa (sezioni e mappatura in `docs/agenti/metodo.md`); `to_mzml` lo scrive nel `<run>` come `userParam`. Il lettore dei metodi del sito (`mzml_metodo`) lo trova quindi in ogni mzML di una `.D` convertita.
- Prove sui dati veri: le chiavi del documento e delle sue sezioni sono quelle di `metodo-1.json`, ogni nome/valore dei file laterali sta in `campi` (conteggio e uno per uno), i testi interi sono in `grezzo`, e l'mzML porta lo stesso documento con il SHA-1 di `MSPeak.bin`.

## Come `Lettore` (`src/agilent/lettore.rs`, `tests/agilent_lettore.rs`)
- Registrato in `LETTORI` come `agilent-d` (estensione `.d`). I membri del `FileSet` hanno i nomi della cartella (`AcqData/MSScan.bin`, `acqmeth.txt`, …; un prefisso `x.D/` dà il nome della corsa). `riconosci`: `Si` con un `MSScan.bin` la cui testa ha l'offset dei record plausibile (byte 88) e un `MSPeak.bin`, `Forse` senza `MSPeak.bin`; un `data.ms` con i byte `01 32` è `Si`. Nessun'altra cosa conta, l'estensione da sola no.
- `apri` legge intere solo l'indice e i file piccoli (contati contro `Limiti::byte`, le voci contro `Limiti::voci`); `MSPeak.bin` resta un `ReadAt`: `scansione(0, k)` legge i soli byte della scansione `k` (stesso `Spectrum` di `AgilentD::run`, provato su tutte le corse). Una corsa con il solo `data.ms` si serve dal testo letto all'apertura.
- `indice`: le scansioni con punti, con `Posizione` in `MSPeak.bin`; il filtro delle SIM non ha gli ioni (stanno nello spettro). `intestazione`: ogni campo del record (anche quelli extra dell'xsd) con il CV dove c'è. `cromatogrammi`: TIC, BPC e un SIC per ogni ione dei gruppi SIM del metodo; il SIC si calcola leggendo le scansioni SIM alla richiesta. `metodo`: il documento `metodo/1` senza il SHA-1 di `MSPeak.bin` (non si legge tutto per calcolarlo).

## Come diventa `Run`/mzML
- `AgilentD::open(nome, &Files)` (record, segmenti, dispositivo, contenuti, albero, metodo), `run(&mut ReadAt)`, `visit`, `to_mzml_progress`. Uno spettro per scansione con punti (le vuote si saltano), `id = scan=<ScanID>`, centroide.
- Livello = `MSLevel`, precursore = `MzOfInterest` solo se livello ≥ 2 e > 0, polarità: codice 0 → +1, 1 → −1 (non visto), altro → 0; filtro «EI Scan» o «EI SIM 95.1,108.1,135.1» (EI se `Contents.xml` dice GC). TIC = `TIC` dei record di livello 1. Gli errori sono `err.file.notAgilentD` (manca `MSScan.bin`) e `err.file.agilent` con `detail` (dimensione del record, formato dello spettro, spettro fuori da `MSPeak.bin`, valore non numerico): mai un'ipotesi.

## Nel sito (conversione in mzML, come il `.wiff`)
- `mzlab-wasm/src/agilent.rs`: `Engine::agilent_to_mzml(nome, MSScan.bin, MSScan.xsd, MSTS.xml, Devices.xml, Contents.xml, MSTree2.bin, acqmeth.txt, dimensione di MSPeak.bin, read, sink, progress)`: i file piccoli arrivano interi (array vuoto = file mancante), `MSPeak.bin` si legge per offset da Blob (finestra di 1 MB, mai intero), l'mzML esce a pezzi.
- Pagina: `agilent-d.js::toMzml` raggruppa i file di ogni `.D`, li passa a `rb.nativeSamples` come membri `x.D/AcqData/…` e ne fa un campione `x.mzML` letto così com'è (`lettura-nativa.md`, `rust-nativo.js`). Prova: `tests_e2e/e2e_lettori.py`, `e2e_agilent.py` (cartella sintetica).
- Cartella intera: `agilent-d.js` (`AgilentD`: `locate`, `group`, `mzmlName` puri e provati in node da `tests/test_agilent_d.py`; `toMzml` mostra l'avanzamento con `ConvUI`, annulla con il segnale, mette l'mzML in `CONV` per «Scarica mzML»). `CartCarica.load` (`cartella-carica.js`) raggruppa i file per cartella `.D` (percorso relativo) e passa a `upload()` un mzML per corsa insieme agli altri file; una `.D` senza `MSScan.bin`/`MSPeak.bin` dà un avviso con chiave (`err.file.notAgilentD`; con un `data.ms` la cartella si apre da quello), senza motore Rust `err.file.agilentNoEngine`.
- Prova nel browser: `tests_e2e/e2e_agilent.py` (cartella sintetica di `tools/agilent_sintetico.py`: due `.D` e un mzML, avviso, TIC, nessuna richiesta a Pyodide). Un `.D` trascinato o scelto senza «Carica una cartella» (file sparsi) non è riconosciuto; il desktop non converte i `.D`.
- Peso: il lettore sta nel motore (nessun caricamento a richiesta); il limite del `.wasm` è in `pages.yml`.
- Memoria: `MSScan.bin` intero (184 byte per scansione) più i record (circa 130 byte): 100 000 scansioni = circa 30 MB sui 192 MB di budget; se l'allocazione fallisce l'errore è `error.memoryBudget`. Gli spettri passano uno alla volta; l'mzML non sta in memoria.

## Non verificato (nessun file vero)
- LC/MS e LC-QTOF MassHunter usano gli stessi nomi di file. Se il loro `MSScan.xsd` ha altri campi (per esempio `UncompressedByteCount`) il layout li segue; se un record ha più `SpectrumParamValues`, se `SpectrumFormatID` non è 1, se i byte per punto non sono 16 o se `UncompressedByteCount` differisce da `ByteCount` (spettri compressi) il lettore si ferma con `err.file.agilent` e i numeri trovati. Non so dove stiano i profili (forse un altro file), con quale precisione né con quale compressione: serve un file vero.
- Alta risoluzione, MS/MS (`MSLevel`, `MzOfInterest`, `CollisionEnergy`, `Fragmentor`), polarità negativa (codice 1) e mobilità ionica: i campi esistono nello schema e si leggono come sopra, ma nessuna delle tre corse li usa; i significati dei codici (`ScanType` ≠ 1, 2; `IonMode`; `IonPolarity` ≠ 0) sono sconosciuti.
- `data.ms` non è un doppione esatto di `AcqData`: tempi, TIC e numero di scansioni sono uguali, ma gli m/z sono interi in ventesimi (fino a 0.05 dai valori tarati di `MSPeak.bin`) e le abbondanze hanno 14 bit di mantissa; letto da solo (corsa senza `AcqData`) dà questi valori. Il formato è stato verificato solo con le tre corse 5977 (tipo `01 32`); altre varianti (altro tipo di file, record di altra lunghezza) danno `err.file.agilent` con i numeri.
- Non so come MassHunter scriva MRM triplo quadrupolo (una scansione per transizione o un record con più punti): il TIC si legge, i cromatogrammi SRM no.
