# Rosina C: Bruker timsTOF (TDF/TSF) and BAF

Legend and index: `SCHEDE.md`. Reader: module `bruker` (`crates/mzlab-lettori/src/bruker/`: minimal SQLite, zstd, LZF); details in `docs/agenti/bruker.md`.

## Status
- TDF (timsTOF Pro/HT/SCP/flex, DDA-PASEF, diaPASEF, PRM, MRM): m/z L with V1 on compression 1 (0.07 ppm against msconvert `combineIonMobilitySpectra`), compression 2 only V7 (`LargestPeakMz` and error-free reading). WAL read.
- **1/K0 not computed** (the `TimsCalibration` `ModelType` 2 formula cannot be derived from the files): today scan ranges.
- TSF (timsTOF without TIMS, MALDI/LC): blocks read, no reference.
- BAF (impact, maXis, compact, solariX, amaZon) and YEP: not read.
- Share: Bruker 7% of the MetaboLights MS studies, 3% of Workbench, 8% of PRIDE (5.5% timsTOF).

## What is missing
1. V1 with `timsdata` (free Bruker SDK; libraries only for Linux and Windows: not available on Mac arm64, `opentimspy` gives only TOF and scan indices; it runs on Windows (Parallels) through the local chat, `ROSINA.md` decision 1; if it does not work there either, "oracle absent" for m/z and 1/K0 and the ProteoWizard fixtures with mobility remain as V2) for 1/K0 and for the compression 2 spectra (PXD037766, PXD058337 in ELENCO-1).
2. 1/K0: two routes. (a) Reference tables: for every corpus file `timsdata` gives 1/K0 per scan; if a closed-form model (polynomial on the `TimsCalibration` parameters) reproduces them to 1e-5, it gets written. (b) If it fails, 1/K0 only in the desktop app with `timsdata` as a library (the maintainer's decision: weight and license).
3. TSF: V1 with `timsdata` (same library) or msconvert.
4. Table metadata beyond `GlobalMetadata`/`Properties` (segments, errors, complete `Precursors` in `metodo/1`) to be listed.

## Possible checks
- V1: `timsdata` (Python `timsdata.py` of the SDK package) and msconvert. V4: timsrust (Rust, MIT/Apache: 1/K0 with the same approximation?), alphatims (Apache, uses the linear `TimsCalibration` formula): agreement as an indicator, not as proof.
- V3: DataAnalysis/Compass if available.

## Metadata field by field (TDF/TSF)
| Vendor field | Where in the file | `metodo/1` | Status | Check (V3 · oracle) |
|---|---|---|---|---|
| InstrumentName, InstrumentFamily, InstrumentRevision, InstrumentSerialNumber, InstrumentVendor | `GlobalMetadata` | `strumento.*` | L | DataAnalysis · `timsdata`/SQL · mzML `instrumentConfiguration` |
| AcquisitionSoftware, AcquisitionSoftwareVersion, AcquisitionFirmwareVersion | `GlobalMetadata` | `strumento.software`, `campi` | L | mzML `softwareList` |
| AcquisitionDateTime, OperatorName, SampleName, MethodName, Description | `GlobalMetadata` | `origine.file_creato`, `operatore.utente` (identita), `campione.nome`, `campi` | L | direct SQL (clean V2: SQLite is open) |
| TimsCompressionType, MaxNumPeaksPerScan, DigitizerNumSamples, MzAcqRangeLower/Upper, OneOverK0AcqRangeLower/Upper | `GlobalMetadata` | `campi` | L | SQL |
| Frames: Time, Polarity, ScanMode, MsMsType, TIC, MaxIntensity, NumScans, AccumulationTime, RampTime, T1, T2, Pressure, Denoised | `Frames` | scans, `campi` (ranges) | L (T1/T2/pressure as series: ? R-06) | SQL; `timsdata` |
| Per-frame properties (voltages, temperatures, gas, all the `PropertyDefinitions`) | `Properties`, `GroupProperties`, `PropertyDefinitions` | `campi` | L | SQL; DataAnalysis "Acquisition Parameters" |
| PASEF precursors: Mz, Charge, ScanNumBegin/End, CollisionEnergy, IsolationMz/Width | `Precursors`, `PasefFrameMsMsInfo` | MS2 precursors | L | `timsdata` · mzML `precursor` |
| diaPASEF and PRM windows | `DiaFrameMsMsWindows`, `DiaFrameMsMsInfo`, `PrmTargets`, `PrmFrameMsMsInfo` | experiments, `isolamento` | L | SQL · mzML |
| m/z calibration (ModelType, coefficients, reference temperatures, per-frame T1/T2 correction) | `MzCalibration` + `Frames` | `campi`; applied to the m/z values | L | `timsdata` `indexToMz` (F3) |
| Mobility calibration (ModelType 2, C0-C9) | `TimsCalibration` | `campi`; 1/K0 N | N for 1/K0 (R-06) | `timsdata` `scanNumToOneOverK0` |
| Segments, errors, acquisition messages | `Segments`, `ErrorLog` (if present) | — | ? (R-06) | SQL |
| Complete method (`*.m/` inside the folder: `microTOFQImpacTemAcquisition.method`, LC) | `<x>.d/<method>.m/` (XML) | `grezzo`? | ? (R-06: list) | DataAnalysis "Method" |
| HyStar: sample, vial, LC | `SampleInfo.xml`, `HyStar*.xml` (if present) | `campione`, `lc` | ? (R-06) | HyStar |
| TSF: spectra as lines or profile, Frames | `analysis.tsf` + `.tsf_bin` | spectra | L without reference | `timsdata` `tsf` |

## BAF (impact, maXis, compact, solariX, amaZon) and YEP
- `analysis.baf` is a closed binary format with an index in `analysis.baf_idx` and, in recent versions, a SQLite (`*.sqlite` created by `baf2sql`); no specification, no reliable free reader.
- **Honest decision**: no reader in the site or in the desktop app. Guide: export mzML from DataAnalysis (`Export → mzML`) or convert with msconvert (Windows). `baf2sql` is a native Bruker SDK: it cannot be ported to the site (weight and license). Reopened only if Bruker publishes a specification.
- **Rejection**: the `bruker` row of `lettori-famiglie.js` recognizes `analysis.baf` and must give the key that points to the guide (R-06 verifies it).

## Work, risk, priority
- **R-06**: 5-7 days. `timsdata` truth (LOC-2), compression 2 at F3, 1/K0 route (a) with an exit criterion (3 days without 1e-5: route (b) as a proposal to the maintainer), TSF, embedded method metadata, BAF as guide and rejection.
- **Risk**: high on 1/K0 (non-public model), low on the rest. **Priority**: 2.
- Cap of the `bruker` module: 434,176 B, measured 421,370 B (12 KB free).
