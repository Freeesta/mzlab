# Cosa contengono i file grezzi e non estraiamo ancora

Regola: nessuna perdita di informazione dei file grezzi (`.wiff`, `.raw`, `.dam`). Questo elenco dice cosa il grezzo contiene e non arriva né nell'mzML né in `metodo/1`; ogni voce si chiude estraendola o dichiarandola.

## Thermo `.raw` (v66, Q Exactive)
- Spettri UV 3D (campo delle lunghezze d'onda di un rivelatore DAD/PDA Dionex: un controller a parte, 572 byte per istante nei file letti, layout non decodificato): `metodo/1.avvisi` dice «UV 3D field (PDA spectra)» e `rivelatori` ne dà l'intervallo e il passo; le tracce a una lunghezza d'onda (`UV_VIS_n`) sono cromatogrammi `Pda`. Lo stesso vale per i file con Accela PDA (non provati). I TSQ che non si aprono in `opentfraw` (Vantage/Quantum più vecchi) e le scansioni SRM senza Q1 non danno tracce.
- Trailer extra per scansione: 62 campi (tempo di iniezione, AGC, risoluzione FT, isolamento, carica, lock mass, ottica ionica, calibrazione, diagnostica). Nell'mzML ogni scansione ha i 33 `opentfraw.*` registrati e, per tutti gli altri, un `userParam` `opentfraw.trailer.<etichetta>` (`extra::trailer_extras`); in `metodo/1.campi` c'è ogni campo con il valore se è uguale in tutte le scansioni, altrimenti l'intervallo.
- `startTimeStamp` e SHA-1 del file: nell'mzML di un `.raw` scaricato dal sito (`rawlettore.rs`, `testata_mzml`) ci sono `startTimeStamp` (data del preambolo, altrimenti l'audit tag, ora locale dello strumento senza fuso; è il `file_creato` di `metodo/1`), lo SHA-1 del `.raw` nel `sourceFile` e il documento `metodo/1` nel `<run>` (userParam `mzLab metodo/1`). Il convertitore `raw::to_mzml` non scrive startTimeStamp né SHA-1.
- Pressione della pompa: il controller analogico (90 001 campioni `f64` prima del suo RunHeader, nome e unità nelle stringhe che lo seguono) esce come `pressure chromatogram` (MS:1003019) in pascal, tutti i campioni; altri canali analogici in un'altra unità non sono scritti (restano in `metodo/1.campi`, `Controller<N>`).
- Status log (v66): lo schema `GenericDataHeader` finisce esattamente a `inst_log_addr` e ogni record è un `f32` tempo in minuti + i campi (`opentfraw::status_log`, `RawFileReader::status_log`). I record per scansione di `inst_log` escono come `opentfraw.status.*`.
  - Ogni canale è un campo di `metodo/1.campi` (origine `thermo-raw:StatusLog`, valore se costante, altrimenti `min..max`, con unità); i titoli di sezione (`====  Ion Source  ====`) non hanno valore. `meta.nonDecodificato` resta solo se lo schema non si trova.
  - I canali che cambiano (almeno due valori diversi) sono serie nel tempo (`status:<sezione>/<etichetta>`), come i campi del trailer che cambiano (`trailer:<etichetta>`, un punto per scansione, separati per livello MS se cambiano in più livelli): `metodo/1.serie_registro` ne dà l'elenco (id, sezione, nome, unità, punti, min, max), i punti si calcolano a richiesta (`/api/registro?k=&id=`, `metodo_raw/serie.rs`). Nell'mzML e nel `.mzidx` non entrano (non sono cromatogrammi del file); nella vista Dati sono il tipo «Registro dello strumento».
  - Il registro degli errori (`error_log`) è in `metodo/1.registro` (`t_min`, `testo`, `livello` se il testo comincia con Info/Warning/Error) e nella finestra Metodo con i tempi.
- Tune: i valori «Tune Data» e la calibrazione di massa stanno nello schema del trailer (già in `campi`), il file di tune di base nel testo del metodo (`grezzo`, `InitialTuneFile`). Un blocco di tune a parte (LTQ/TSQ) non è stato verificato su file veri: finché non si prova, non si dichiara estratto.
- Il nome del computer e i percorsi sono scritti nel `<run>` dell'mzML (userParam `opentfraw.*`); il documento `metodo/1` che porta li riporta con `operatore`, `strumento.serie` e i campi marcati `identita`.

## SCIEX `.wiff` / `.dam`
- Non decodificati: PDA/UV 3D del `.wiff` (flussi `DAD*`: di `DADSampleHeader` è letta l'intestazione, in `campi`; i valori di `DADIdx` e `DADRealTimeData` no, perché manca un file UV di riferimento), tracce `Devices/Device_N/DevData` (analogiche, pressione), `DDERealTimeData` (IDA), blocchi dei TripleTOF/ZenoTOF (altro codec, rifiutati), binari `MSConfigInfo*`, `MassSpecMethod*`, `ExperimentHeader*Ex`, `PeriodStream*`, `CycleDepParam`.
- Calibrazione di massa e risoluzione (`GLPTables`) e stringhe di `DabsInfo`/`SampleTable`: in `metodo/1.campi` dal modulo `sciex`; stato del Log a inizio e fine corsa, audit CFR, ora di acquisizione per campione: non estratti.
- Utente, computer, ente e percorsi stanno nelle proprietà OLE e in `CFR_INFO`: identità, non copiati nell'mzML.

## Agilent `.D`
- Tipo di campione (`sequence.xml`), metodo GC completo (`acqmeth.txt`, `7890Method.bin`), valori effettivi (`GC.ini`), metodo di quantificazione, audit trail.

## mzML con numpress (modulo «aperti»)
- Nello spettro riscritto, i cvParam che lo scrittore mzML non conosce passano come userParam «ACC nome» (non come cvParam); `dataProcessing`, `referenceableParamGroup` e `scanSettings` del file originale non sono portati.

## mzXML (modulo «aperti»)
- Non portati: il testo degli elementi della testata (p. es. lo SHA-1 in coda al file), `scanOrigin`, `maldi`, `separation`/`spotting` di una scansione, `processingOperation`; gli attributi di ogni elemento della testata stanno invece tutti in `metodo/1.campi`, quelli di ogni scansione in `intestazione`.
