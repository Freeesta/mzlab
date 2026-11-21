# Agilent MassHunter `.D` reader (GC/MS)

`crates/mzlab-core/src/agilent/`: pure Rust, no new crate. A `.D` folder is read from its files: `AcqData/MSScan.bin` (index, one record per scan) and `AcqData/MSPeak.bin` (the points), plus the small files (`MSTS.xml`, `Devices.xml`, `Contents.xml`, `MSTree2.bin`, `acqmeth.txt` in the `.D` folder).

The Agilent 7000D GC/TQ in full scan with centroids (MS1, positive EI, `f32` points) is also supported: read like the 5977. NOT read: SIM, MRM and product-ion of the TQ. Tested on synthetic `.D`; verification on the real `.D` (13,002 scans, record TIC = sum of the points) is left to the local machine.

LC/MS runs (QqQ, Q-TOF) are read by the variant `crates/mzlab-core/src/agilent_ext/` (feature `estesi`, module `agilent`): `agilent-lcms.md`.

The format was derived from three real runs of a 5977 single-quadrupole GC/MS (EI, positive; one in 50-500 m/z scan, two in SIM) and from the schema `AcqData/MSScan.xsd` that Agilent puts in the folder; Reference: the mzML files that ProteoWizard msconvert made from the same three runs (64 bit, zlib, no peak picking, SIM as spectra; data repository, `GC-MS/mzML/`), plus internal checks (below). Everything that is not in the three runs is listed in "Not verified".

## Files of a `.D` folder and what they are for
- `AcqData/MSScan.bin`: index (format below). `AcqData/MSPeak.bin`: points. These two are enough to read everything.
- `AcqData/MSTS.xml`: time segments (`TimeSegment`: `StartTime`, `EndTime` in minutes, `NumOfScans`). `NumOfScans` counts by TIME: the scan straddling two SIM segments may have the previous segment in the record and `MSTS.xml` the next (a difference of 1 between the two counts; the total matches).
- `AcqData/MSIndex.bin` (only in SIM runs): for each segment a short header (with the number of scans of the segment) and the scan times (f64, minutes); the first segment was verified: times equal to those of the records. The reader does not need it.
- `AcqData/MSTree2.bin`: `u32 86, u32 5`, three texts (`u8` length + bytes: device, technique, instrument), four f64 (first and last time, minimum and maximum m/z), `00 01 01 00 00 00`, scan type text ("Scan", "SelectedIon"), `u32 1`, polarity text ("Positive"), `u32` number of scans, then the times. `info::tree` reads only the head and gives `None` if the shape is different.
- `MSTree2.bin` after the times: the TIC of each scan as `f32` (equal to the records in all three runs; tested in `tests/agilent_tree.rs`), then one block per ion: `01 00 00 00 00 00 00 00 00 00 00 00` (in the first block; the following blocks start with `01` and an index), a `u32`, the ion (m/z, f64), `u32` 1, four `u32` at zero, the number of values and the values as `f32`.
- In the SIM run of the data (3 ions, 3417 scans) the values of an ion are its intensities in the 1142 scans that contain it (equal to `MSPeak.bin`); in the full-scan run (ion 51.0, 216 values) they are NOT the intensities of `MSPeak.bin` (1639.9 against 1453 in the fourth scan): it is a value computed from the raw data with an unknown rule.
- After the ion blocks there is a long tail of increasing `u32` (up to 4.8 MB) not decoded. None of this is additional original data: they are copies and derivatives of the records and of `MSPeak.bin`.
- `AcqData/Devices.xml` (name, model, serial, firmware), `Contents.xml` (date, separation technique 2 = GC, software), `sample_info.xml` and `sequence.xml` (sample, operator, vial: not read), `acqmeth.txt` (UTF-16, the method; `method::parse`), `*.M/` (data analysis method: not read), `data.ms` (old ChemStation format, "GC / MS Data File": the source data from which `IndexedDataConverter.log` generated `AcqData`; not read), `checksum.xml`, `GC.ini`, `runstart.txt`, `PRE_POST.INI`, `TEMPBASE`.

## Everything a `.D` contains and whether I extract it
Rule: no information of the raw files is thrown away; what I cannot map is kept as name/value with the source file (`AgilentD::metadata`, `meta::collect`; the caller passes the small files in `Files::others`).

