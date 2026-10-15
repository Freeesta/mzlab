# Cosa contengono i file grezzi e non estraiamo ancora

Regola: nessuna perdita di informazione dei file grezzi (`.wiff`, `.raw`, `.dam`). Questo elenco dice cosa il grezzo contiene e non arriva né nell'mzML né in `metodo/1`; ogni voce si chiude estraendola o dichiarandola.

## Thermo `.raw` (v66, Q Exactive)
- Trailer extra per scansione: 62 campi (tempo di iniezione, AGC, risoluzione FT, isolamento, carica, lock mass, ottica ionica, calibrazione, diagnostica). Nell'mzML ogni scansione ha i 33 `opentfraw.*` registrati e, per tutti gli altri, un `userParam` `opentfraw.trailer.<etichetta>` (`extra::trailer_extras`); in `metodo/1.campi` c'è ogni campo con il valore se è uguale in tutte le scansioni, altrimenti l'intervallo.
- `startTimeStamp` e SHA-1 del file: nell'mzML di un `.raw` scaricato dal sito (`rawlettore.rs`, `testata_mzml`) ci sono `startTimeStamp` (data del preambolo, altrimenti l'audit tag, ora locale dello strumento senza fuso; è il `file_creato` di `metodo/1`), lo SHA-1 del `.raw` nel `sourceFile` e il documento `metodo/1` nel `<run>` (userParam `mzLab metodo/1`). Il convertitore `raw::to_mzml` non scrive startTimeStamp né SHA-1.
- Pressione della pompa: il controller analogico (90 001 campioni `f64` prima del suo RunHeader, nome e unità nelle stringhe che lo seguono) esce come `pressure chromatogram` (MS:1003019) in pascal, tutti i campioni; altri canali analogici in un'altra unità non sono scritti (restano in `metodo/1.campi`, `Controller<N>`).
- Status log (v66): lo schema `GenericDataHeader` finisce esattamente a `inst_log_addr` e ogni record è un `f32` tempo in minuti + i campi (`opentfraw::status_log`, `RawFileReader::status_log`). I record per scansione di `inst_log` escono come `opentfraw.status.*`.
  - Ogni canale è un campo di `metodo/1.campi` (origine `thermo-raw:StatusLog`, valore se costante, altrimenti `min..max`, con unità); i titoli di sezione (`====  Ion Source  ====`) non hanno valore. `meta.nonDecodificato` resta solo se lo schema non si trova.
  - `StatusLog::series()` dà le serie nel tempo con nome e unità; nell'mzML non diventano cromatogrammi (msconvert non li scrive), restano in `metodo/1.campi`.
- Tune: i valori «Tune Data» e la calibrazione di massa stanno nello schema del trailer (già in `campi`), il file di tune di base nel testo del metodo (`grezzo`, `InitialTuneFile`).
- Il nome del computer e i percorsi sono scritti nel `<run>` dell'mzML (userParam `opentfraw.*`); il documento `metodo/1` che porta li riporta con `operatore`, `strumento.serie` e i campi marcati `identita`.

## SCIEX `.wiff` / `.dam`
- Non decodificati: PDA/UV 3D del `.wiff` (flussi `DAD*`; `wiff/` non ha un lettore: per scriverlo servono un `.wiff` con PDA e il suo mzML di msconvert, con la traccia UV da confrontare), binari `MSConfigInfo*`, `MassSpecMethod*`, `ExperimentHeader*Ex`, `PeriodStream*`, `CycleDepParam`.
- Calibrazione di massa e risoluzione (`GLPTables`), stato del Log a inizio e fine corsa, audit CFR, ora di acquisizione per campione (`SampleTable`).
- Utente, computer, ente e percorsi stanno nelle proprietà OLE e in `CFR_INFO`: identità, non copiati nell'mzML.

## Agilent `.D`
- Tipo di campione (`sequence.xml`), metodo GC completo (`acqmeth.txt`, `7890Method.bin`), valori effettivi (`GC.ini`), metodo di quantificazione, audit trail.

## mzML con numpress (modulo «aperti»)
- Nello spettro riscritto, i cvParam che lo scrittore mzML non conosce passano come userParam «ACC nome» (non come cvParam); `dataProcessing`, `referenceableParamGroup` e `scanSettings` del file originale non sono portati.
