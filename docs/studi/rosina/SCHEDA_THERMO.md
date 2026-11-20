# Rosina C: Thermo `.raw`

Legend and index: `SCHEDE.md`. Reader: module `thermo` (`opentfraw` adapted), `metodo_raw.rs` and submodules; details in `docs/agenti/metodo.md` and `docs/agenti/lettura-nativa.md`.

## Status
- Spectra and chromatograms L on Orbitrap/Q Exactive/Exploris (parity with the msconvert mzML on the corpus files that have it); method L (serial and version equal to msconvert, gradient equal to the script).
- P: TSQ Vantage/Quantum (may not open in `opentfraw`), LTQ/LCQ/Velos traps, multiple-DIA Orbitrap Tribrid, Astral (not in the corpus), 3D PDA (not decoded), format versions v57-v66 not counted.
- Share: 81% of the PRIDE projects, 47% of the MetaboLights MS studies, 51% of the Workbench analyses. It is the most used format in the world: every defect here weighs more than elsewhere.

## What is missing
1. Automatic V1: TRFP (RawFileReader) gives spectra and JSON metadata; today parity is only against msconvert mzML files present in the corpus.
2. Families without a reference: TSQ (PXD043691, ELENCO-1), traps (corpus), Exploris with FAIMS (no file), Astral (no small file).
3. PDA/UV: single-wavelength traces are read, the 3D field is not (572 bytes per instant, unknown layout). Proof: MTBLS2956 (ELENCO-1) with the TRFP export? TRFP does not export the PDA: only V3 (FreeStyle) or a free reader (`rawrr` no).
4. Tune data and mass calibration (per-scan coefficients) not in `metodo/1`.
5. Centroid peak data (noise, baseline, resolution, charge per peak: "label data") not exported.

## Possible checks
- V1: TRFP on the Mac (`-f 2` mzML, `-m 0` JSON metadata, `-L` levels) and msconvert in Docker; repeatable, free, RawFileReader license (use allowed, no reverse engineering of the library: only the outputs are compared).
- V2: PXD with the authors' mzML (rare). V3: FreeStyle/Xcalibur if the laboratory has a copy. V4: none independent (`opentfraw` is our base).
- Minimum proof for "L" per family: F0-F5 on 1 Orbitrap file, 1 Q Exactive, 1 TSQ, 1 trap, 1 Tribrid, 1 with PDA.