| File | Content | Extracted? |
|---|---|---|
| `AcqData/MSScan.bin` | index, one record per scan | all fields: the mapped ones in `ScanRec`, the others (status, sampling period, measured mass range, threshold, `MinX`/`MaxX`…) in `ScanRec::extra` with the names in `AgilentD::extra_names`; the 160-byte header (apart from the offset at +88) is not decoded |
| `AcqData/MSPeak.bin` | points (m/z, intensity): block per scan, `n` m/z then `n` intensities, all `f64` (16 bytes/point, 5977) or all `f32` (8 bytes/point, centroids of the 7000D GC/TQ in full scan, `dataStorage PeakDetected`: without `MSProfile.bin`, `MSMassCal.bin`, `MSTree2.bin`); the block size chooses the layout, other sizes are an error. SIM/MRM/product-ion of the TQ are not read | yes; the 68 header bytes are not decoded |
| `AcqData/MSScan.xsd` | record schema | yes, it determines the layout; kept as text |
| `AcqData/MSTS.xml` | time segments | yes (`segments`, and all fields in `metadata`) |
| `AcqData/MSIndex.bin` | scan times per segment | no (they equal the record times; verified on the first segment) |
| `AcqData/MSTree2.bin` | instrument, scan type, polarity as text, number of scans, times, `f32` TIC, per-ion blocks, unknown tail | only the head (`tree`); TIC and first ion block checked in the test, the rest is not decoded (declared with bytes and SHA-1) |
| `AcqData/Devices.xml`, `Contents.xml` | model, serial, firmware, driver, date, technique, software | yes, all fields in `metadata` |
| `AcqData/sample_info.xml`, `sequence.xml` | sample, vial, volume, operator, method, paths | yes, in `metadata` (name/value and attributes) |
| `AcqData-CumulativeAuditTrail.xml`, `checksum.xml` | audit trail (user, computer, date, comment), hash | yes, in `metadata` |
| `acqmeth.txt` | method (oven, ALS, inlet, column, transfer line, MS, SIM groups, tune) | whole text + mode, solvent delay, range, SIM groups with ions and dwell + tune parameters (`tune NAME`) |
| `GC.ini`, `PRE_POST.INI` | GC state at the end of the run (temperatures, flows, columns), tune, factors, injection, hardware | yes, `[section] key = value` in `metadata` |
| `runstart.txt`, `IndexedDataConverter.log` | run-start variables, converter log | whole text in `metadata` |
| `TEMPBASE` | empty | no |
| `data.ms` | ChemStation file (big-endian) from which MassHunter converted `AcqData`; format below | read (`datams.rs`): header strings, scans, times, TIC; numeric header and table after the records declared with bytes and SHA-1; gives the spectra only if `AcqData` is missing |
| `AcqData/*.M/` and `*.m/` (copy of the acquisition method, `DaMethod/Quant`, `Unknowns`) | RTP, monitor, audit trails, quantitation (RT, ions, levels, curve) | XML yes (`metadata`), binaries and texts not decoded with SHA-1; the `.m` outside the run are not read |
| TIC and base peak | record TIC per scan (= mzML TIC) and `BasePeakMZ`/`BasePeakValue` | TIC yes (chromatogram); the base peak is in the record, not written in a chromatogram |

## `MSScan.bin`
- The record layout is read from the folder's `AcqData/MSScan.xsd` (`Layout::from_xsd`: the `xs:element` of `ScanRecordType` in order, those in comments excluded, `SpectrumParamValues` = the elements of `SpectrumParamsType`; types int/long/double/short/byte/float). For the three runs the xsd gives exactly the 184 bytes and the table's offsets (tested). Without an xsd `Layout::builtin` applies (the same table). Each field is looked up by name in the layout index (the first with that name): an xsd of 20,000 fields reads its records in linear time (`tests/robustness_agilent_layout.rs`).
- Ion mobility (IMS) run: a `ScanRecordType` with `FrameID` and `DriftBin` and without `SpectrumParamValues` (IMS frames divided into drift bins) gives `err.file.agilentIms` on both builds (`scan::ims`, before looking for the other types): format not read, nothing loaded, the step is the mzML from the instrument's software. Test: `crates/mzlab-core/tests/agilent_ims.rs`.
- An unknown type, a missing mandatory field or a size that does not divide the file are errors with the numbers (and, if `MSTS.xml` announces N scans, the record size that the file would have).
- 160-byte header, equal in the three runs; the `i32` at +88 is the offset of the first record (160). Then `(size − 160) / 184` records of 184 bytes, without padding, little-endian. The number of records = number of scans (3868 and 3417 in the runs; `MSTS.xml` gives the same totals).
- Record, one field per row (offset, field, type, what is seen):

