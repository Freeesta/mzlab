# Rosina C: Waters MassLynx `.raw` (folder)

Legend and index: `SHEETS.md`. Reader: module `waters` (`crates/mzlab-lettori/src/waters/`); details in `docs/agenti/waters.md`.

## Status
- Targeted functions (MRM/SIR) P: chromatograms with the msconvert ids; V2 possible with Workbench ST004453 (TQ-XS), ST003942, ST003984 (ELENCO-1).
- TOF scan functions (Xevo G2/G2-S/G2-XS, Synapt), MSe, DDA: P (13 real folders, no reference). `Cal Function` calibration applied; **lock mass not applied** (30-100 ppm on the files with lock mass, declared).
- IMS (HDMSe, SONAR, cyclic): summed over the drift, warning declared. 6-byte encoding and MALDI functions: no file.
- Share: 12% of the MetaboLights MS studies, 5% of Workbench, 2% of PRIDE.

## What is missing
1. V1: msconvert (Waters DLL `MassLynxRaw`) in Docker; MassLynx Raw SDK (has a Linux reader, Waters license) for lock mass and drift as a local oracle, never in the site.
2. Lock mass: declared option "correction with the reference function" (decision 4 of `ROSINA.md`), proven against the SDK.
3. IMS drift: drift time per bin (msconvert splits into one scan per bin): expose as mobility or stay summed with a warning (decision in R-05).
4. `.STS`/`.EE` beyond "Set Mass" and "Collision Energy", `_HISTORY.INF`, ramp parameters: not decoded.

## Possible checks
- V1: msconvert in Docker; MassLynx SDK (Linux) for corrected m/z and drift. V2: Workbench (the authors' mzML). V3: MassLynx if available. V4: openwraw (indicator), rainbow (Waters `.raw` UV and MS: to be tried).

## Metadata field by field
| Vendor field | Where in the file | `metodo/1` | Status | Check (V3 · oracle) |
|---|---|---|---|---|
| Instrument (model#serial) | `_HEADER.TXT` `$$ Instrument:` | `strumento.modello`, `.seriale` | L | MassLynx "Header" · mzML `instrumentConfiguration` |
| Acquired Name, Date, Time | `_HEADER.TXT` | `origine.file_creato`, `campi` | L | MassLynx · mzML `startTimeStamp` |
| Sample Description, Bottle Number, Sample ID, Job Code, Task, Conditions, Plate Desc | `_HEADER.TXT` | `campione.nome`, `.vial`, `.id`, `campi` | L | MassLynx "Sample List" |
| User Name, Submitter, Laboratory Name | `_HEADER.TXT` | `operatore.*` (identita) | L | MassLynx |
| Software (MassLynx version), Instrument Type, Encoding | `_HEADER.TXT` | `strumento.software`, `campi` | L | MassLynx · mzML `softwareList` |
| `Cal Function N` (polynomial, T0/T1), Cal MS1/MS2 Static/Dynamic, temperature correction | `_HEADER.TXT` | `campi` (applied to the m/z values) | L | implicit in F3; MassLynx "Calibration" |
| Functions: type, subtype, cycle time, `tof_depth`, dwell, Q1/Q3 or range ×32 | `_FUNCTNS.INF` (416 B record) | `ms.periodi[0].esperimenti[]`, `transizioni[]` | L | MassLynx "Acquisition Method" · mzML (functions as `function=N`) |
| Source: capillary, cone, temperatures, desolvation/cone gas, mode, polarity, ramp or CE per function | `_extern.inf` | `ms.sorgente.parametri`, experiments | L | MassLynx "Tune Page" (text) |
| LC gradient, flow, oven, duration, sampler | `_INLET.INF` | `lc`, `campionatore` | L | MassLynx "Inlet Method" |
| Analog traces (pressure, flow, % solvents, UV, temperatures) | `_CHROMS.INF` + `_CHROnnn.DAT` | analog chromatograms, `campi` | L | MassLynx "Analog" · mzML? (msconvert no: V3) |
| Channel compound names | `_FUNCnnn.CMP` | `transizioni[].nome` | L | MassLynx |
| Per-scan statistics: Set Mass, Collision Energy | `_FUNCnnn.STS`/`.EE` | precursor, CE | L | mzML `precursor` |
| Other `.STS` columns (pressures, per-scan voltages, LteffMulti, TOF temperature) | `_FUNCnnn.STS` | — | N (R-05) | MassLynx "Scan Statistics"? · SDK `GetScanItems` |
| Lock mass: reference function, m/z, window, frequency | `_FUNCTNS.INF` (reference function), `_extern.inf` | `ms.calibrazione.lock_mass` ("not applied") | L description; correction N | MassLynx "Lock Mass" · SDK `LockMass` |
| IMS drift: bin, time per bin, wave (velocity, height) | `_FUNCnnn.DAT` (per-bin blocks), `_extern.inf` | — (`avvisi`) | N (summed) | msconvert (one scan per bin) · SDK |
| Processing history | `_HISTORY.INF` | — | N | MassLynx |

## Work, risk, priority
- **R-05**: 6-8 days. msconvert and Workbench truth (LOC-2), TOF and MSe at F3, lock mass as an option, drift as a decision, complete `.STS` if the SDK confirms it.
- **Risk**: medium (instrument corrections not written into the data; the SDK license allows use as a reader, not redistribution: it stays on the Mac). **Priority**: 2.
- Cap of the `waters` module: 303,104 B, measured 293,150 B (10 KB free).
