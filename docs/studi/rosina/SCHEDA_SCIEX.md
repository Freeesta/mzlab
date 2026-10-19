# Rosina C: SCIEX `.wiff`, `.wiff` TOF, `.wiff2`

Legenda e indice: `SCHEDE.md`. Lettore: `crates/mzlab-core/src/wiff/` (motore e modulo `sciex`), `metodo_wiff.rs`; dettagli in `docs/agenti/wiff.md`, `wiff-varianti.md`, `wiff_specifica_completa.md`.

## Stato
- `.wiff` + `.wiff.scan` di Analyst (3200 QTRAP del laboratorio, QTRAP 4000/5500/6500): L per full scan, MRM, sMRM, EPI, PIS, NL; 32 file veri, 388 campioni letti, 6 rifiutati (TOF e `.wiff2`). Più campioni per contenitore L. File troncato: si carica con le scansioni che ha (`mancanti`).
- Rifiutati con chiave onesta: periodi multipli (`periods=N`), blocchi TOF, codici di scansione ignoti, versione 4.
- DAD decodificato, non validato. Tracce analogiche dei dispositivi (`DevData`), criteri IDA (`DDERealTimeData`), configurazione MS e tabella di taratura: non decodificati.
- Quota: SCIEX 23 % degli studi MS MetaboLights, 14 % di Workbench (primo produttore nei QqQ di Workbench: 628 analisi su 1 214), 5 % dei progetti PRIDE (quasi tutti TripleTOF/ZenoTOF).

## Cosa manca
1. V1 su ogni esperimento: msconvert (DLL SCIEX) nel Docker Wine. V2 già per PIS/NL (Zenodo 7897576, MTBLS1381) e per il 3200.
2. Periodi multipli, MS3, EMC/ER, scansioni Q1/Q3: nessun file vero. Fonte probabile: Workbench (QTRAP con metodi lipidomici a più periodi) e la chat locale con l'Analyst del laboratorio (un file costruito apposta: decisione di Federico).
3. TripleTOF/ZenoTOF/X500: codec dei blocchi TOF ignoto; servono `.wiff` + `.wiff.scan` + mzML di riferimento (MTBLS13023, Zenodo 10889321 in ELENCO-1).
4. Polarità mista: Zenodo 22673685 (ELENCO-1).
5. `campione.vial` da confermare; volume d'iniezione, posizione nel rack/piastra, tipo di campione, nome del batch: in `SampleTable` (`campi` con `estesi`), non nelle sezioni.

## Prove possibili
- V1: msconvert (DLL SCIEX `Clearcore2`) in Docker Wine; per il `.wiff2` anche SCIEX MS Data Converter (Windows, gratuito).
- V2: mzML degli autori (Zenodo 7897576, MTBLS1381, fixture di ProteoWizard nel privato).
- V3: **Analyst del 3200 al laboratorio** (Federico): «File Information», «Show Experiment Info», XIC di una transizione con area. È la prova più forte per i metadati che msconvert non scrive (sorgente, LC, campionatore).
- V4: opensxraw (dichiarato inaffidabile dai suoi autori): solo come spia.