| Offset | Field | Type | In the runs |
|---|---|---|---|
| 0 | `ScanID` | i32 | 1, 2, 3… |
| 4 | `ScanMethodID` | i32 | 1 |
| 8 | `TimeSegmentID` | i32 | from 0 |
| 12 | `ScanTime` | f64 | minutes |
| 20 | `MSLevel` | i32 | 1 |
| 24 | `ScanType` | i32 | 1 scan, 2 SIM |
| 28 / 36 / 44 | `TIC`, `BasePeakMZ`, `BasePeakValue` | f64 | |
| 52 | `CycleNumber` | i32 | = ScanID |
| 56 | `Status` | i32 | 0 |
| 60 | `IonMode` | i32 | 2 (EI) |
| 64 | `IonPolarity` | i32 | 0 (all positive) |
| 68 / 76 / 84 | `Fragmentor`, `CollisionEnergy`, `MzOfInterest` | f64 | 0 |
| 92 | `SamplingPeriod` | f64 | 0 |
| 100 / 108 | `MeasuredMassRangeMin`, `Max` | f64 | 0 and 1050: instrument limit, not the method's ions |
| 116 | `Threshold` | f64 | 0 |
| 124 / 128 | `IsFragmentorDynamic`, `IsCollisionEnergyDynamic` | i32 | 0 |
| 132 | `SpectrumFormatID` | i32 | 1 |
| 136 | `SpectrumOffset` | i64 | inside `MSPeak.bin` |
| 144 / 148 | `ByteCount`, `PointCount` | i32 | |
| 152 / 160 | `MinY`, `MaxY` | f64 | MinY is always the maximum f64 (initial value) |
| 168 / 176 | `MinX`, `MaxX` | f64 | m/z ÷ 20 |
- `MinX`/`MaxX` hold the first and last m/z divided by 20: the reader does not use them. `BasePeakMZ` can differ by up to 0.05 from the m/z of the highest point (calibration), `BasePeakValue` by a few tens of counts from `MaxY`: the reader uses the points.
- `ScanTime` is the end of the scan: the SIM group of a scan is the one in progress at the time of the previous scan (the scan printed as 8.5009 min still carries the ions of the group that ends at 8.5).

## `data.ms`
- Header: type `01 32`; Pascal strings (length + text) at 0x04 (file type), 0x18 (sample), 0x94 (operator), 0xB2 (date), 0xD0 (instrument), 0xE4 (method); `u16` at 0x10A = start of the records in words minus 2; `u32` at 0x116 = number of scans (big-endian). The rest of the header is not decoded.
- Record: `u16` length in words, `u32` time in ms, `u16` 12 or 34, `u16` 1, `u16` 0, `u16` n peaks, the base-peak pair, n pairs `(u16 m/z×20, u16 abundance)` from the highest m/z to the lowest, then `u16` 5, `u16` 4, `u16` 0 and `u32` TIC. Abundance: 2 exponent bits e, 14 mantissa bits, value = mantissa × 8^e. Length in words = (24 + 4 × (n + 1)) / 2.
- After the last record follows a table (about 20 % of the file) not decoded: it is declared with bytes and SHA-1.
- The count at 0x116 is checked: whole records beyond those declared (length from the number of peaks, `5 4 0` tail, time not smaller) are read too and the declared number goes in the metadata ("scans declared by the header"), never just the first scans (`whole_records_after`, in `agilent/` and `agilent_ext/`).
- A `data.ms` record that does not fit (length, number of peaks) ends the reading: the scans before are read, the declared ones after are counted in `AgilentD::damaged` (`warn.agilent.damaged`); a file that ends inside a record counts them in `missing`. Only the first broken record is `err.file.agilent` with the numbers.


