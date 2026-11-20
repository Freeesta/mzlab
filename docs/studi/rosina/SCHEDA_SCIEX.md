# Rosina C: SCIEX `.wiff`, `.wiff` TOF, `.wiff2`

Legend and index: `SCHEDE.md`. Reader: `crates/mzlab-core/src/wiff/` (engine and module `sciex`), `metodo_wiff.rs`; details in `docs/agenti/wiff.md`, `wiff-varianti.md`, `wiff_specifica_completa.md`.

## Status
- Analyst `.wiff` + `.wiff.scan` (the laboratory's 3200 QTRAP, QTRAP 4000/5500/6500): L for full scan, MRM, sMRM, EPI, PIS, NL; 32 real files, 388 samples read, 6 rejected (TOF and `.wiff2`). Multiple samples per container L. Truncated file: loads with the scans it has (`mancanti`).
- Rejected with an honest key: multiple periods (`periods=N`), TOF blocks, unknown scan codes, version 4.
- DAD decoded, not validated. Analog device traces (`DevData`), IDA criteria (`DDERealTimeData`), MS configuration and calibration table: not decoded.
- Share: SCIEX 23% of the MetaboLights MS studies, 14% of Workbench (first vendor in the Workbench QqQs: 628 analyses out of 1,214), 5% of the PRIDE projects (almost all TripleTOF/ZenoTOF).

## What is missing
1. V1 on every experiment: msconvert (SCIEX DLL) in the Docker Wine. V2 already for PIS/NL (Zenodo 7897576, MTBLS1381) and for the 3200.
2. Multiple periods, MS3, EMC/ER, Q1/Q3 scans: no real file. Likely source: Workbench (QTRAP with multi-period lipidomics methods) and the local chat with the laboratory's Analyst (a purpose-built file: the maintainer's decision).
3. TripleTOF/ZenoTOF/X500: the TOF block codec is unknown; needs `.wiff` + `.wiff.scan` + reference mzML (MTBLS13023, Zenodo 10889321 in ELENCO-1).
4. Mixed polarity: Zenodo 22673685 (ELENCO-1).
5. `campione.vial` to be confirmed; injection volume, rack/plate position, sample type, batch name: in `SampleTable` (`campi` with `estesi`), not in the sections.

## Possible checks
- V1: msconvert (SCIEX DLL `Clearcore2`) in Docker Wine; for `.wiff2` also SCIEX MS Data Converter (Windows, free).
- V2: the authors' mzML (Zenodo 7897576, MTBLS1381, ProteoWizard fixtures in the private repo).
- V3: **the laboratory's 3200 Analyst** (the maintainer): "File Information", "Show Experiment Info", XIC of a transition with area. It is the strongest proof for the metadata that msconvert does not write (source, LC, sampler).
- V4: opensxraw (declared unreliable by its authors): indicator only.

