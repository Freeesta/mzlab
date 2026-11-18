# What the raw files contain and we do not extract yet

Rule: no loss of information from the raw files (`.wiff`, `.raw`, `.dam`). This list says what the raw file contains and reaches neither the mzML nor `method/1`; each entry is closed by extracting it or declaring it.

## Thermo `.raw` (v66, Q Exactive)
- UV 3D spectra (wavelength field of a Dionex DAD/PDA detector: a separate controller, 572 bytes per instant in the files read, layout not decoded): `method/1.warnings` says "UV 3D field (PDA spectra)" and `detectors` gives its range and step; single-wavelength traces (`UV_VIS_n`) are `Pda` chromatograms. The same holds for files with Accela PDA (not tried). TSQs that do not open in `opentfraw` (older Vantage/Quantum) and SRM scans without Q1 give no traces.
- Extra trailer per scan: 62 fields (injection time, AGC, FT resolution, isolation, charge, lock mass, ion optics, calibration, diagnostics). In the mzML every scan (also from `readers_mzml::scrivi`) has the 33 registered `opentfraw.*` and, for all the others, a `userParam` `opentfraw.trailer.<label>` (`extra::trailer_extras`); in `method/1.fields` every field is there with the value if it is equal in all scans, otherwise the range.
- `startTimeStamp` and SHA-1 of the file: in the mzML of a `.raw` downloaded from the site (`rawreader.rs`, `testata_mzml`) there are `startTimeStamp` (date of the preamble, otherwise the audit tag, local time of the instrument without time zone; it is the `file_created` of `method/1`), the SHA-1 of the `.raw` in the `sourceFile` and the `method/1` document in the `<run>` (userParam `mzLab method/1`).
- `conversion/1`: every conversion (`readers_mzml::Conversione`) writes the `userParam` `mzLab conversion/1`: reader, version, fields with and without CV term, scans `declared` and `read`, `partial` and the `loss` key of the cause (from `Reader::losses`), and `not_carried` (field and reason of what the format has and the mzML does not, declared by `Lettore::not_carried`).
- Pump pressure: the analog controller (90 001 `f64` samples before its RunHeader, name and unit in the strings that follow) comes out as `pressure chromatogram` (MS:1003019) in pascal, with the values in a `pressure array` (MS:1000821, UO:0000110; psi, bar, mbar, kPa and MPa converted once by the reader), all samples; other analog channels in another unit are not written (they stay in `method/1.fields`, `Controller<N>`).
- Status log (v66): the `GenericDataHeader` schema ends exactly at `inst_log_addr` and each record is an `f32` time in minutes + the fields (`opentfraw::status_log`, `RawFileReader::status_log`). The per-scan records of `inst_log` come out as `opentfraw.status.*`.
  - Each channel is a field of `method/1.fields` (origin `thermo-raw:StatusLog`, value if constant, otherwise `min..max`, with unit); section titles (`====  Ion Source  ====`) have no value. `meta.nonDecodificato` remains only if the schema is not found.
  - Channels that change (at least two different values) are time series (`status:<section>/<label>`), like the trailer fields that change (`trailer:<label>`, one point per scan, split by MS level if they change at several levels): `method/1.log_series` lists them (id, section, name, unit, points, min, max), the points are computed on request (`/api/registro?k=&id=`, `method_raw/series.rs`). They do not enter the mzML nor the `.mzidx` (they are not chromatograms of the file); in the Data view they are the "Instrument log" type.
  - The error log (`error_log`) is in `method/1.log` (`t_min`, `testo`, `level` if the text starts with Info/Warning/Error) and in the Method window with the times.
- Sequence row and ASInfo: the numbers and texts of the sequence row with no known meaning (`unk1`, `unk2`, `unk_a`…`unk_s`, `unk_long`) and the ASInfo block (24-byte preamble, autosampler text) are read and kept (`SeqRow::unknown_*`, `RawFileReader::as_info`): the non-empty ASInfo text is a `thermo-raw:ASInfo` field of `method/1`, the rest only in the `scandump/1` dump (`sequence`, `asinfo`); Sample Type and Level Name are assigned to the right position with the RFR dump, not by guessing.
- Tune: the "Tune Data" values and the mass calibration are in the trailer schema (already in `fields`), the base tune file in the method text (`raw`, `InitialTuneFile`). A separate tune block (LTQ/TSQ) has not been verified on real files: until it is tried, it is not declared extracted.
- The computer name and the paths are written in the mzML `<run>` (userParam `opentfraw.*`); the `method/1` document it carries reports them with `operator`, `instrument.serie` and the fields marked `identity`.

- Embedded method, not yet extracted (no instrument file at hand to measure the text): LC of Dionex Chromatography MS Link, Proxeon EASY-nLC, Accela/LegacyDualPump/Surveyor and Waters ACQUITY (`lc` stays empty, the whole texts are in `raw` and each script line in `fields`); linear-trap experiments (LTQ, Velos, Elite, Orbitrap XL: "Scan Event N" and dependent settings, only in `raw`); `AuditData` and `Header` of the OLE streams. The reason is the same: the text format is decoded on the real text, never by guessing.
- Chromatographic column: the files read do not carry it, so `lc.colonna` does not exist and is not invented.

