# Rosina C: schede per formato

Una scheda per formato: stato, cosa manca, prove possibili (scala V1-V7 di `../ROSINA.md` §4), metadati campo per campo, lavoro, rischio, priorità. Le schede dei produttori stanno nei file accanto; qui l'indice, la legenda e i formati aperti.

| Formato | Scheda | Stato oggi | Priorità | Pacchetto |
|---|---|---|---|---|
| Thermo `.raw` | `SCHEDA_THERMO.md` | spettri L (Orbitrap, Q Exactive, Exploris); TSQ e trappole P | 1 | R-02 |
| SCIEX `.wiff` (QTRAP, API) | `SCHEDA_SCIEX.md` | L per il 3200 e QTRAP 5500/6500 (MRM, EPI, PIS, NL); periodi multipli rifiutati | 1 | R-03 |
| SCIEX `.wiff` TOF (TripleTOF, ZenoTOF, X500) | `SCHEDA_SCIEX.md` | rifiutati (codec dei blocchi TOF) | 2 | R-03 (modulo `sciex-tof`) |
| SCIEX `.wiff2` | `SCHEDA_SCIEX.md` | rifiutato | guida | — |
| Agilent `.D` GC/MS e LC/MS | `SCHEDA_AGILENT.md` | GC/MS L; QqQ MRM/dMRM e Q-TOF P; DAD non decodificato; IMS rifiutato | 1 | R-04 |
| Waters `.raw` | `SCHEDA_WATERS.md` | MRM/SIR P (V2 da Workbench); TOF, MSe P; lock mass non applicata; IMS sommata | 2 | R-05 |
| Bruker TDF/TSF | `SCHEDA_BRUKER.md` | TDF L per m/z (V1 su compressione 1); 1/K0 assente; compressione 2 solo V7 | 2 | R-06 |
| Bruker BAF/YEP | `SCHEDA_BRUKER.md` | non letto | guida | — |
| Shimadzu `.lcd`/`.qgd` | `SCHEDA_SHIMADZU.md` | P: nessun riferimento; polarità, CE e dwell non decodificati; 9030 rifiutato | 3 | R-07 |
| netCDF/ANDI | qui | L (V2 con `.D` della stessa corsa) | 3 | R-08 |
| mzML altrui | qui | L (testata `metodo/1` P: nessun mzML vero di msconvert) | 1 | R-01, R-08 |
| mzXML | qui | L, V7 soltanto | 3 | R-08 |
| mzData | qui | non letto | 4 | R-08 |
| MGF, MSP | qui | letti come librerie, non come corsa | 4 | R-08 |
| imzML | qui | non letto | fuori | nota in R-08 |
| LECO `.peg`, JEOL, PerkinElmer, AB SCIEX `.t2d` | qui | non letti | fuori | guida |

