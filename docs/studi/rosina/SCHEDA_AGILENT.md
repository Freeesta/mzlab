# Rosina C: Agilent `.D` (MassHunter GC/MS and LC/MS)

Legend and index: `SCHEDE.md`. Readers: `crates/mzlab-core/src/agilent/` (GC/MS, engine) and `agilent_ext/` (LC/MS, module `agilent`, feature `estesi`); details in `docs/agenti/agilent.md`, `agilent-lcms.md`.

## Status
- Single-quadrupole GC/MS (5977, EI, scan and SIM): L, V2 against msconvert on the three reference runs; ChemStation `data.ms` read.
- LC/MS: QqQ MRM/dMRM, Q-TOF profile and centroids (per-scan calibration from `MSMassCal.bin`) P: V2 on ST003656, ST004061, ST004922. Q-TOF 284 B and 6546: P (corpus, no reference).
- DAD (`DAD1.cd/.sd/.sp`), pump/oven/sampler traces: not decoded (size and SHA-1 in `campi`). IM-QTOF 6560 (`IMFrame*`): rejected.
- Share: 15% of the MetaboLights MS studies, 24% of the Workbench analyses (second vendor), 0.5% of PRIDE.

## What is missing
1. Automatic V1 with msconvert (MHDAC) on every type (GC/MS SIM, QqQ, Q-TOF profile, centroids, 6560).
2. DAD and module traces: the `.cd`/`.cg` format (OpenLab/ChemStation "registry") is unknown; rainbow (Python) reads ChemStation `.ch`/`.uv`, not MassHunter `.cd`: to be tried (V4).
3. Actual instrument values per scan (gas, temperatures, voltages: `MSActualDefs.xml` + records) are not in `metodo/1`: ? (R-04).
4. GC: oven, injector and column only in `campi` and text (no `gc` section in `metodo/1`): section proposal in R-04 (extended schema, Coordinator's decision).
5. 6560: honest rejection or sum over the declared drift (as with Waters): decision in R-04 with MTBLS427 (ELENCO-1).

## Possible checks
- V1: msconvert in Docker Wine (MHDAC is included in ProteoWizard). V2: Workbench with the authors' mzML (ST003656, ST004061, ST004922, ST004286 in ELENCO-1). V3: MassHunter Qualitative if available. V4: rainbow (ChemStation DAD), openaraw (indicator).

## Metadata field by field
| Vendor field | Where in the file | `metodo/1` | Status | Check (V3 · oracle) |
|---|---|---|---|---|
| Modules: name, model, serial, firmware, type | `AcqData/Devices.xml` | `strumento.*`, `strumento.moduli[]` | L | MassHunter "Instrument Info" · mzML `instrumentConfiguration` |
| Acquisition program and version, date, technique (GC/LC) | `AcqData/Contents.xml` | `strumento.software`, `origine.file_creato` | L | MassHunter File Info · mzML `softwareList`, `startTimeStamp` |
| Sample name, vial/position, injection volume, method file, dilution, sample type, comment | `sequence.xml`, `AcqData/sample_info.xml` | `campione.*` | L | MassHunter "Sample Information" · mzML `sampleList` (name only) |
| Operator (`SequenceOperator`, `OperatorName`), computer | `sequence.xml`, `sample_info.xml`, audit log | `operatore.*` (identita) | L | MassHunter |
| GC/MS method (EI tune, mass range, SIM groups with dwell, threshold, solvent delay) | `acqmeth.txt` (UTF-16) | `ms.sorgente`, `ms.periodi[0].esperimenti[]`, `grezzo` | L | MassHunter "Method" · mzML (SIM filters) |
| GC oven, injector, column, carrier gas, flow | `acqmeth.txt` | `campi`, `grezzo` | C (no section) | MassHunter "Method" |
| LC/MS method (pump, gradient, oven, sampler, detector) | `AcqData/AcqMethod.xml` (`RCDevicesXml`) | `lc`, `campionatore`, `rivelatori` | L | MassHunter "Method" |
| MS method (source, gas, temperatures, Vcap, fragmentor, skimmer, time segments) | `AcqMethod.xml` (`SCICDevicesXml`) | `ms.sorgente.parametri`, `ms.periodi[]` | L | MassHunter "Method" |
| MRM/dMRM transitions (Q1, Q3, CE, fragmentor, dwell, RT and window, polarity, name) | `AcqMethod.xml` | `transizioni[]` | L | MassHunter · mzML `SRM SIC` |
| Q-TOF reference (lock) lists and preferred/excluded precursor lists | `AcqMethod.xml` | `ms.calibrazione`, `inclusione[]` | L | MassHunter |
| Time segments, number of scans | `AcqData/MSTS.xml` | `ms.periodi[]` | L | MassHunter |
| Per scan: type, level, polarity, ionization, `MzOfInterest`, CE, fragmentor, TIC, BPI | `MSScan.bin` (schema `MSScan.xsd`) | scans | L | mzML |
| Per-scan calibration (A, T0, polynomial) and fallback | `MSMassCal.bin`, `DefaultMassCal.xml` | `campi` | L (used for the m/z values) | implicit in F3 |
| Actual values per scan (gas, temperatures, voltages) | `MSActualDefs.xml` + records | — | ? (R-04) | MassHunter "Actuals" · mzML? (no) |
| DAD: spectra and single λ; pump (pressure), oven, sampler traces | `DAD1.cd/.cg/.sd/.sp`, other `.cd` | `campi` (bytes, SHA-1), `avvisi` | N (R-04) | MassHunter "DAD"/"Instrument Curves" · msconvert? (no) · rainbow (V4, if it reads `.cd`) |
| Data analysis method `*.M/`, quantitation `DaMethod/Quant` | under `AcqData/` and `.m` outside the run | `campi`, `quantificazione` (`.m` only) | C/L | MassHunter Quant |
| Audit log | `AcqData/*Audit*` | `campi` | C | MassHunter Audit Trail |
| Header of `MSScan.bin`, tail of `MSTree2.bin`, table after the `data.ms` records | binaries | `avvisi`, `campi` (bytes, SHA-1) | N declared (copies and derivatives, no extra data) | — |
| 6560 mobility (`IMFrame*`, `IMBin`) | `AcqData/IMS*` | — | N, rejected | MassHunter IM-MS Browser · msconvert (combines or splits) |

## Work, risk, priority
- **R-04**: 6-8 days. msconvert truth for every type (LOC-2), Q-TOF 284 B/6546, actual values, DAD if the layout can be derived (no V1: needs a MassHunter export or rainbow), proposed GC section, decision on the 6560.
- **Risk**: medium (many side files, versions B.0x-12). **Priority**: 1 (second vendor in Workbench, first in the GC/MS of the courses).
- Cap of the `agilent` module: 438,272 B, measured 424,789 B (13 KB free): DAD and 6560 go into a new `agilent-uv` module if they do not fit.
