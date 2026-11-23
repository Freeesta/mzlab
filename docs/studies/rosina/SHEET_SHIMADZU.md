# Rosina C: Shimadzu `.lcd` (LabSolutions LC/MS) and `.qgd` (GCMSsolution)

Legend and index: `SHEETS.md`. Reader: module `shimadzu` (`crates/mzlab-lettori/src/shimadzu/`: OLE container, zlib); details in `docs/agenti/shimadzu.md`.

## Status
- `.qgd` GC/MS (QP2010/QP2020): scan and SIM P; `.lcd` LC/MS QqQ (8040/8050/8060) MRM P, LCMS-2020 (single quadrupole) P. No real file with a reference.
- Not decoded: polarity of the `Mass Raw Data` events, CE and dwell of the MRMs (two transitions with the same Q1/Q3 and different CE would merge), PDA (`PDA 3D Raw Data`), GC parameters, instrument state, embedded `.qgm` method. LCMS-9030 (Q-TOF) and IT-TOF: rejected.
- Share: 1.7% of the MetaboLights MS studies, 0.7% of Workbench; but very common in Italian teaching laboratories (GC/MS).

## What is missing
1. V1: msconvert has the Shimadzu DLL only for the LC/MS `.lcd`; for `.qgd` there is no automatic oracle: V2 with a `.cdf` exported from the same file (GCMSsolution "File Conversion → AIA") or V3.
2. Files with a reference: MTBLS12691 (8060), MTBLS13204 (9030), Zenodo 18633076 (2020 + PDA), ST001609 (QP2010 Ultra) in ELENCO-1.
3. Polarity, CE, dwell, PDA, GC; 9030 in a new `shimadzu-tof` module.

## Possible checks
- V1 (`.lcd`): msconvert in Docker. V2 (`.qgd`): `.cdf` of the same run, read by `andi`. V3: LabSolutions/GCMSsolution (Postrun) if available. V4: openszraw (immature, indicator).

## Metadata field by field
| Vendor field | Where in the file | `metodo/1` | Status | Check (V3 · oracle) |
|---|---|---|---|---|
| Instrument name, configuration | `GCMS Configuration/System`, system streams | `strumento.modello` | L (as the file declares it: "GCMS-QP2010" also for QP2020NX) | Postrun "Instrument" · mzML (`.lcd`) |
| File and software version | `File Property` (offset 4, 148 in the `.qgd`) | `strumento.software`, `campi` | L | Postrun |
| generatedBy, modifiedBy, date | `File Property` (20, 52, 60, 92) | `campi` (identita) | L | Postrun "File Property" |
| Sample name, sample ID, vial, injection volume | `File Property` (204, 236) / properties XML (`.lcd`) | `campione.*` | L | Postrun "Sample Information" · mzML `sampleList` |
| Operator, acquisition date | `File Property` (300, 508) | `operatore.utente` (identita), `origine.file_creato` | L | Postrun |
| Paths of the data files, method, tune, batch | `File Property` (580, 836, 1092, 1604) | `campi` (identita, not in the mzML) | L | Postrun |
| Events: type (scan/SIM), m/z range, SIM ions, times | `Mass Raw Data`, events seen | `ms.periodi[].esperimenti[]` | L (SIM heuristic not verified) | mzML (`.lcd`); `.cdf` (`.qgd`) |
| Polarity per event | `Mass Raw Data` (event header) | scans | N (R-07) | mzML (`.lcd`) |
| MRM transitions: Q1, Q3, polarity | `TLM Raw Data` (zlib records, byte 33 bits 6-7) | `transizioni[]` | L | mzML `SRM SIC` |
| CE, dwell, pause, Q1/Q3 pre-bias | MS method (`Mass Parameters`?) | — | N (R-07) | LabSolutions "Method" · mzML? (msconvert writes CE: to be checked) |
| GC method: oven, injector, column, gas | `GC-2010/GC17A Instrument Parameters` | — (`avvisi`) | N (R-07) | GCMSsolution "Method" |
| MS tune and state (vacuum, filament, detector, temperatures) | `MS Status Data`, `<storage>/Status`, `Status Curve`, `GUMM_Information` | — (`avvisi`) | N (R-07) | Postrun "Tuning"/"Status" |
| PDA: spectra and single λ | `PDA 3D Raw Data`, `LSS Raw Data` | — (`avvisi`) | N (R-07) | LabSolutions "PDA" · msconvert? (no) |
| Embedded `.qgm` method | OLE stream | — (`avvisi`) | N | GCMSsolution |
| 9030 / IT-TOF (TOF spectra) | other streams | — | N, rejected | LabSolutions · msconvert |

## Work, risk, priority
- **R-07**: 6-8 days. msconvert (`.lcd`) and `.cdf` (`.qgd`) truth (LOC-2), polarity, CE/dwell, PDA, GC as decoded `campi`, 9030 in `shimadzu-tof` (cap = measured + 3%) only with V1.
- **Risk**: medium (many streams without a clear name, no mature free reader). **Priority**: 3 (small shares, but useful for the courses).
- Cap of the `shimadzu` module: 290,816 B, measured 281,572 B (9 KB free).
