# Cosa contengono i file grezzi e non estraiamo ancora

Regola: nessuna perdita di informazione dei file grezzi (`.wiff`, `.raw`, `.dam`). Questo elenco dice cosa il grezzo contiene e non arriva né nell'mzML né in `metodo/1`; ogni voce si chiude estraendola o dichiarandola.

## Thermo `.raw` (v66, Q Exactive)
- Spettri UV 3D (campo delle lunghezze d'onda di un rivelatore DAD/PDA Dionex: un controller a parte, 572 byte per istante nei file letti, layout non decodificato): `metodo/1.avvisi` dice «UV 3D field (PDA spectra)» e `rivelatori` ne dà l'intervallo e il passo; le tracce a una lunghezza d'onda (`UV_VIS_n`) sono cromatogrammi `Pda`. Lo stesso vale per i file con Accela PDA (non provati). I TSQ che non si aprono in `opentfraw` (Vantage/Quantum più vecchi) e le scansioni SRM senza Q1 non danno tracce.
- Trailer extra per scansione: 62 campi (tempo di iniezione, AGC, risoluzione FT, isolamento, carica, lock mass, ottica ionica, calibrazione, diagnostica). Nell'mzML ogni scansione ha i 33 `opentfraw.*` registrati e, per tutti gli altri, un `userParam` `opentfraw.trailer.<etichetta>` (`extra::trailer_extras`); in `metodo/1.campi` c'è ogni campo con il valore se è uguale in tutte le scansioni, altrimenti l'intervallo.
- `startTimeStamp` e SHA-1 del file: nell'mzML di un `.raw` scaricato dal sito (`rawlettore.rs`, `testata_mzml`) ci sono `startTimeStamp` (data del preambolo, altrimenti l'audit tag, ora locale dello strumento senza fuso; è il `file_creato` di `metodo/1`), lo SHA-1 del `.raw` nel `sourceFile` e il documento `metodo/1` nel `<run>` (userParam `mzLab metodo/1`). Il convertitore `raw::to_mzml` non scrive startTimeStamp né SHA-1.
- Pressione della pompa: il controller analogico (90 001 campioni `f64` prima del suo RunHeader, nome e unità nelle stringhe che lo seguono) esce come `pressure chromatogram` (MS:1003019) in pascal, tutti i campioni; altri canali analogici in un'altra unità non sono scritti (restano in `metodo/1.campi`, `Controller<N>`).
- Status log (v66): lo schema `GenericDataHeader` finisce esattamente a `inst_log_addr` e ogni record è un `f32` tempo in minuti + i campi (`opentfraw::status_log`, `RawFileReader::status_log`). I record per scansione di `inst_log` escono come `opentfraw.status.*`.
  - Ogni canale è un campo di `metodo/1.campi` (origine `thermo-raw:StatusLog`, valore se costante, altrimenti `min..max`, con unità); i titoli di sezione (`====  Ion Source  ====`) non hanno valore. `meta.nonDecodificato` resta solo se lo schema non si trova.
  - `StatusLog::series()` dà le serie nel tempo con nome e unità; nell'mzML non diventano cromatogrammi (msconvert non li scrive), restano in `metodo/1.campi`.
- Tune: i valori «Tune Data» e la calibrazione di massa stanno nello schema del trailer (già in `campi`), il file di tune di base nel testo del metodo (`grezzo`, `InitialTuneFile`).
- Il nome del computer e i percorsi sono scritti nel `<run>` dell'mzML (userParam `opentfraw.*`); il documento `metodo/1` che porta li riporta con `operatore`, `strumento.serie` e i campi marcati `identita`.

