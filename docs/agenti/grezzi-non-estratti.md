# Cosa contengono i file grezzi e non estraiamo ancora

Regola: nessuna perdita di informazione dei file grezzi (`.wiff`, `.raw`, `.dam`). Questo elenco dice cosa il grezzo contiene e oggi non arriva né nell'mzML né in `metodo/1`; ogni voce si chiude estraendola o dichiarandola. Fonte: audit dell'Architetto (repository privato `mzlab-dati`, `scratchpad/ARCHITETTO_METADATI.md`).

## Thermo `.raw` (v66, Q Exactive)
- Trailer extra per scansione: 62 campi (tempo di iniezione, AGC, risoluzione FT, isolamento, carica, lock mass, ottica ionica, calibrazione, diagnostica). Nell'mzML ogni scansione ha i 33 `opentfraw.*` registrati e, per tutti gli altri, un `userParam` `opentfraw.trailer.<etichetta>` (`extra::trailer_extras`); in `metodo/1.campi` c'è ogni campo con il valore se è uguale in tutte le scansioni, altrimenti l'intervallo.
- `startTimeStamp` (il FILETIME sta negli audit tag; `metodo/1` lo dà come `origine.file_creato`) e SHA-1 del file: non scritti nell'mzML.
- Pressione della pompa: il controller analogico (90 001 campioni `f64` prima del suo RunHeader, nome e unità nelle stringhe che lo seguono) esce come `pressure chromatogram` (MS:1003019) in pascal, tutti i campioni; altri canali analogici in un'altra unità non sono scritti (restano in `metodo/1.campi`, `Controller<N>`).
- Status log (90 record binari, avviso `meta.nonDecodificato`), tune data.
- Il nome del computer e i percorsi sono scritti nel `<run>` dell'mzML (userParam `opentfraw.*`); il documento `metodo/1` che porta li riporta con `operatore`, `strumento.serie` e i campi marcati `identita`.

## SCIEX `.wiff` / `.dam`
- Decodifica mancante: PDA 3D del `.wiff` (flussi `DAD*`), binari `MSConfigInfo*`, `MassSpecMethod*`, `ExperimentHeader*Ex`, `PeriodStream*`, `CycleDepParam`.
- Calibrazione di massa e risoluzione (`GLPTables`), stato del Log a inizio e fine corsa, audit CFR, ora di acquisizione per campione (`SampleTable`).
- Utente, computer, ente e percorsi stanno nelle proprietà OLE e in `CFR_INFO`: identità, non copiati nell'mzML.

## Agilent `.D`
- Tipo di campione (`sequence.xml`), metodo GC completo (`acqmeth.txt`, `7890Method.bin`), valori effettivi (`GC.ini`), metodo di quantificazione, audit trail.
