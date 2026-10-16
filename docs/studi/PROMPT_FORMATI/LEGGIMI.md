# Prompt dei pacchetti dello studio sui formati nativi

Un file per pacchetto (dipendenze, moduli per famiglia e file reali: `docs/studi/FORMATI_NATIVI_MS.md`, §4). Ogni prompt è autosufficiente: si copia la parte dopo «---». Ordine definitivo:
1. DEV-00.
2. DEV-01: Thermo fuori dal motore, approvata.
3. DEV-03…DEV-08, una famiglia ciascuno, in parallelo dopo DEV-00; DEV-02 dopo DEV-01.
4. DEV-09…DEV-11 (motore) dopo DEV-01.

La dieta del motore non c'è più: è assorbita da DEV-01.

- [DEV-00](DEV-00.md): Infrastruttura comune dei moduli per famiglia
- [DEV-01](DEV-01.md): Thermo fuori dal motore: modulo `thermo` e lettore esterno
- [DEV-02](DEV-02.md): Thermo `.raw`: SRM dei TSQ, lock mass, PDA/UV (modulo `thermo`)
- [DEV-03](DEV-03.md): Formati aperti: mzML numpress, mzXML, netCDF/ANDI (modulo `aperti`)
- [DEV-04](DEV-04.md): Agilent MassHunter `.D` LC/MS: QqQ (MRM/dMRM), Q-TOF, negativo, DAD/UV, metodo LC/MS (modulo `agilent`)
- [DEV-05](DEV-05.md): SCIEX `.wiff` oltre il 3200 QTRAP, DAD (modulo `sciex`)
- [DEV-06](DEV-06.md): Waters MassLynx `.raw` (cartella) (modulo `waters`)
- [DEV-07](DEV-07.md): Shimadzu `.qgd` (GC/MS) e `.lcd` (LC/MS) (modulo `shimadzu`)
- [DEV-08](DEV-08.md): Bruker timsTOF TDF/TSF (modulo `bruker`)
- [DEV-11](DEV-11.md): `metodo/1` di un mzML scritto da altri