## Legenda delle tabelle dei metadati
- **Campo del produttore**: nome come lo scrive il software del produttore (o, se non c'è, la descrizione).
- **Dove nel file**: membro, flusso, tabella o offset; «?» = posizione non ancora trovata.
- **`metodo/1`**: percorso nel documento (`campi` = solo nell'elenco piatto con `origine` e `chiave`; `grezzo` = solo nel testo). Le identità (persone, computer, percorsi) hanno `identita` e non passano nell'mzML.
- **Stato**: **L** letto in una sezione; **C** solo in `campi`/`grezzo`; **N** non letto o non decodificato; **?** da misurare nel pacchetto indicato.
- **Controllo**: come lo mostra il software del produttore (V3) e quale oracolo automatico lo dà (V1/V2). «mzML» = l'mzML di msconvert lo scrive (cvParam o userParam); «TRFP» = JSON di ThermoRawFileParser `-m 0`.
- Ogni riga **N** o **?** di una scheda diventa un punto del pacchetto, con la prova indicata, oppure un `avviso` `meta.nonDecodificato` dichiarato nel documento: mai silenzio.

## Formati aperti

### netCDF/ANDI-MS (`.cdf`)
- **Stato**: lettore `andi` L (CDF1/CDF2, SIM, profilo, polarità); V2 contro il `.D` della stessa corsa. Esportato da ChemStation, Xcalibur, LabSolutions, ChromaTOF.
- **Manca**: file esportati da LECO, Shimadzu e Thermo (il corpus ha soprattutto Agilent); la variante ANDI-Chrom (solo UV, `.cdf` senza `mass_values`): oggi errore onesto?; netCDF4/HDF5 rifiutato (giusto: non è ANDI).
- **Prove**: V2 (nativo della stessa corsa), V4 (netCDF4-python, xcms `readMSData`), specifica ASTM E2077 (variabili obbligatorie).
- **Lavoro**: 1-2 giorni (R-08). **Rischio**: basso. **Priorità**: 3.

| Campo | Dove | `metodo/1` | Stato | Controllo |
|---|---|---|---|---|
| produttore, modello, seriale, firmware, software | attributi globali `instrument_mfr`, `_model`, `_serial_no`, `_fw_version`, `_sw_version` | `strumento.*` | L | `ncdump -h` |
| data dell'esperimento | `experiment_date_time_stamp` | `origine.file_creato` | L | `ncdump -h` |
| titolo, operatore | `experiment_title`, `operator_name` | `campione.nome`, `operatore.utente` | L | `ncdump -h` |
| ionizzazione, polarità, funzione di scansione, tipo | `test_ionization_mode`, `_polarity`, `test_scan_function`, `experiment_type` | `ms.sorgente.tipo`, scansioni | L | `ncdump -h` |
| intervallo di massa, risoluzione, tempo di scansione, detector | `test_*` restanti | `campi` | ? (R-08) | `ncdump -h` |
| fonte dei dati (file nativo, software di esportazione) | `source_file_reference`, `netcdf_file_date_time_stamp`, `languages` | `campi` | ? (R-08) | `ncdump -h` |
| per scansione: RT, TIC, intervalli | `scan_acquisition_time`, `total_intensity`, `mass_range_*` | spettri, TIC | L | xcms |

### mzML scritto da altri
- **Stato**: lettore e testata L; `metodo_mzml.rs` legge la testata fino a 64 scansioni. Non verificato su mzML veri di msconvert, OpenMS, MZmine, SCIEX MS Data Converter, Bruker CompassXport.
- **Manca**: corpus di mzML di 6 scrittori; indice `indexedmzML` sbagliato o assente (ripiego per scansione?); numpress L (modulo `aperti`); cromatogrammi senza spettri (SRM puri) L; `referenceableParamGroup` usati da Agilent e SCIEX; mobilità come array (`MS:1002816`) e come scansioni separate.
- **Prove**: V4 (pyteomics, OpenMS `FileInfo -v`, `psims`), XSD `mzML1.1.0_idx.xsd`, semantica PSI (`FileInfo -c`).
- **Lavoro**: 2-3 giorni (R-01 il convertitore, R-08 la testata dei veri). **Rischio**: basso. **Priorità**: 1 (è l'uscita di ogni convertitore e l'ingresso più comune: 27 % degli studi MetaboLights).
- Campi: ogni elemento della testata (`fileDescription`, `sourceFileList` con SHA-1, `instrumentConfigurationList`, `softwareList`, `dataProcessingList`, `scanSettingsList`, `sampleList`, `run@startTimeStamp`) va in `campi` con `mzml:<elemento>`; i parametri per scansione (tempo d'iniezione, risoluzione, filtro, CE, isolamento) in `ms.periodi[0].esperimenti[]` se costanti, altrimenti intervallo. Controllo: `FileInfo -v` e lettura XML diretta.

### mzXML
- **Stato**: L (32/64 bit, zlib, indice), solo V7: nessun mzML della stessa corsa nel corpus.
- **Manca**: V2 (MetaboLights ha studi con mzXML e il nativo nello stesso deposito: 11,9 % degli studi); `msInstrument` multipli; `parentFile` con SHA-1.
- **Prove**: V2 (nativo dello stesso deposito convertito da msconvert nel workflow `oracoli.yml`), V4 (pyteomics `mzxml`).
- **Lavoro**: 1 giorno (R-08). **Rischio**: basso. **Priorità**: 3.

### mzData
- **Stato**: non letto (1,2 % degli studi MetaboLights, quasi tutti prima del 2012).
- **Raccomandazione**: lettore piccolo nel modulo nuovo `storici` (XML semplice, base64, nessuna compressione), solo se R-08 ha spazio; prova V4 con OpenMS `FileConverter`/`FileInfo` (legge mzData).
- **Lavoro**: 2 giorni. **Rischio**: basso. **Priorità**: 4.

### MGF e MSP
- **Stato**: letti come librerie (`mzlab-lib`, bersaglio fuzz `mzlib_msp_mgf`), non come corsa. Un MGF di una corsa DDA (1,1 % degli studi) non ha MS1 né TIC.
- **Raccomandazione**: aprire un MGF come «elenco di MS2» nel Banco HR, non come corsa; nessun cromatogramma inventato. Decisione di Federico (R-08 la propone).

### imzML
- **Fuori da Rosina** (0,8 % degli studi; una vista diversa). R-08 scrive solo una nota: struttura (`.imzML` + `.ibd`, UUID, modo continuo/processato), cosa servirebbe.

### LECO `.peg`, JEOL, PerkinElmer, altri
- LECO ChromaTOF (2,7 % degli studi MetaboLights, 2,3 % di Workbench, quasi tutti GC×GC-TOF): formato chiuso, nessun lettore libero, nessuna DLL in msconvert. **Guida**: esportare netCDF da ChromaTOF (lo legge `andi`).
- JEOL, PerkinElmer, AB SCIEX `.t2d` (MALDI 4800), Thermo `.dat` di ISQ vecchi: nessuna quota misurabile nei depositi (< 0,1 %). **Guida**: esportare mzML o netCDF dal software dello strumento.
