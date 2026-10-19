# Rosina C: Bruker timsTOF (TDF/TSF) e BAF

Legenda e indice: `SCHEDE.md`. Lettore: modulo `bruker` (`crates/mzlab-lettori/src/bruker/`: SQLite minimo, zstd, LZF); dettagli in `docs/agenti/bruker.md`.

## Stato
- TDF (timsTOF Pro/HT/SCP/flex, DDA-PASEF, diaPASEF, PRM, MRM): m/z L con V1 su compressione 1 (0,07 ppm contro msconvert `combineIonMobilitySpectra`), compressione 2 solo V7 (`LargestPeakMz` e lettura senza errori). WAL letto.
- **1/K0 non calcolata** (la formula di `TimsCalibration` `ModelType` 2 non si ricava dai file): oggi intervalli di scansione.
- TSF (timsTOF senza TIMS, MALDI/LC): blocchi letti, nessun riferimento.
- BAF (impact, maXis, compact, solariX, amaZon) e YEP: non letti.
- Quota: Bruker 7 % degli studi MS MetaboLights, 3 % di Workbench, 8 % di PRIDE (5,5 % timsTOF).

## Cosa manca
1. V1 con `timsdata` (SDK Bruker gratuito, Linux/macOS, ridistribuibile con condizioni) per 1/K0 e per gli spettri della compressione 2 (PXD037766, PXD058337 in ELENCO-1).
2. 1/K0: due strade. (a) Tabelle di riferimento: per ogni file del corpus `timsdata` dà 1/K0 per scansione; se un modello chiuso (polinomio sui parametri di `TimsCalibration`) le riproduce a 1e-5, si scrive. (b) Se non riesce, 1/K0 solo nell'app desktop con `timsdata` come libreria (decisione di Federico: peso e licenza).
3. TSF: V1 con `timsdata` (stessa libreria) o msconvert.
4. Metadati delle tabelle oltre `GlobalMetadata`/`Properties` (segmenti, errori, `Precursors` completi nel `metodo/1`) da elencare.

## Prove possibili
- V1: `timsdata` (Python `timsdata.py` del pacchetto SDK) e msconvert. V4: timsrust (Rust, MIT/Apache: 1/K0 con la stessa approssimazione?), alphatims (Apache, usa la formula lineare di `TimsCalibration`): concordanza come spia, non come prova.
- V3: DataAnalysis/Compass se disponibile.

## Metadati campo per campo (TDF/TSF)
| Campo del produttore | Dove nel file | `metodo/1` | Stato | Controllo (V3 · oracolo) |
|---|---|---|---|---|
| InstrumentName, InstrumentFamily, InstrumentRevision, InstrumentSerialNumber, InstrumentVendor | `GlobalMetadata` | `strumento.*` | L | DataAnalysis · `timsdata`/SQL · mzML `instrumentConfiguration` |
| AcquisitionSoftware, AcquisitionSoftwareVersion, AcquisitionFirmwareVersion | `GlobalMetadata` | `strumento.software`, `campi` | L | mzML `softwareList` |
| AcquisitionDateTime, OperatorName, SampleName, MethodName, Description | `GlobalMetadata` | `origine.file_creato`, `operatore.utente` (identita), `campione.nome`, `campi` | L | SQL diretto (V2 pulito: SQLite è aperto) |
| TimsCompressionType, MaxNumPeaksPerScan, DigitizerNumSamples, MzAcqRangeLower/Upper, OneOverK0AcqRangeLower/Upper | `GlobalMetadata` | `campi` | L | SQL |
| Frames: Time, Polarity, ScanMode, MsMsType, TIC, MaxIntensity, NumScans, AccumulationTime, RampTime, T1, T2, Pressure, Denoised | `Frames` | scansioni, `campi` (intervalli) | L (T1/T2/pressione come serie: ? R-06) | SQL; `timsdata` |
| Proprietà per frame (tensioni, temperature, gas, tutte le `PropertyDefinitions`) | `Properties`, `GroupProperties`, `PropertyDefinitions` | `campi` | L | SQL; DataAnalysis «Acquisition Parameters» |
| Precursori PASEF: Mz, Charge, ScanNumBegin/End, CollisionEnergy, IsolationMz/Width | `Precursors`, `PasefFrameMsMsInfo` | precursori delle MS2 | L | `timsdata` · mzML `precursor` |
| Finestre diaPASEF e PRM | `DiaFrameMsMsWindows`, `DiaFrameMsMsInfo`, `PrmTargets`, `PrmFrameMsMsInfo` | esperimenti, `isolamento` | L | SQL · mzML |
| Calibrazione m/z (ModelType, coefficienti, temperature di riferimento, correzione per frame T1/T2) | `MzCalibration` + `Frames` | `campi`; applicata agli m/z | L | `timsdata` `indexToMz` (F3) |
| Calibrazione di mobilità (ModelType 2, C0-C9) | `TimsCalibration` | `campi`; 1/K0 N | N per 1/K0 (R-06) | `timsdata` `scanNumToOneOverK0` |
| Segmenti, errori, messaggi di acquisizione | `Segments`, `ErrorLog` (se presenti) | — | ? (R-06) | SQL |
| Metodo completo (`*.m/` dentro la cartella: `microTOFQImpacTemAcquisition.method`, LC) | `<x>.d/<metodo>.m/` (XML) | `grezzo`? | ? (R-06: elencare) | DataAnalysis «Method» |
| HyStar: campione, vial, LC | `SampleInfo.xml`, `HyStar*.xml` (se presenti) | `campione`, `lc` | ? (R-06) | HyStar |
| TSF: spettri in linee o profilo, Frames | `analysis.tsf` + `.tsf_bin` | spettri | L senza riferimento | `timsdata` `tsf` |

## BAF (impact, maXis, compact, solariX, amaZon) e YEP
- `analysis.baf` è un formato binario chiuso con indice in `analysis.baf_idx` e, nelle versioni recenti, un SQLite (`*.sqlite` creato da `baf2sql`); nessuna specifica, nessun lettore libero affidabile.
- **Decisione onesta**: nessun lettore nel sito né nell'app desktop. Guida: esportare mzML da DataAnalysis (`Export → mzML`) o convertire con msconvert (Windows). `baf2sql` è un SDK nativo Bruker: non si porta nel sito (peso e licenza). Si riapre solo se Bruker pubblica una specifica.
- **Rifiuto**: la riga `bruker` di `lettori-famiglie.js` riconosce `analysis.baf` e deve dare la chiave che rimanda alla guida (R-06 lo verifica).

## Lavoro, rischio, priorità
- **R-06**: 5-7 giorni. Verità `timsdata` (LOC-2), compressione 2 a F3, 1/K0 strada (a) con criterio d'uscita (3 giorni senza 1e-5: strada (b) come proposta a Federico), TSF, metadati del metodo incorporato, BAF come guida e rifiuto.
- **Rischio**: alto su 1/K0 (modello non pubblico), basso sul resto. **Priorità**: 2.
- Tetto del modulo `bruker`: 434 176 B, misura 421 370 B (12 KB liberi).