## `MSPeak.bin`
- 68 header bytes (`03 01` and zeros), then the blocks, one per scan, one after the other: the first is at 68 and the last ends at the last byte of the file (no byte without an owner).
- Format 1: `PointCount` m/z values then `PointCount` intensities, all f64 (16 bytes per point, `ByteCount = 16 × PointCount`). The values are as precise as an f32 (the low 4 bytes are zero). The points are already filtered CENTROIDS (the 50-500 scan has from 14 to 300; a SIM scan has one per ion of the group: 3 or 4) in increasing m/z order. No compression.
- Sum of the intensities = record `TIC` within 0.2 % (the TIC is an integer, the intensities are calibrated).

## Checks made on the three runs (`crates/mzlab-core/tests/agilent_real.rs` and `agilent_synthetic.rs`, skipped without the data)
Scans numbered 1…N, increasing times, blocks covering `MSPeak.bin` from 68 to the end, total of `MSTS.xml` = records, scans per segment counted by time = `MSTS.xml`, head of `MSTree2.bin` (count, "Positive"), sum of intensities = TIC, maximum = base peak, increasing m/z, ions of each SIM scan = ions of the method group (`acqmeth.txt`), m/z range of the scan inside `Low Mass`/`High Mass`, written mzML read back equal. Synthetic tests (without data in `agilent_synthetic.rs`): full cycle, empty scans, MS2 with precursor, errors.

## Parity with msconvert (`crates/mzlab-core/tests/agilent_msconvert.rs`, skipped without the data)
- On the three runs: same number of spectra (3417, 3417, 3868), same points per spectrum, equal intensities (difference 0), time within 1e-9 min (msconvert writes 12 decimals), level 1, positive polarity, TIC (chromatogram) equal point by point, TIC and base peak per scan equal to those of the record.
- Known differences: m/z up to 1.5e-5 (msconvert writes them as short decimals, the file has f32 values); nativeID `scanId=N` in msconvert, `scan=N` here because the program derives the scan number from `scan=`; msconvert adds the term "SIM spectrum", the scan windows and a SIC chromatogram per ion (`Q1` = method m/z minus 0.05, 7 in the SIM runs): not here, the XICs are built by the program from the spectra.

## Loss audit (`tests/agilent_audit.rs`, `tests/agilent_audit/`)
- Inventory of the raw data made without the reader's code: list of files, text and attribute values of the XMLs (plain-text scan), values of the `.ini`, fields of the record layout. Each entry must be in `AgilentD::metadata` (name or value, origin = path in the folder), in `ScanRec` or `extra_names`, or in `scarti.txt` with the reason. Files that are not decoded (`MSTree2.bin`, header of `MSScan.bin`, unknown formats) enter with "undecoded bytes" and "undecoded sha1".
- `noti.txt` (only file names and counts, the method names are replaced by `*.M`): if a known file gives fewer entries than before the test fails; it is rewritten with `AUDIT_WRITE=1`. Always-active synthetic test: a never-seen field is kept.
- The XML reader of the small files handles UTF-16 XML, entities (`&lt;` arrives separately), mixed text with children and the display name of the fields of `sample_info.xml`: the audit verifies it.
- The `*.M` folders inside `AcqData` (copy of the acquisition method: `7890Method.bin`, `Acq.RTP*.xml`, audit trails, `quantitative.xml` with ions and calibration levels) are read as small files (`Files::others`): name/value XML, the rest declared with SHA-1.

## What it writes in the mzML besides the spectra
- Header: `startTimeStamp` = `AcquiredTime` of `Contents.xml` as written (with its time zone, without fractions of a second); msconvert writes it 2 hours ahead (16:00:45+01:00 becomes 17:00:45Z), not here. Serial (MS:1000529) from `Devices.xml`; acquisition software MS:1000678 with the version of `Contents.xml` (msconvert writes 8.0, the file says 10.0); `sampleList` and userParam "sample type" from `sequence.xml`; for each file read MassHunter format, nativeID format and SHA-1 (`MSPeak.bin` is computed before converting, in 1 MB blocks); `fileContent` with centroid, TIC and BPC.
- Per spectrum: "SIM spectrum" (MS:1000582) in SIM runs, observed minimum and maximum m/z, scan window (that of the method, `Low Mass`/`High Mass`, for scans; first and last ion for SIM). Chromatograms: TIC and BPC (from the records).
- SIC: in runs with SIM scans, one SIC (MS:1000627, id `SIC <m/z>`) for each ion, built from the points of the SIM scans (distinct ions at 0.01 m/z; two points of the same ion in the same scan are summed). The SIC is a derived value: msconvert does not write it, and there is no real SIM run to compare it with (verified only on synthetic data).
- Not written: the spectrum title.
- GC method (`AcqMethod::gc`, `method::parse_gc`): oven (initial, hold, ramps, equilibration), injector (mode, heater, pressure, total flow), column (description with the serial: identity field, dimensions, flow, mode, velocity, holdup), transfer line, injected volume. The audit with msconvert is `tests/agilent_cv.rs`: every CV accession of msconvert is also in our mzML, except the exceptions listed in the test with the reason.

