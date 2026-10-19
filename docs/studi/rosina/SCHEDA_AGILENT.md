# Rosina C: Agilent `.D` (MassHunter GC/MS e LC/MS)

Legenda e indice: `SCHEDE.md`. Lettori: `crates/mzlab-core/src/agilent/` (GC/MS, motore) e `agilent_ext/` (LC/MS, modulo `agilent`, feature `estesi`); dettagli in `docs/agenti/agilent.md`, `agilent-lcms.md`.

## Stato
- GC/MS a singolo quadrupolo (5977, EI, scansione e SIM): L, V2 contro msconvert sulle tre corse di riferimento; `data.ms` ChemStation letto.
- LC/MS: QqQ MRM/dMRM, Q-TOF profilo e centroidi (taratura per scansione da `MSMassCal.bin`) P: V2 su ST003656, ST004061, ST004922. Q-TOF 284 B e 6546: P (corpus, senza riferimento).
- DAD (`DAD1.cd/.sd/.sp`), tracce di pompa/forno/campionatore: non decodificati (dimensione e SHA-1 in `campi`). IM-QTOF 6560 (`IMFrame*`): rifiutato.
- Quota: 15 % degli studi MS MetaboLights, 24 % delle analisi Workbench (secondo produttore), 0,5 % di PRIDE.

## Cosa manca
1. V1 automatico con msconvert (MHDAC) su ogni tipo (GC/MS SIM, QqQ, Q-TOF profilo, centroidi, 6560).
2. DAD e tracce dei moduli: formato `.cd`/`.cg` (OpenLab/ChemStation «registro») ignoto; rainbow (Python) legge i `.ch`/`.uv` ChemStation, non i `.cd` MassHunter: da provare (V4).
3. Valori effettivi dello strumento per scansione (gas, temperature, tensioni: `MSActualDefs.xml` + record) non in `metodo/1`: ? (R-04).
4. GC: forno, iniettore e colonna solo in `campi` e testo (nessuna sezione `gc` in `metodo/1`): proposta di sezione in R-04 (schema esteso, decisione del Coordinatore).
5. 6560: rifiuto onesto o somma sulla deriva dichiarata (come Waters): decisione in R-04 con MTBLS427 (ELENCO-1).

## Prove possibili
- V1: msconvert in Docker Wine (MHDAC incluso in ProteoWizard). V2: Workbench con mzML degli autori (ST003656, ST004061, ST004922, ST004286 in ELENCO-1). V3: MassHunter Qualitative se disponibile. V4: rainbow (DAD ChemStation), openaraw (spia).

## Metadati campo per campo
| Campo del produttore | Dove nel file | `metodo/1` | Stato | Controllo (V3 · oracolo) |
|---|---|---|---|---|
| Moduli: nome, modello, seriale, firmware, tipo | `AcqData/Devices.xml` | `strumento.*`, `strumento.moduli[]` | L | MassHunter «Instrument Info» · mzML `instrumentConfiguration` |
| Programma e versione di acquisizione, data, tecnica (GC/LC) | `AcqData/Contents.xml` | `strumento.software`, `origine.file_creato` | L | MassHunter File Info · mzML `softwareList`, `startTimeStamp` |
| Sample name, vial/position, injection volume, method file, dilution, sample type, comment | `sequence.xml`, `AcqData/sample_info.xml` | `campione.*` | L | MassHunter «Sample Information» · mzML `sampleList` (solo nome) |
| Operator (`SequenceOperator`, `OperatorName`), computer | `sequence.xml`, `sample_info.xml`, registro di audit | `operatore.*` (identita) | L | MassHunter |
| Metodo GC/MS (tune EI, intervallo di massa, gruppi SIM con dwell, soglia, solvent delay) | `acqmeth.txt` (UTF-16) | `ms.sorgente`, `ms.periodi[0].esperimenti[]`, `grezzo` | L | MassHunter «Method» · mzML (filtri SIM) |
| Forno GC, iniettore, colonna, gas di trasporto, flusso | `acqmeth.txt` | `campi`, `grezzo` | C (nessuna sezione) | MassHunter «Method» |
| Metodo LC/MS (pompa, gradiente, forno, campionatore, rivelatore) | `AcqData/AcqMethod.xml` (`RCDevicesXml`) | `lc`, `campionatore`, `rivelatori` | L | MassHunter «Method» |
| Metodo MS (sorgente, gas, temperature, Vcap, frammentatore, skimmer, segmenti di tempo) | `AcqMethod.xml` (`SCICDevicesXml`) | `ms.sorgente.parametri`, `ms.periodi[]` | L | MassHunter «Method» |
| Transizioni MRM/dMRM (Q1, Q3, CE, frammentatore, dwell, RT e finestra, polarità, nome) | `AcqMethod.xml` | `transizioni[]` | L | MassHunter · mzML `SRM SIC` |
| Liste di riferimento (lock) e di precursori preferiti/esclusi del Q-TOF | `AcqMethod.xml` | `ms.calibrazione`, `inclusione[]` | L | MassHunter |
| Segmenti di tempo, numero di scansioni | `AcqData/MSTS.xml` | `ms.periodi[]` | L | MassHunter |
| Per scansione: tipo, livello, polarità, ionizzazione, `MzOfInterest`, CE, frammentatore, TIC, BPI | `MSScan.bin` (schema `MSScan.xsd`) | scansioni | L | mzML |
| Taratura per scansione (A, T0, polinomio) e di riserva | `MSMassCal.bin`, `DefaultMassCal.xml` | `campi` | L (usata per gli m/z) | implicita in F3 |
| Valori effettivi per scansione (gas, temperature, tensioni) | `MSActualDefs.xml` + record | — | ? (R-04) | MassHunter «Actuals» · mzML? (no) |
| DAD: spettri e λ singole; tracce di pompa (pressione), forno, campionatore | `DAD1.cd/.cg/.sd/.sp`, altri `.cd` | `campi` (byte, SHA-1), `avvisi` | N (R-04) | MassHunter «DAD»/«Instrument Curves» · msconvert? (no) · rainbow (V4, se legge `.cd`) |
| Metodo di analisi dati `*.M/`, quantificazione `DaMethod/Quant` | sotto `AcqData/` e `.m` fuori corsa | `campi`, `quantificazione` (solo `.m`) | C/L | MassHunter Quant |
| Registro di audit | `AcqData/*Audit*` | `campi` | C | MassHunter Audit Trail |
| Intestazione di `MSScan.bin`, coda di `MSTree2.bin`, tabella dopo i record di `data.ms` | binari | `avvisi`, `campi` (byte, SHA-1) | N dichiarato (copie e derivati, nessun dato in più) | — |
| Mobilità 6560 (`IMFrame*`, `IMBin`) | `AcqData/IMS*` | — | N, rifiutato | MassHunter IM-MS Browser · msconvert (combina o divide) |

## Lavoro, rischio, priorità
- **R-04**: 6-8 giorni. Verità msconvert per ogni tipo (LOC-2), Q-TOF 284 B/6546, valori effettivi, DAD se il layout si ricava (V1 non c'è: serve export di MassHunter o rainbow), sezione GC proposta, decisione sul 6560.
- **Rischio**: medio (molti file laterali, versioni B.0x-12). **Priorità**: 1 (secondo produttore in Workbench, primo nel GC/MS dei corsi).
- Tetto del modulo `agilent`: 438 272 B, misura 424 789 B (13 KB liberi): DAD e 6560 vanno in un modulo nuovo `agilent-uv` se non entrano.
