# Rosina C: per-format sheets

One sheet per format: state, what is missing, possible proofs (scale V1-V7 of `../ROSINA.md` §4), metadata field by field, work, risk, priority. The vendor sheets are in the neighbouring files; here are the index, the legend and the open formats.

| Format | Sheet | State today | Priority | Package |
|---|---|---|---|---|
| Thermo `.raw` | `SCHEDA_THERMO.md` | spectra L (Orbitrap, Q Exactive, Exploris); TSQ and traps P | 1 | R-02 |
| SCIEX `.wiff` (QTRAP, API) | `SCHEDA_SCIEX.md` | L for the 3200 and QTRAP 5500/6500 (MRM, EPI, PIS, NL); multiple periods refused | 1 | R-03 |
| SCIEX `.wiff` TOF (TripleTOF, ZenoTOF, X500) | `SCHEDA_SCIEX.md` | refused (TOF block codec) | 2 | R-03 (module `sciex-tof`) |
| SCIEX `.wiff2` | `SCHEDA_SCIEX.md` | refused | guide | none |
| Agilent `.D` GC/MS and LC/MS | `SCHEDA_AGILENT.md` | GC/MS L; QqQ MRM/dMRM and Q-TOF P; DAD not decoded; IMS refused | 1 | R-04 |
| Waters `.raw` | `SCHEDA_WATERS.md` | MRM/SIR P (V2 from Workbench); TOF, MSe P; lock mass not applied; IMS summed | 2 | R-05 |
| Bruker TDF/TSF | `SCHEDA_BRUKER.md` | TDF L for m/z (V1 on compression 1); 1/K0 absent; compression 2 only V7 | 2 | R-06 |
| Bruker BAF/YEP | `SCHEDA_BRUKER.md` | not read | guide | none |
| Shimadzu `.lcd`/`.qgd` | `SCHEDA_SHIMADZU.md` | P: no reference; polarity, CE and dwell not decoded; 9030 refused | 3 | R-07 |
| netCDF/ANDI | here | L (V2 with the `.D` of the same run) | 3 | R-08 |
| Third-party mzML | here | L (`metodo/1` header P: no real msconvert mzML) | 1 | R-01, R-08 |
| mzXML | here | L, V7 only | 3 | R-08 |
| mzData | here | not read | 4 | R-08 |
| MGF, MSP | here | read as libraries, not as a run | 4 | R-08 |
| imzML | here | not read | out of scope | note in R-08 |
| LECO `.peg`, JEOL, PerkinElmer, AB SCIEX `.t2d` | here | not read | out of scope | guide |

## Legend of the metadata tables
- **Vendor field**: name as written by the vendor software (or, if none, the description).
- **Where in the file**: member, stream, table or offset; "?" = position not yet found.
- **`metodo/1`**: path in the document (`campi` = only in the flat list with `origine` and `chiave`; `grezzo` = only in the text). Identities (people, computers, paths) carry `identita` and do not pass into the mzML.
- **State**: **L** read into a section; **C** only in `campi`/`grezzo`; **N** not read or not decoded; **?** to be measured in the indicated package.
- **Check**: how the vendor software shows it (V3) and which automatic oracle gives it (V1/V2). "mzML" = msconvert's mzML writes it (cvParam or userParam); "TRFP" = ThermoRawFileParser JSON `-m 0`.
- Every **N** or **?** row of a sheet becomes an item of the package, with the stated proof, or a declared `meta.nonDecodificato` warning (`avviso`) in the document: never silence.

## Open formats

### netCDF/ANDI-MS (`.cdf`)
- **State**: reader `andi` L (CDF1/CDF2, SIM, profile, polarity); V2 against the `.D` of the same run. Exported by ChemStation, Xcalibur, LabSolutions, ChromaTOF.
- **Missing**: files exported by LECO, Shimadzu and Thermo (the corpus is mostly Agilent); the ANDI-Chrom variant (UV only, `.cdf` without `mass_values`): an honest error today?; netCDF4/HDF5 refused (right: it is not ANDI).
- **Proofs**: V2 (native file of the same run), V4 (netCDF4-python, xcms `readMSData`), ASTM E2077 specification (mandatory variables).
- **Work**: 1-2 days (R-08). **Risk**: low. **Priority**: 3.