## Metadata field by field
| Vendor field | Where in the file | `metodo/1` | Status | Check (V3 · oracle) |
|---|---|---|---|---|
| Instrument Model, Name, Serial Number, Software Version, Hardware Version | InstID block (strings 1-5) | `strumento.modello`, `.serie`, `.software`, `.hardware` | L (positions measured on v66, ? other versions: R-02) | FreeStyle "Instrument" · TRFP `InstrumentModel`, `SerialNumber`, `SoftwareVersion`; mzML `instrumentConfiguration` |
| File version, creation date, file creator | FileHeader | `origine.file_creato`, `campi` | L | TRFP `CreationDate`; mzML `run@startTimeStamp` |
| Operator (audit tag), computer | AuditStart `tag2`, computer name | `operatore.utente`, `.computer` (identita) | L | TRFP `Operator` |
| Sample Name, Sample ID, Vial, Inj Vol, Dilution, Sample Wt, Sample Vol, ISTD Amt, Comment, Instrument Method, Processing Method, Calibration File | sequence row (SeqRow) | `campione.*` | L | FreeStyle "Sample Information" · TRFP `SampleData` |
| Sample Type, Row Number, Level Name, User Text 0-4, Path, Raw File Name | sequence row | `campi` | L: Row, User Text 0-4, Path, Raw File Name; ? Sample Type, Level Name: the unnamed numbers and texts come out by position in the `scandump/1` dump (`sequence.unknown_*`) pending the RFR dump | FreeStyle · TRFP |
| Start/End Time, Low/High Mass, number of scans, expected run time, mass resolution, tolerance unit | RunHeader | `campi` | L | TRFP `StartTime`, `EndTime`, `NumberOfMs1Scans` |
| Instrument method (text per device: MS, LC, autosampler) | embedded OLE container (`Text`, `Data`, `Header`, `Comments`) | `grezzo`, `lc`, `campionatore`, `ms.impostazioni[]`, `ms.periodi[0].esperimenti[]` | L | FreeStyle "Instrument Method" · TRFP `-m` (method text) |
| Inclusion/exclusion list | table after `INCLUSION LIST` in the text | `ms.periodi[].esperimenti[].inclusione[]` | L (test on real text: skipped without the private repo) | FreeStyle |
| TSQ SRM table (Q1, Q3, CE, RF/Tube Lens, Dwell, times, polarity) | MS method text | `transizioni[]` | L; truncation declared (`meta.nonDecodificato`) | TSQ Tune/FreeStyle · mzML `SRM SIC` |
| Scan filter per scan | ScanEvent + text filter | scans (`filterString`) | L | TRFP `filter string`; mzML `MS:1000512` |
| MS order, polarity, scan mode, analyzer, ionization, activation, CE, isolation width and offset, multiplex, SPS | ScanEvent | scans; experiments from the XML | L (SPS/MS3 ?: R-02) | mzML `precursor`, `activation`; TRFP |
| Extra trailer (62 fields: Ion Injection Time, AGC Target, Monoisotopic M/Z, Charge State, Master Scan Number, FT Resolution, LM Correction (ppm), Micro Scan Count, HCD Energy, FAIMS CV, Scan Description…) | per-scan trailer | `campi` (value or range); some in the scans | L | FreeStyle "Scan Header" · TRFP `-L` · mzML (injection time, monoisotopic) |
| Status log (pressures, temperatures, voltages, flows per channel and time) | status log | `campi` (`thermo-raw:StatusLog`), `serie_registro`, `/api/registro` | L | FreeStyle "Status Log" · TRFP? (does not export it: V3) |
| Error log | error log | `registro[]` | L | FreeStyle "Error Log" |
| Tune data (Spray Voltage, Capillary Temp, Sheath/Aux/Sweep Gas, S-Lens RF, Heater Temp…) | tune header/tune data per segment | — | N (T-4: the spot in the file cannot be found without an RFR dump) | FreeStyle "Tune Data" · RawFileReader `GetTuneData` (via TRFP? no: V3) |
| Lock mass: mode, lists, number in the trailer, correction | method text + trailer | `ms.calibrazione.lock_mass` | L (description only, no corrected data: correct, the instrument already applies it) | FreeStyle |
| Per-scan mass calibration coefficients (A, B, C…) | scan header (conversion params) | — | N (used for the m/z values, not exposed) | no direct oracle: implicit in the m/z values (F3) |
| Analog and single-λ UV channels (name, unit, samples, driver, model, serial) | controller `Controller<N>` | analog/UV chromatograms, `campi` | L | FreeStyle · TRFP? (no) · mzML (msconvert writes them as chromatograms) |
| 3D PDA (UV spectra per instant) | PDA controller (572 B per instant) | — (`avvisi` "UV 3D field") | N | FreeStyle; no automatic oracle |
| UV detectors (wavelengths, bandwidth, reference, 3D field, rate) | Chromeleon script | `rivelatori[]` | L | FreeStyle "Instrument Method" |
| Centroid data per peak (noise, baseline, resolution, charge) | centroid stream / label data | — | N | TRFP `-N` (noise) · RawFileReader `GetCentroidStream` |
| Instrument profile and centroids | scan data + centroid stream | spectra (`profilo`) | L | TRFP `-p` (no peak picking) against the default |
| Sequence/batch (`.sld`) | separate file, not in the `.raw` | — | out of scope: only the sample row | Xcalibur Sequence |

## Work, risk, priority
- **R-02**: 6-8 days. TRFP truth for every family (LOC-2), tune data, SPS/MS3, missing sequence fields, TSQ Quantum/Vantage (open or reject with the right key), format versions counted on the corpus, 3D PDA only if a file with a known export exists (otherwise the `avviso` stays).
- **Technical risk**: medium (version variants). **Legal**: low (observed format; RawFileReader used only as an oracle, its license forbids decompiling it, not comparing outputs).
- **Priority**: 1. Cap of the `thermo` module: 540,672 B, measured 523,597 B: new code must fit in the 17 KB free or go into a `thermo-ext` module (feature `estesi`) proposed by the chat.