## `method/1` (`src/agilent/doc.rs`, `tests/agilent_method.rs`)
- `AgilentD::metodo(&Source)` gives the `method/1` document of the run (sections and mapping in `docs/agenti/method.md`); `readers_mzml::Conversione` writes it in the `<run>` as `userParam`. The site's method reader (`mzml_metodo`) therefore finds it in every mzML of a converted `.D`.
- Tests on the real data: the keys of the document and of its sections are those of `method-1.json`, every name/value of the side files is in `fields` (count and one by one), the whole texts are in `raw`, and the mzML carries the same document with the SHA-1 of `MSPeak.bin`.

## As `Lettore` (`src/agilent/reader.rs`, `tests/agilent_reader.rs`)
- Registered in `LETTORI` as `agilent-d` (extension `.d`). The members of the `FileSet` have the folder's names (`AcqData/MSScan.bin`, `acqmeth.txt`, …; a prefix `x.D/` gives the run name). `riconosci`: `Si` with an `MSScan.bin` whose head has a plausible record offset (byte 88) and an `MSPeak.bin`, `Forse` without `MSPeak.bin`; a `data.ms` with the bytes `01 32` is `Si`. Nothing else counts, the extension alone does not.
- `open` reads whole only the index and the small files (counted against `Limiti::byte`, the entries against `Limiti::voci`); `MSPeak.bin` stays a `ReadAt`: `scansione(0, k)` reads only the bytes of scan `k` (same `Spectrum` as `AgilentD::run`, tested on all runs). A run with only `data.ms` is served from the text read at opening.
- `indice`: the scans with points, with `Posizione` in `MSPeak.bin`; the SIM filter has no ions (they are in the spectrum). `intestazione`: every field of the record (also the extra ones of the xsd) with the CV where there is one. `cromatogrammi`: TIC, BPC and a SIC for each ion of the SIM groups of the method; the SIC is computed by reading the SIM scans on request. `metodo`: the `method/1` document without the SHA-1 of `MSPeak.bin` (not everything is read to compute it).

## How it becomes `Run`/mzML
- `AgilentD::open(name, &Files)` (records, segments, device, contents, tree, method), `run(&mut ReadAt)`, `visit`. One spectrum per scan with points (empty ones are skipped), `id = scan=<ScanID>`, centroid.
- Level = `MSLevel`, precursor = `MzOfInterest` only if level ≥ 2 and > 0, polarity: code 0 → +1, 1 → −1 (not seen), other → 0; filter "EI Scan" or "EI SIM 95.1,108.1,135.1" (EI if `Contents.xml` says GC). TIC = `TIC` of the level-1 records. The errors are `err.file.notAgilentD` (`MSScan.bin` missing), `err.file.agilent` with `detail` (record size, spectrum format, spectrum outside `MSPeak.bin`, non-numeric value, with the step to take) and `err.file.agilentCut` (`name`: truncated copy with no whole scan): never a guess.
- Truncated copy (`scan::parse`): the whole scans remain, the others go in `missing`. It is a cut only if `MSTS.xml` announces more scans than the whole records of `MSScan.bin` (without `MSTS.xml` a piece of record at the end stays `err.file.agilent`) or if the records with points beyond the end of `MSPeak.bin` are the last ones.
- Damaged record (`scan::parse`): a record without time or with points outside `MSPeak.bin` before good records is damaged: left out and counted in `AgilentD::damaged`, cause `warn.agilent.damaged` (`AgilentD::losses`; loading row `conv.row.leftOut`), the other scans are read. No whole scan: `err.file.agilentCut`, or `err.file.agilentNoPeaks` if `MSPeak.bin` is missing or empty. Test: `crates/mzlab-core/tests/agilent_cuts.rs`, `tests/test_lettori_parita.py`.
- Ancillary files (`MSTS.xml`, `Devices.xml`, `Contents.xml`, `MSTree2.bin`, `acqmeth.txt`, `sample_info.xml`, `sequence.xml`; in `estesi` also `MSMassCal.bin`, `DefaultMassCal.xml`, `AcqMethod.xml` and the members of `AcqData`): one that cannot be read (I/O, byte budget) is left out and named in `method/1.warnings` (`meta.damaged`, `AgilentD::notes`), the run is read; only `MSScan.bin`, `MSScan.xsd` and `data.ms` stop it.