## Metadata field by field (`.wiff`)
| Vendor field | Where in the file | `metodo/1` | Status | Check (V3 · oracle) |
|---|---|---|---|---|
| Instrument model, serial number | sample log, "Config Table" record | `strumento.modello`, `.seriale` | L | Analyst "File Information" · mzML `instrumentConfiguration` (model, `MS:1000529`) |
| Analyst/SCIEX OS version | `SampleDABE/CFR_INFO`, `GLPTables` | `strumento.software` | L (confirmed: 1.6.2, 1.6.3) | Analyst "About"/File Info · mzML `softwareList` |
| Creation date | `SummaryInformation` property 12 (FILETIME) | `origine.file_creato` | L | mzML `run@startTimeStamp` |
| Acquisition start/end | log | `campi` | L | Analyst File Info |
| User, computer | log `driver:User=` | `operatore.*` (identita) | L | Analyst File Info |
| Sample name | `SampleDABE/DATA` first string | `campione.nome` | L (confirmed) | mzML name `<wiff>-<name>.mzML`; `sampleList` |
| Vial/plate position | `DATA` last string | `campione.vial` | ? (R-03: V3 on Analyst) | Analyst "Sample Information" |
| Sample ID, type, comment, dilution, injection volume, rack, plate, batch | `SampleTable` | `campi` (`estesi`) | C (R-03: bring into the sections) | Analyst Batch Editor |
| Audit trail (CFR) | `GLPTables` | `campi` (`estesi`) | C | Analyst Audit Trail |
| Scan type per experiment (full, MRM, EPI, PIS, NL, MS3, ER, EMC, TOF) | `ExperimentHeader` +122 | `ms.periodi[].esperimenti[].tipo` | L (4, 5, 7, 11, 15); N for MS3/ER/EMC (no file) | Analyst "Acquisition Method" · mzML `MS:1000580`/filters |
| Polarity | `ExperimentHeader` u32 +86 | experiments, scans | L (40 files) | mzML `MS:1000130/129` |
| Precursor/fixed mass | `ExperimentHeader` f64 +42 | experiments, `precursore_mz` | L (EPI: method mass, ? with IDA) | mzML `precursor` |
| Mass ranges, step, scan time, dwell | `ExperimentHeader` and mass tables | `parametri`, `transizioni[].dwell_ms` | L | Analyst · mzML `scanWindow`, `dwell` (userParam) |
| Q1, Q3, CE, DP, EP, CXP per transition; compound name | experiment mass table | `transizioni[]` | L (Q1/Q3/CE/name); ? DP/EP/CXP (R-03) | Analyst · mzML `SRM SIC` with `MS:1000045` |
| Scheduled MRM: expected RT, window, target scan time | sMRM parameters | `programmato`, `finestra_s` (`estesi`) | L | Analyst · msconvert does not write it: V3 |
| Source/gas parameters (CUR, IS, TEM, GS1, GS2, CAD, ihe) per period | period parameters | `ms.sorgente.parametri` | L | Analyst "Acquisition Method" (msconvert does not write it: V3) |
| Source type | log "Source Type" | `ms.sorgente.tipo` | L | Analyst |
| Multiple periods (duration, cycles) | `Period1…` | — | N, rejected (no file) | Analyst |
| IDA criteria (thresholds, exclusion, number of candidates) | `MassSpecMethod*`, `DDERealTimeData` | — | N (R-03) | Analyst "IDA Criteria" · mzML (precursors of the dependent scans) |
| LC method (pumps, gradient, oven) and autosampler | device method (`DeviceMethod`) | `lc`, `campionatore` | L (pumps and sampler from the driver: 3200); ? other modules (R-03) | Analyst "Acquisition Method" |
| Device traces (pressure, oven temperature, ADC) | `Devices/Device_N/DevData` | — | N (R-03) | Analyst "Show ADC Data"/"Show Instrument Data" · mzML? (msconvert writes some traces as chromatograms: to be checked in LOC-2) |
| DAD (time × λ matrix, TWC, single λ) | DAD streams | `Pda` chromatograms (mAU) | L not validated (R-03: V3 with an Analyst export) | Analyst "Show DAD Data" |
| MS configuration, mass calibration table, resolution | `MSConfigInfo*` | — | N | Analyst "Tune"; no automatic oracle |
| Instrument state (vacuum, temperatures) during the run | log / `DevData` | `grezzo` | C (text only) | Analyst "Instrument Data" |

## `.wiff` TOF (TripleTOF, ZenoTOF, X500)
- Same OLE container and same method; the `.wiff.scan` blocks use another codec (channel-based TOF, compressed intensities). Method metadata (`ExperimentHeader`, codes 8/9) already decoded: the only missing piece is the spectrum.
- **Work**: 6-10 days in a new `sciex-tof` module (cap = measured + 3%), only with V1 (msconvert in Docker or the authors' mzML of MTBLS13023/Zenodo 10889321). **Risk**: high (unknown codec; the `MSConfigInfo` calibration table may be needed for the m/z values). **Priority**: 2.
- Exit rule: if after 3 days there is no m/z within 5 ppm on a file with a reference, the chat stops, writes what it found into `wiff_specifica_completa.md` and the format stays "rejected with guide".

## `.wiff2` (SCIEX OS)
- Different container (SQLite + compressed streams, declared version), no public specification, no free reader; SCIEX OS is the only writer of the new instruments (7500, ZenoTOF 7600).
- **Honest decision**: no reader. Guide for the student: conversion with msconvert (Windows) or with SCIEX MS Data Converter (free, `-profile`/`-centroid`), then mzML in mzLab. Letter to SCIEX (the maintainer) asking for a specification for teaching use; reopened only with a specification or a redistributable SDK.
- **Rejection in the program**: `.wiff2` recognized and rejected with the key that points to the guide (already so).

## Work, risk, priority (`.wiff` QTRAP)
- **R-03**: 6-8 days. msconvert truth on all the corpus `.wiff` files (LOC-2), mixed polarity, `vial` and `SampleTable` in the sections, DP/EP/CXP, LC method of the other modules, `DevData` (traces) and IDA if the layout can be derived with V1, DAD against an Analyst export (V3), multiple periods if a file arrives. TOF as above.
- **Risk**: medium (layouts derived by observation; EU law 2009/24/EC art. 5.3). **Priority**: 1 (the laboratory's instrument).
- Cap of the `sciex` module: 327,680 B (not raised), measured 319,930 B: new code beyond ~7 KB goes into `sciex-tof` or a `sciex-ext` module.
