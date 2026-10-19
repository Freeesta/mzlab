# Prompt dei pacchetti del progetto «Rosina»

Un file per pacchetto; piano, ordine e dipendenze in `docs/studi/ROSINA.md` E; schede per formato in `docs/studi/rosina/`. Ogni prompt è autosufficiente: si copia la parte dopo «---». Ordine: LOC-1 e R-00 subito e in parallelo; LOC-2 dopo R-00 e LOC-1; R-01 e R-08 dopo R-00; R-02…R-07 dopo LOC-2, in parallelo tra loro. Priorità se non tutte insieme: R-02, R-03, R-04, R-01, R-05, R-06, R-07, R-08.

- [LOC-1](LOC-1.md): Scaricare ELENCO-1, contare MassIVE, provare gli oracoli sul Mac (fatto il 19/10)
- [R-00](R-00.md): Banco comune: verità `verita/1`, strumenti `tools/rosina/`, test di parità e fuzz dei moduli
- [LOC-2](LOC-2.md): Oracoli su tutto il corpus: TRFP sul Mac, msconvert e `timsdata` su Windows (Parallels), verità
- [R-01](R-01.md): Convertitore mzML: validazione, profilo e centroidi dichiarati, andata e ritorno
- [R-02](R-02.md): Thermo `.raw`: TSQ, trappole, tune, SPS/MS3, metadati contro TRFP
- [R-03](R-03.md): SCIEX `.wiff`: QTRAP oltre il 3200, polarità mista, metadati; TripleTOF/ZenoTOF nel modulo `sciex-tof`
- [R-04](R-04.md): Agilent `.D`: Q-TOF, valori effettivi, DAD, sezione GC, decisione sul 6560
- [R-05](R-05.md): Waters `.raw`: TOF, MSe, DDA, lock mass come opzione, deriva
- [R-06](R-06.md): Bruker TDF/TSF: 1/K0, compressione 2, TSF; BAF come guida
- [R-07](R-07.md): Shimadzu: parità con msconvert e `.cdf`, polarità, CE/dwell, PDA; 9030 nel modulo `shimadzu-tof`
- [R-08](R-08.md): Formati aperti minori: netCDF di altri produttori, mzXML con riferimento, mzData, MGF, nota imzML

Regole comuni: in fondo a ogni prompt (chat DEV e chat locale hanno due blocchi diversi). Il Coordinatore cambia l'id di sessione in tutti i file se cambia Coordinatore (`grep -l session_01Au8 docs/studi/PROMPT_ROSINA/*`).