## In the site (native reading, like the `.wiff`)
- Engine: `crates/mzlab-wasm/src/natives.rs` (`rb.nativeSamples`, `build_mzidx_native`, `native_to_mzml`). The `.D` folder does not require an intermediate mzML conversion: it is read natively through the `Lettore` trait of `mzlab-core`.
- Page: `agilent-d.js` (`AgilentD`: `locate`, `group`, `mzmlName`) groups the `x.D/AcqData/…` members, passes them to `rb.nativeSamples(members)` (`native_samples`: `{format, samples:[{id,name,scans}]}`) and makes an `x.mzML` sample with `mzlabNat` handled directly by the engine (`docs/agenti/native-reading.md`, `rust-nativo.js`). The `.mzidx` index is built directly from the members with `build_mzidx_native`; "Download .mzML" generates the mzML only on request through `rb.nativeToMzml` (`native_to_mzml`).
- Whole folder: `CartCarica.load` (`cartella-carica.js`) groups the files by `.D` folder (relative path) and passes to `upload()` the native sample per run; a `.D` without `MSScan.bin`/`MSPeak.bin` gives a warning with a key (`err.file.notAgilentD`; with a `data.ms` the folder opens from that), without the Rust engine `err.file.agilentNoEngine`.
- Browser test: `tests_e2e/e2e_agilent.py` (synthetic folder of `tools/agilent_sintetico.py`: two `.D` and an mzML, warning, TIC, no request to Pyodide), `tests_e2e/e2e_lettori.py`, `tests_e2e/e2e_agilent_riaggiunto.py` (the same folder again stays one sample: `docs/agenti/native-reading.md`). A `.D` dragged or chosen without "Load a folder" (loose files) is not recognised; the desktop uses the same native readers.
- Weight: the reader is in the engine (no on-demand loading); the `.wasm` limit is in `pages.yml`.
- Memory: whole `MSScan.bin` (184 bytes per scan) plus the records (about 130 bytes): 100,000 scans = about 30 MB of the 192 MB budget; if the allocation fails the error is `error.memoryBudget`. Spectra pass one at a time; no mzML is in memory during loading.

## Not verified
- LC/MS (QqQ MRM and dMRM, Q-TOF profile, centroids and MS/MS, negative polarity, LC/MS method) is in `agilent-lcms.md`: in the engine (without `estesi`) an LC/MS `.D` still gives `err.file.agilent` with the numbers found (xsd with nested types, spectrum format other than 1, bytes per point other than 16, `UncompressedByteCount` other than `ByteCount`), never a guess; the `agilent` module opens it (`probe` gives `variante`).
- A single path between site and CLI: the CLI and the app always read with `agilent_ext` (the "all" features of `mzlab-lettori` turn on `estesi`), the site asks the engine first and passes to the `agilent` module the keys of `errori` in `lettori-famiglie.js` (`err.file.agilent`, `err.file.agilentNoPeaks`). For this reason the engine refuses with `err.file.agilent` a run with MRM visits (`ScanType` 256), which the module reads as SRM chromatograms. Test: `crates/mzlab-core/tests/agilent_path.rs`, `tests/test_agilent_percorso.py`.
- `data.ms` read alone: fixed positive polarity (`polarity: 0`) and EI source deduced from the GC type, not read from the file; the polarity byte in the record is not known.
- Ion mobility (6560): no real run; the fields exist in the schema and are read as above, the meanings of the `IonMode` codes are unknown.
- `data.ms` is not an exact duplicate of `AcqData`: times, TIC and number of scans are equal, but the m/z are integers in twentieths (up to 0.05 from the calibrated values of `MSPeak.bin`) and the abundances have 14 mantissa bits; read alone (run without `AcqData`) it gives these values. The format was verified only with the three 5977 runs (type `01 32`); other variants (other file type, records of other length) give `err.file.agilent` with the numbers.
