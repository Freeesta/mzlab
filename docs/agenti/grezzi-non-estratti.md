# Cosa contengono i file grezzi e non estraiamo ancora

Regola: nessuna perdita di informazione dei file grezzi (`.wiff`, `.raw`, `.dam`). Questo elenco dice cosa il grezzo contiene e oggi non arriva né nell'mzML né in `metodo/1`; ogni voce si chiude estraendola o dichiarandola. Fonte: audit dell'Architetto (repository privato `mzlab-dati`, `scratchpad/ARCHITETTO_METADATI.md`).

## Thermo `.raw` (v66, Q Exactive)
- Trailer extra per scansione: 62 campi (tempo di iniezione, AGC, risoluzione FT, isolamento, carica, lock mass, ottica ionica, calibrazione, diagnostica); `opentfraw` ne mappa 33 come `opentfraw.*` e noi non li rileggiamo.
- `startTimeStamp` (il FILETIME sta negli audit tag) e SHA-1 del file: non scritti nell'mzML.
- Metodo dello strumento: `raw::instrument_method_text` dà i blocchi di testo UTF-16 latino (panoramica del metodo LC con il gradiente, riepilogo dello strumento, XML dell'Orbitrap) separati da una riga vuota; non è ancora interpretato in `metodo/1` (nessun campo `lc`/`ms` dal `.raw`).
- Pressione della pompa (controller analogico), status log (temperature, vuoto), tune data, rumore e baseline per picco FT, riga di sequenza (vial, volume, diluizione, ISTD, commento), seriale e versioni dello strumento.
- Il nome del computer e i percorsi sono scritti nel `<run>` dell'mzML (userParam `opentfraw.*`); `raw::Convert { anonymous }` li toglie (`rawfilter.rs`).

## SCIEX `.wiff` / `.dam`
- Decodifica mancante: PDA 3D del `.wiff` (flussi `DAD*`), binari `MSConfigInfo*`, `MassSpecMethod*`, `ExperimentHeader*Ex`, `PeriodStream*`, `CycleDepParam`.
- Calibrazione di massa e risoluzione (`GLPTables`), stato del Log a inizio e fine corsa, audit CFR, ora di acquisizione per campione (`SampleTable`).
- Utente, computer, ente e percorsi stanno nelle proprietà OLE e in `CFR_INFO`: identità, non copiati nell'mzML.

## Agilent `.D`
- Tipo di campione (`sequence.xml`), metodo GC completo (`acqmeth.txt`, `7890Method.bin`), valori effettivi (`GC.ini`), metodo di quantificazione, audit trail.