## Metadati campo per campo (`.wiff`)
| Campo del produttore | Dove nel file | `metodo/1` | Stato | Controllo (V3 · oracolo) |
|---|---|---|---|---|
| Instrument model, serial number | Log del campione, record «Config Table» | `strumento.modello`, `.seriale` | L | Analyst «File Information» · mzML `instrumentConfiguration` (modello, `MS:1000529`) |
| Analyst/SCIEX OS version | `SampleDABE/CFR_INFO`, `GLPTables` | `strumento.software` | L (confermato: 1.6.2, 1.6.3) | Analyst «About»/File Info · mzML `softwareList` |
| Creation date | `SummaryInformation` proprietà 12 (FILETIME) | `origine.file_creato` | L | mzML `run@startTimeStamp` |
| Acquisition start/end | Log | `campi` | L | Analyst File Info |
| User, computer | Log `driver:User=` | `operatore.*` (identita) | L | Analyst File Info |
| Sample name | `SampleDABE/DATA` prima stringa | `campione.nome` | L (confermato) | nome dell'mzML `<wiff>-<nome>.mzML`; `sampleList` |
| Vial/plate position | `DATA` ultima stringa | `campione.vial` | ? (R-03: V3 su Analyst) | Analyst «Sample Information» |
| Sample ID, type, comment, dilution, injection volume, rack, plate, batch | `SampleTable` | `campi` (`estesi`) | C (R-03: portare nelle sezioni) | Analyst Batch Editor |
| Audit trail (CFR) | `GLPTables` | `campi` (`estesi`) | C | Analyst Audit Trail |
| Scan type per esperimento (full, MRM, EPI, PIS, NL, MS3, ER, EMC, TOF) | `ExperimentHeader` +122 | `ms.periodi[].esperimenti[].tipo` | L (4, 5, 7, 11, 15); N per MS3/ER/EMC (nessun file) | Analyst «Acquisition Method» · mzML `MS:1000580`/filtri |
| Polarity | `ExperimentHeader` u32 +86 | esperimenti, scansioni | L (40 file) | mzML `MS:1000130/129` |
| Precursor/fixed mass | `ExperimentHeader` f64 +42 | esperimenti, `precursore_mz` | L (EPI: massa del metodo, ? con IDA) | mzML `precursor` |
| Mass ranges, step, scan time, dwell | `ExperimentHeader` e tabelle delle masse | `parametri`, `transizioni[].dwell_ms` | L | Analyst · mzML `scanWindow`, `dwell` (userParam) |
| Q1, Q3, CE, DP, EP, CXP per transizione; nome del composto | tabella delle masse dell'esperimento | `transizioni[]` | L (Q1/Q3/CE/nome); ? DP/EP/CXP (R-03) | Analyst · mzML `SRM SIC` con `MS:1000045` |
| Scheduled MRM: RT atteso, finestra, target scan time | parametri sMRM | `programmato`, `finestra_s` (`estesi`) | L | Analyst · msconvert non lo scrive: V3 |
| Source/gas parameters (CUR, IS, TEM, GS1, GS2, CAD, ihe) per periodo | parametri del periodo | `ms.sorgente.parametri` | L | Analyst «Acquisition Method» (msconvert non lo scrive: V3) |
| Source type | Log «Source Type» | `ms.sorgente.tipo` | L | Analyst |
| Periodi multipli (durata, cicli) | `Period1…` | — | N, rifiutato (nessun file) | Analyst |
| Criteri IDA (soglie, esclusione, numero di candidati) | `MassSpecMethod*`, `DDERealTimeData` | — | N (R-03) | Analyst «IDA Criteria» · mzML (precursori delle scansioni dipendenti) |
| Metodo LC (pompe, gradiente, forno) e autocampionatore | metodo dei dispositivi (`DeviceMethod`) | `lc`, `campionatore` | L (pompe e campionatore dal driver: 3200); ? altri moduli (R-03) | Analyst «Acquisition Method» |
| Tracce dei dispositivi (pressione, temperatura del forno, ADC) | `Devices/Device_N/DevData` | — | N (R-03) | Analyst «Show ADC Data»/«Show Instrument Data» · mzML? (msconvert scrive alcune tracce come cromatogrammi: da vedere in LOC-2) |
| DAD (matrice tempo × λ, TWC, λ singole) | flussi DAD | cromatogrammi `Pda` (mAU) | L non validato (R-03: V3 con export di Analyst) | Analyst «Show DAD Data» |
| Configurazione MS, tabella di taratura di massa, risoluzione | `MSConfigInfo*` | — | N | Analyst «Tune»; nessun oracolo automatico |
| Stato dello strumento (vuoto, temperature) durante la corsa | Log / `DevData` | `grezzo` | C (solo testo) | Analyst «Instrument Data» |

## `.wiff` TOF (TripleTOF, ZenoTOF, X500)
- Stesso contenitore OLE e stesso metodo; i blocchi del `.wiff.scan` usano un altro codec (TOF a canali, intensità compresse). Metadati del metodo (`ExperimentHeader`, codici 8/9) già decodificati: il pezzo mancante è solo lo spettro.
- **Lavoro**: 6-10 giorni in un modulo nuovo `sciex-tof` (tetto = misura + 3 %), solo con V1 (msconvert in Docker o mzML degli autori di MTBLS13023/Zenodo 10889321). **Rischio**: alto (codec ignoto; possibile che serva la tabella di taratura di `MSConfigInfo` per gli m/z). **Priorità**: 2.
- Regola d'uscita: se dopo 3 giorni non c'è un m/z entro 5 ppm su un file con riferimento, la chat si ferma, scrive ciò che ha trovato in `wiff_specifica_completa.md` e il formato resta «rifiutato con guida».

## `.wiff2` (SCIEX OS)
- Contenitore diverso (SQLite + flussi compressi, versione dichiarata), nessuna specifica pubblica, nessun lettore libero; SCIEX OS è il solo scrittore dei nuovi strumenti (7500, ZenoTOF 7600).
- **Decisione onesta**: nessun lettore. Guida per lo studente: conversione con msconvert (Windows) o con SCIEX MS Data Converter (gratuito, `-profile`/`-centroid`), poi mzML in mzLab. Lettera a SCIEX (Federico) per una specifica a uso didattico; si riapre solo con una specifica o un SDK ridistribuibile.
- **Rifiuto nel programma**: `.wiff2` riconosciuto e rifiutato con la chiave che rimanda alla guida (già così).

## Lavoro, rischio, priorità (`.wiff` QTRAP)
- **R-03**: 6-8 giorni. Verità msconvert su tutti i `.wiff` del corpus (LOC-2), polarità mista, `vial` e `SampleTable` nelle sezioni, DP/EP/CXP, metodo LC degli altri moduli, `DevData` (tracce) e IDA se il layout si ricava con V1, DAD contro export Analyst (V3), periodi multipli se un file arriva. TOF come sopra.
- **Rischio**: medio (layout ricavati per osservazione; legge UE 2009/24/CE art. 5.3). **Priorità**: 1 (lo strumento del laboratorio).
- Tetto del modulo `sciex`: 327 680 B (non si alza), misura 319 930 B: il codice nuovo oltre ~7 KB va in `sciex-tof` o in un modulo `sciex-ext`.