## SCIEX `.wiff` / `.dam`
- Non decodificati: PDA/UV 3D del `.wiff` (flussi `DAD*`: di `DADSampleHeader` è letta l'intestazione, in `campi`; i valori di `DADIdx` e `DADRealTimeData` no, perché manca un file UV di riferimento), tracce `Devices/Device_N/DevData` (analogiche, pressione), `DDERealTimeData` (IDA), blocchi dei TripleTOF/ZenoTOF (altro codec, rifiutati), binari `MSConfigInfo*`, `MassSpecMethod*`, `ExperimentHeader*Ex`, `PeriodStream*`, `CycleDepParam`.
- Calibrazione di massa e risoluzione (`GLPTables`) e stringhe di `DabsInfo`/`SampleTable`: in `metodo/1.campi` dal modulo `sciex`; stato del Log a inizio e fine corsa, audit CFR, ora di acquisizione per campione: non estratti.
- Utente, computer, ente e percorsi stanno nelle proprietà OLE e in `CFR_INFO`: identità, non copiati nell'mzML.

## Agilent `.D`
- Tipo di campione (`sequence.xml`), metodo GC completo (`acqmeth.txt`, `7890Method.bin`), valori effettivi (`GC.ini`), metodo di quantificazione, audit trail.
- LC/MS (modulo `agilent`, `agilent-lcms.md`): i dati del rivelatore a diodi (`AcqData/DAD1.cd/.cg/.sd/.sp`), le tracce dei moduli (pressione, flusso, temperatura: `*.cd`/`*.cg`), i valori effettivi per scansione (`MSActual.bin`) e i file binari che non si decodificano (`MSMassCal.bin` si legge, il resto no) sono nominati in `campi` con dimensione e SHA-1 (se sotto 4 MiB) e in `avvisi` (`meta.nonDecodificato`); `rivelatori` viene dal metodo, non dai dati. I file ChemStation `.ch`/`.uv` non esistono nelle corse vere.

## mzML con numpress (modulo «aperti»)
- Nello spettro riscritto, i cvParam che lo scrittore mzML non conosce passano come userParam «ACC nome» (non come cvParam); `dataProcessing`, `referenceableParamGroup` e `scanSettings` del file originale non sono portati.

## mzXML (modulo «aperti»)
- Non portati: il testo degli elementi della testata (p. es. lo SHA-1 in coda al file), `scanOrigin`, `maldi`, `separation`/`spotting` di una scansione, `processingOperation`; gli attributi di ogni elemento della testata stanno invece tutti in `metodo/1.campi`, quelli di ogni scansione in `intestazione`.

## Waters MassLynx `.raw` (modulo `waters`)
- Mobilità ionica (HDMSe, SONAR, cyclic IMS): gli m/z si sommano su tutti i bin di deriva; il tempo di deriva e la calibrazione di mobilità (`mob_cal.csv`) non sono estratti. Lock mass: la funzione è data come cromatogramma e in `campi`, ma la correzione dell'm/z non è applicata (30-100 ppm).
- Non decodificati: le righe di `_FUNCnnn.STS`/`.EE` oltre «Set Mass» e «Collision Energy» (stanno solo nei byte), la tabella `_HISTORY.INF`, i parametri di rampa oltre quelli di `_extern.inf`, la codifica a 6 byte (nessun file vero) e le funzioni di tipo MALDI.
- Identità (`User Name`, `Submitter`, `Laboratory Name`, percorsi del computer): in `metodo/1.campi` marcate `identita`, mai nell'mzML.

## Shimadzu `.qgd` / `.lcd` (modulo `shimadzu`)
- Non letti: programma del forno e dell'iniettore (`GC-2010/GC17A Instrument Parameters`), `GCMS Configuration` (solo il nome dello strumento), il metodo `.qgm` incorporato, `Status` e `Status Curve` (pressioni, temperature), `MS Status Data`, `Mass Parameters`, `GUMM_Information`; UV (`LSS Raw Data`, `PDA 3D Raw Data`); l'evento della polarità di `Mass Raw Data`; energia di collisione e tempo di attesa dei record MRM; il Q-TOF/IT-TOF/9030 (`QTFL RawData`) per intero.
- Non portati nell'mzML: operatore, computer, percorsi e nomi dei file di dati, metodo, tune e batch (`identita`). Dettagli e offset in `docs/agenti/shimadzu.md`.