## SCIEX `.wiff` / `.dam`
- PDA/UV 3D of the `.wiff` (streams `DAD/DADSampleHeader`, `DADIdx`, `DADRealTimeData`): decoded in `crates/mzlab-core/src/wiff/dad.rs` (`DadData`, `read_dad`), `dad_map.rs`, `dad_uv.rs` (feature `mappa-dad`, active in `mzlab-wasm`).
  - Extracts a 2D time × wavelength matrix in mAU (row-major `f32`), pre-summed TWC and single-wavelength chromatograms (`ChromKind::Pda`, `"mAU"`). 2D map, UV spectra and per-λ traces are reachable both natively and from the site via `itemroutes::pda_map/pda_spectrum/pda_wavelength`. Status: decoded, not validated against Analyst (no numeric comparison with Analyst exports; no `meta.nonDecodificato` warning).
- Device traces (`Devices/Device_N/DevData`: pressure in pascal, flow, temperature): chromatograms of the reader (`wiff/devices.rs`, `wiff-variants.md`).
- Not decoded: `DDERealTimeData` (IDA), TripleTOF/ZenoTOF blocks (other codec, refused), binaries `MSConfigInfo*`, `MassSpecMethod*`, `ExperimentHeader*Ex`, `PeriodStream*`, `CycleDepParam`.
- Mass calibration and resolution (`GLPTables`) and strings of `DabsInfo`/`SampleTable`: in `method/1.fields` from the `sciex` module; status of the Log at start and end of run, CFR audit, acquisition time per sample: not extracted.
- User, computer, institution and paths are in the OLE properties and in `CFR_INFO`: identity, not copied into the mzML.

## Agilent `.D`
- Sample type (`sequence.xml`), full GC method (`acqmeth.txt`, `7890Method.bin`), actual values (`GC.ini`), quantification method, audit trail.
- LC/MS (module `agilent`, `agilent-lcms.md`): diode-array data (`AcqData/DAD1.cd/.cg/.sd/.sp`), module traces (pressure, flow, temperature: `*.cd`/`*.cg`), per-scan actual values (`MSActual.bin`) and the binary files that are not decoded (`MSMassCal.bin` is read, the rest not) are named in `fields` with size and SHA-1 (if under 4 MiB) and in `warnings` (`meta.nonDecodificato`); `detectors` comes from the method, not from the data. ChemStation `.ch`/`.uv` files do not exist in real runs.

## Bruker timsTOF `.d` (module "bruker")
- Inverse mobility 1/K0 of every point (TIMS scan index, `TimsCalibration` `ModelType` 2, formula unknown): not computed; the scan range per spectrum and the calibration table are kept as name/value.
- TSF/MALDI: image point positions, regions and other tables beyond `Frames`/`GlobalMetadata`; profiles with zero samples. Neighbouring files: `chromatography-data.sqlite`, `.mcf`, `.baf`.

## mzML with numpress (module "aperti")
- In the rewritten spectrum, cvParams that the mzML writer does not know pass as userParam "ACC name" (not as cvParam); `dataProcessing`, `referenceableParamGroup` and `scanSettings` of the original file are not carried.

## mzXML (module "aperti")
- Not carried: the text of the header elements (e.g. the SHA-1 at the end of the file), `scanOrigin`, `maldi`, `separation`/`spotting` of a scan, `processingOperation`; the attributes of every header element are instead all in `method/1.fields`, those of every scan in `intestazione`.

## Waters MassLynx `.raw` (module `waters`)
- Ion mobility (HDMSe, SONAR, cyclic IMS): the m/z are summed over all drift bins; the drift time and the mobility calibration (`mob_cal.csv`) are not extracted. Lock mass: the function is given as a chromatogram and in `fields`, but the m/z correction is not applied (30-100 ppm).
- Not decoded: the lines of `_FUNCnnn.STS`/`.EE` beyond "Set Mass" and "Collision Energy" (they are only in the bytes), the `_HISTORY.INF` table, the ramp parameters beyond those of `_extern.inf`, the 6-byte encoding (no real file) and MALDI-type functions.
- Identity (`User Name`, `Submitter`, `Laboratory Name`, computer paths): in `method/1.fields` marked `identity`, never in the mzML.

## Shimadzu `.qgd` / `.lcd` (module `shimadzu`)
- Not read: oven and injector program (`GC-2010/GC17A Instrument Parameters`), `GCMS Configuration` (only the instrument name), the embedded `.qgm` method, `Status` and `Status Curve` (pressures, temperatures), `MS Status Data`, `Mass Parameters`, `GUMM_Information`; UV (`LSS Raw Data`, `PDA 3D Raw Data`); the polarity event of `Mass Raw Data`; collision energy and dwell time of MRM records; the Q-TOF/IT-TOF/9030 (`QTFL RawData`) entirely.
- Not carried into the mzML: operator, computer, paths and names of the data files, method, tune and batch (`identity`). Details and offsets in `docs/agenti/shimadzu.md`.