| Field | Where | `metodo/1` | State | Check |
|---|---|---|---|---|
| vendor, model, serial, firmware, software | global attributes `instrument_mfr`, `_model`, `_serial_no`, `_fw_version`, `_sw_version` | `strumento.*` | L | `ncdump -h` |
| experiment date | `experiment_date_time_stamp` | `origine.file_creato` | L | `ncdump -h` |
| title, operator | `experiment_title`, `operator_name` | `campione.nome`, `operatore.utente` | L | `ncdump -h` |
| ionisation, polarity, scan function, type | `test_ionization_mode`, `_polarity`, `test_scan_function`, `experiment_type` | `ms.sorgente.tipo`, scans | L | `ncdump -h` |
| mass range, resolution, scan time, detector | remaining `test_*` | `campi` | ? (R-08) | `ncdump -h` |
| data source (native file, export software) | `source_file_reference`, `netcdf_file_date_time_stamp`, `languages` | `campi` | ? (R-08) | `ncdump -h` |
| per scan: RT, TIC, ranges | `scan_acquisition_time`, `total_intensity`, `mass_range_*` | spectra, TIC | L | xcms |

### mzML written by others
- **State**: reader and header L; `metodo_mzml.rs` reads the header up to 64 scans. Not verified on real mzML from msconvert, OpenMS, MZmine, SCIEX MS Data Converter, Bruker CompassXport.
- **Missing**: corpus of mzML from 6 writers; wrong or absent `indexedmzML` index (per-scan fallback?); numpress L (module `aperti`); chromatograms without spectra (pure SRM) L; `referenceableParamGroup` used by Agilent and SCIEX; mobility as an array (`MS:1002816`) and as separate scans.
- **Proofs**: V4 (pyteomics, OpenMS `FileInfo -v`, `psims`), XSD `mzML1.1.0_idx.xsd`, PSI semantics (`FileInfo -c`).
- **Work**: 2-3 days (R-01 the converter, R-08 the header of real files). **Risk**: low. **Priority**: 1 (it is the output of every converter and the most common input: 27 % of MetaboLights studies).
- Fields: every header element (`fileDescription`, `sourceFileList` with SHA-1, `instrumentConfigurationList`, `softwareList`, `dataProcessingList`, `scanSettingsList`, `sampleList`, `run@startTimeStamp`) goes into `campi` as `mzml:<element>`; per-scan parameters (injection time, resolution, filter, CE, isolation) go into `ms.periodi[0].esperimenti[]` if constant, otherwise as a range. Check: `FileInfo -v` and direct XML reading.

### mzXML
- **State**: L (32/64 bit, zlib, index), V7 only: no mzML of the same run in the corpus.
- **Missing**: V2 (MetaboLights has studies with mzXML and the native file in the same deposit: 11.9 % of studies); multiple `msInstrument`; `parentFile` with SHA-1.
- **Proofs**: V2 (native file of the same deposit converted by msconvert on Windows), V4 (pyteomics `mzxml`).
- **Work**: 1 day (R-08). **Risk**: low. **Priority**: 3.

### mzData
- **State**: not read (1.2 % of MetaboLights studies, almost all before 2012).
- **Recommendation**: small reader in the new module `storici` (plain XML, base64, no compression), only if R-08 has room; V4 proof with OpenMS `FileConverter`/`FileInfo` (reads mzData).
- **Work**: 2 days. **Risk**: low. **Priority**: 4.

### MGF and MSP
- **State**: read as libraries (`mzlab-lib`, fuzz target `mzlib_msp_mgf`), not as a run. An MGF of a DDA run (1.1 % of studies) has no MS1 and no TIC.
- **Recommendation**: open an MGF as an "MS2 list" in the HR bench, not as a run; no invented chromatogram. The maintainer decides (R-08 proposes it).

### imzML
- **Out of scope for Rosina** (0.8 % of studies; a different view). R-08 writes only a note: structure (`.imzML` + `.ibd`, UUID, continuous/processed mode), what would be needed.

### LECO `.peg`, JEOL, PerkinElmer, others
- LECO ChromaTOF (2.7 % of MetaboLights studies, 2.3 % of Workbench, almost all GC×GC-TOF): closed format, no free reader, no DLL in msconvert. **Guide**: export netCDF from ChromaTOF (`andi` reads it).
- JEOL, PerkinElmer, AB SCIEX `.t2d` (MALDI 4800), Thermo `.dat` of old ISQ: no measurable share in the deposits (< 0.1 %). **Guide**: export mzML or netCDF from the instrument software.
