# Study "Rosina": reading every mass spectrometer file (native readers and mzML converter)

Status: decided, not done. The bench (R-00) is built; the reader packages R-01..R-08 are open.
## Goal
mzLab reads, on any computer, the files of every vendor, either natively or as an mzML converter (profile and centroids), and reads their metadata perfectly. Starting point: `docs/agenti/native-reading.md` (inventory and matrix). The real gap is not files but truth: most corpus files have no reference mzML, so this is first an oracle-and-truth project, then a reader project.

## Census shares (public deposits, measured 2026-10-19)
- PRIDE (42 168 projects): Thermo Orbitrap/FT 81.4 %, Bruker timsTOF 5.5 %, SCIEX TOF 4.8 %, Waters 2.4 %, Bruker other 2.2 %, Thermo trap/QqQ 1.5 %, Agilent 0.5 %, SCIEX QTRAP/QqQ 0.2 %.
- MetaboLights (3 546 studies), file formats: Thermo `.raw` 42.1 %, mzML 27.1 %, mzXML 11.9 %, SCIEX `.wiff` 10.2 %, `.d` 10.2 %, Waters 7.7 %, netCDF 6.6 %, mzData 1.2 %, MGF 1.1 %, Shimadzu 1.1 %, imzML 0.8 %, LECO 0.4 %.
- MetaboLights, vendor from protocol text: Thermo 46.6 %, SCIEX 22.9 %, Agilent 14.9 %, Waters 11.7 %, Bruker 6.9 %, LECO 2.7 %, Shimadzu 1.7 %.
- Metabolomics Workbench (7 321 MS analyses): Thermo 50.6 %, Agilent 23.6 %, SCIEX 14.0 %, Waters 5.0 %, Bruker 3.0 %, LECO 2.3 %, Shimadzu 0.7 %; triple quadrupoles 1 227 (SCIEX 628, Agilent 315, Thermo 149, Waters 100, Shimadzu 22).
- MassIVE (20 414 datasets): Thermo 68 %, Bruker 13 %, Agilent 6 %, SCIEX 4 %, Waters 4 %, Shimadzu 0.7 %.
- Deposits are research-biased (proteomics = Orbitrap); routine and environmental labs (QqQ, GC-MS) are underrepresented, so for our domain SCIEX and Agilent weigh first.

## Priority of formats
1. Thermo; 2. SCIEX `.wiff` (TripleTOF/ZenoTOF still refused); 3. Agilent; 4. the mzML converter; 5. Waters; 6. Bruker timsTOF; 7. Shimadzu; 8. netCDF (LECO/Shimadzu/Thermo); 9. mzData and imzML; MGF as a run and `.wiff2`/BAF only as a guide.

## Legal rules
- Observe files and write an interoperable reader; never decompile vendor SDKs or software (their licences forbid it; EU directive 2009/24/EC art. 5.3 allows observation).
- Vendor SDKs (RawFileReader via ThermoRawFileParser, `timsdata`, MassLynx SDK, msconvert) are used only as oracles, as end users on Mac/Windows; nothing from an SDK enters the repo, the site or the desktop app. Save the accepted licence before each oracle and check that comparison use is allowed.
- Redistribution follows the licence: CC0, CC-BY, MIT, Apache with attribution; CC-BY-SA, CC-BY-NC, EBI terms of use and files without a licence stay private. Clinical files carry personal data: only accessions and numbers appear in this repo.

## D. Data policy
- Real data live only in the private repo `Freeesta/mzlab-dati`, resolved by `tools/dati_percorsi.py`; never write paths by hand. Files over 90 MB stay outside git (listed in a manifest, fetched by `tools/rosina/scarica.py`, ceiling 20 GB on the Mac).
- Private layout: native files in `dati/formati/<vendor>/<family>/<accession>_<experiment>/` with `references/<oracle>-<version>/` (the vendor's or authors' mzML, if <= 90 MB); truths in `scratchpad/rosina/verita/` with the same relative path (`<file>.verita.json`). Vendors: `thermo`, `sciex`, `agilent`, `waters`, `bruker`, `shimadzu`, `leco`, `open` (mzML, mzXML, mzData, imzML, netCDF with no known vendor). The instrument recorded is the one read in the file, not the one the deposit declares.
- One index `INDICE.tsv` (writes `INDICE.md`): path, vendor, model in the file, format version, experiments (closed vocabulary), source, accession, DOI, licence, redistributable (`yes`/`no`/`to-check`), bytes, SHA-256, URL, reference (`none`, `authors`, `msconvert-<v>`, `trfp-<v>`), truth (`yes`/`no`), notes. Big files are listed in `GRANDI.tsv` and fetched by `tools/rosina/scarica.py` (resume, byte and checksum check, refuses to pass the ceiling); on the Mac they live outside git (ceiling 20 GB). Licence texts of every source and of the SDKs used as oracles sit in `dati/formati/_licenze/`.
- The public repo gets only synthetic files and, after the maintainer's licence check, small CC0/CC-BY files (<= 2 MB each, <= 10 MB total) in `tests/formati/` with attributions and no personal or clinical names.

## F. Test protocol

### F.1 Truth file
Truth file `verita/1`: one JSON per native file (one per sample in multi-sample containers). A new oracle adds a truth, never replaces the old one (two oracles that disagree are information).
- `schema`; `file` (logical name, SHA-256); `oracle` (`name`, `version`, `options`, `date`, `platform`); `counts` (spectra per level, chromatograms per type).
- `scans[]`: `id`, `rt_min`, `level`, `polarity`, `precursor_mz`, `isolation`, `ce`, `activation`, `profile`, `n_points`, `tic`, `bpi_mz`, `bpi`.
- `points`: for 8 chosen scans (first, last, median, max TIC, one per level) m/z and intensity as exact numbers.
- `chromatograms[]`: `id`, `type`, `n`, `sum`, `max`, 16 sample points.
- `metadata`: field -> value as the oracle gives it, with the vendor name and the expected `metodo/1` path from `SCHEDE.md`.
- Optional: `tolerance` (`hr`, `lr` or an object that changes single thresholds) and `mobility` (Bruker). Typical size 50-500 KB.
Generators (in `tools/rosina/`, they read what an oracle produced; the oracles run on Mac or Windows, never in the site): `verita.py MZML` (from the reference mzML), `verita_trfp.py` (ThermoRawFileParser JSON), `verita_timsdata.py` (`timsdata` 1/K0 table), `verita_scandump.py` (from a `scandump/1`).

### F.2 Test levels (every reader, every corpus file)

| Level | Check | Tolerance |
|---|---|---|
| F0 | structure: recognised, samples, no panic, errors as keys | none |
| F1 | counts per level and chromatogram type, id sets | equal |
| F2 | scans: RT, level, polarity, precursor, isolation, CE, profile/centroid | RT 1e-4 min; m/z 1e-4; rest equal |
| F3 | points: m/z, intensity, number of points | HR 0.5 ppm (1 ppm with declared per-scan calibration), LR 1e-3 Da; relative intensity 1e-5 (exact f32 if the file stores f32) |
| F4 | chromatograms: TIC, BPC, SRM/SIM, analog, UV: same ids, times, values | times 1e-4 min; values 1e-6 relative |
| F5 | metadata: every truth field equal, mapped with declared conversion, or a defect | text equal after declared normalisation (spaces, units) |
| F6 | converter: XSD, `FileInfo -v`, `indexedmzML` offsets and SHA-1, re-read = reader, profile/centroid flags and `dataProcessing` | no error |
| F7 | robustness: fuzz >= 300 s per family, truncation at 1/4, 1/2, 3/4, memory within limits | no panic, memory under the ceiling |
| F8 | scale (local only): time and peak memory on the largest file of the family | written to `docs/agenti/measurements.md` |

Outcome per file: `L` (F0-F5 against V1 or V2 evidence, see Evidence), `P` (a level is missing, stated), `NV` (V7 only). The outcome, not an impression, updates the format row. The format row in `docs/agenti/native-reading.md` follows this outcome.

### F.3 Where it runs
- Public CI (`test.yml`): F0-F4 and F6 on synthetic and small public files; F5 on the synthetic `metodo/1` documents.
- Private nightly workflow in `mzlab-dati` (text in `rosina/WORKFLOW_NOTTE.md`): checks out `Freeesta/mzlab` `main`, runs `cargo test -p mzlab-lettori --features tutte --test 'truth_*'` (one test per family) on every `scratchpad/rosina/verita/**/*.verita.json` (files <= 90 MB); the format x level summary stays in an artefact, never public.
- Local (Mac): F8 and the files in the big-files folder, with the same test and `MZLAB_DATI_GRANDI=<path>`.

### F.4 Edge cases (case x format; an empty cell is "no file", to be found)
- Per-scan calibration / lock mass: Thermo LTQ Velos Pro and XL lock mass; Agilent Q-TOF `MSMassCal.bin`; Waters MTBLS1584, MTBLS178; Bruker temperature per frame.
- Vendor centroids against profile: Thermo Q Exactive; SCIEX QTRAP profile (lab); Agilent ST003656 profile + centroids; Waters G2-S; Bruker TSF lines/profile; Shimadzu: no file.
- Polarity switching: Thermo MTBLS7889, MTBLS1752; SCIEX Zenodo 22673685; Agilent, Waters, Shimadzu: no file.
- Several scan events or periods: Thermo Tribrid DIA; SCIEX Z15679243 (16 experiments; multiple periods: no file); Agilent time segments; Waters functions; Shimadzu 2020 events.
- Ion mobility: Thermo FAIMS and SCIEX SelexION: no file; Agilent MTBLS427 6560; Waters Synapt HDMS and cyclic; Bruker timsTOF.
- Units: Thermo min; SCIEX s or min per experiment; Agilent min; Waters min; Bruker s with normalisation to 100 ms; Shimadzu ms.
- Format versions: Thermo v57-v66 (count in R-02); SCIEX Analyst 1.4-1.7 / SCIEX OS; Agilent B.0x-12; Waters 4.1/4.2; Bruker timsControl 1-5; Shimadzu 5.x. An unseen version gives an error with the version in `detail`, never a guess.
- Files over 4 GB: Thermo Astral/large Orbitrap; SCIEX `.wiff.scan` (largest held 1.1 GB); Agilent ST000044 2.1 GB; Waters PXD051707 7.9 GB; Bruker alphatims 1.9 GB.
- Several samples per container: SCIEX Z22807392 (20 samples). Truncated or in-acquisition files: Bruker PXD074950 `-wal`; the rest to build. Endianness: all little-endian (Bruker SQLite big-endian).

## Method
Principles: (1) the vendor's own library is the oracle, not only msconvert; every oracle runs only on the Mac or a Windows PC, never in the site, and its result is frozen in a truth file; (2) triangulate: at least two independent sources per file (mzLab reader, vendor oracle, authors' mzML, free reader such as opentfraw, openwraw, timsrust, pyteomics); if mzLab agrees with one only, the field is "to clarify", not "verified"; (3) metadata field by field (`SCHEDE.md`): a field the oracle sees and mzLab does not is a defect (rule "no loss", `docs/agenti/raw-not-extracted.md`); (4) converter conformance: mzML 1.1 XSD, OpenMS `FileInfo -v`, declared `psi-ms.obo` version, `indexedmzML` offsets and SHA-1, mzLab centroids declared in `dataProcessing`, never passed off as the vendor's; (5) robustness and scale: one fuzz target per family seeded from the first KB of truncated real files; one large file per family opened locally with time and peak memory written to `docs/agenti/measurements.md`, 64-bit offsets proven on real files; (6) community: ask for ProteoWizard `vendor_readers/*Test.data` fixtures, propose missing CV terms to HUPO-PSI, ask SCIEX (`.wiff2`) and Bruker (BAF) for documentation, use the NORMAN network for environmental files.

| Oracle | Gives | Runs on |
|---|---|---|
| ThermoRawFileParser (RawFileReader) | Thermo reference mzML and JSON metadata (`-m 0`) | Mac (.NET), native |
| msconvert (ProteoWizard, vendor DLLs) | reference mzML (64 bit, zlib, no peak picking) and one with `peakPicking vendor` | Windows (Parallels on the Mac), by the maintainer; not Mac arm64 (Wine aborts) |
| `timsdata` (Bruker SDK) | 1/K0 per scan, m/z per frame | Windows (Parallels); on the Mac `opentimspy` gives only TOF and scan indices |
| MassLynx Raw SDK (Waters) | m/z with and without lock mass, drift time, function headers | Windows; read the licence first |
| Vendor software (Analyst, Xcalibur, MassHunter, MassLynx, LabSolutions, DataAnalysis) | metadata "as the user sees it" copied by hand (e.g. Analyst of the lab 3200: `.wiff`/`.dam` with the method printout) | instrument PCs |
| OpenMS `FileInfo -v`, `xmllint --schema mzML1.1.0_idx.xsd`; pyteomics, pymzml, OpenMS, mzmine 4 | syntactic and semantic validation of mzLab mzML; round trip with third-party readers | Mac and CI |
| cargo-fuzz (`fuzz.yml`, Sunday), ImHex / Kaitai Struct | one target per `mzlab-lettori` family; readable binary spec of observed formats | CI; Mac |

Open-source readers (ProteoWizard `vendor_api` and `vendor_readers`, ThermoRawFileParser, OpenMS, opentims, timsrust, alphatims, pyteomics, pymzml, rainbow, openwraw, opentfraw, openaraw, opensxraw, openszraw) are read to understand, never copied. Oracle outcomes of the maintainer's Windows runs are recorded in the private repo (`coordinamento/dati_esterni/rosina/ORACOLI.md`).

## Evidence (strongest first)
- V1: same file, oracle that uses the vendor library: same spectra (ppm, relative intensities, scan count, TIC) and same metadata.
- V2: the authors' own mzML in the same deposit (version declared in the mzML).
- V3: vendor software on screen (metadata read by hand; numbers of a peak).
- V4: an independent free reader agrees.
- V5: a published result on the same dataset (feature or identification count, standard areas) reproduced within the paper's tolerance.
- V6: a file from a third laboratory (other deposit and instrument).
- V7: internal consistency (TIC = sum of intensities, index = record) and fuzz corpus without crashes: necessary, never sufficient.
The converter also needs the PSI validator passed and an identical re-read with 3 third-party readers.

| Format | V1 route | V2 | V3 | V4 | Minimum proof for `L` |
|---|---|---|---|---|---|
| Thermo `.raw` | TRFP on the Mac (proven on Q Exactive and TSQ Quantum Access Max, JSON metadata too); msconvert on Windows | rare | Xcalibur if available | opentfraw (the base: not independent) | V1 on one file per instrument family (TSQ, Q Exactive, Orbitrap Tribrid, trap) |
| SCIEX `.wiff` | msconvert | PIS/NL of Zenodo 7897576, MTBLS1381 | Analyst of the lab 3200 | opensxraw (declared unreliable) | V1 or V2 per instrument and experiment |
| SCIEX `.wiff2` | msconvert or SCIEX MS Data Converter | rare | SCIEX OS | none | no reader: guide only |
| Agilent `.D` | msconvert (MHDAC) | ST003656, ST004061, ST004922 (already used) | MassHunter if available | openaraw, rainbow (DAD) | V1/V2 for QqQ, Q-TOF profile, centroids, DAD |
| Waters `.raw` | msconvert; MassLynx SDK for lock mass and drift | ST004453 (TQ-XS), ST003942, ST003984 | MassLynx if available | openwraw | V1 for TOF, MSe, DDA, lock mass |
| Bruker TDF/TSF | `timsdata` (m/z, 1/K0) and msconvert on Windows | ProteoWizard fixtures | DataAnalysis if available | timsrust, alphatims | V1 for 1/K0 and compression 2 |
| Bruker BAF | msconvert, `baf2sql` | rare | DataAnalysis | none | no reader: guide only |
| Shimadzu `.lcd`/`.qgd` | msconvert (LC only) | none found | LabSolutions if available | openszraw (immature) | V1 for LC; for `.qgd` V3 or a `.cdf` exported from the same file |
| netCDF/ANDI | no (open format) | `.D` of the same run (done) | exporting software | netCDF4-python, xcms | V2/V4 + ASTM spec |
| Others' mzML, mzXML, mzData | no (open formats) | conversion of the same native | none | pyteomics, OpenMS | V4 with two third-party readers |

Metadata as a priority of the maintainer: per format a table "vendor field -> place in the file -> `metodo/1` path -> how the vendor software shows it -> automatic oracle" (`rosina/SCHEDE.md`). Format versions: an unseen version gives an error with the version in `detail`, not a guess.

## Packages (E)
| # | Package | Touches | Depends on | Truths from |
|---|---|---|---|---|
| R-00 | Done: shared bench (see Done) | `tools/rosina/`, `crates/mzlab-lettori/tests/`, `fuzz/`, private repo | none | none |
| LOC-2 | Oracles on the whole corpus: TRFP JSON and mzML on the Mac; msconvert (no peak picking and `peakPicking vendor`) and `timsdata` 1/K0 on Windows; truths with the R-00 scripts; scale (size, time, RAM) | private repo | R-00 | local chat and the maintainer |
| R-01 | mzML converter: XSD and semantic validation, declared profile/centroids, round trip with third-party readers | `lettori_mzml`, `lettori_centroidi`, writers | R-00 (no vendor oracle needed) | none |
| R-02 | Thermo: TSQ, PDA, lock mass, traps, metadata against TRFP | module `thermo`, `metodo_raw` | R-00, LOC-2 | TRFP |
| R-03 | SCIEX `.wiff`: QTRAP 4000-7500, multiple periods, EPI, vial, DAD; TripleTOF/ZenoTOF in a new module `sciex-tof` | `sciex`, new `sciex-tof` | R-00, LOC-2 | the maintainer: Analyst of the 3200 |
| R-04 | Agilent: Q-TOF 284 B and 6546, DAD, module traces, 6560 honest refusal, metadata | `agilent` | R-00, LOC-2 | msconvert |
| R-05 | Waters: TOF spectra, MSe, DDA, lock mass, drift | `waters` | R-00, LOC-2 | MassLynx SDK |
| R-06 | Bruker: 1/K0, compression 2, TSF LC; BAF as a guide | `bruker` | R-00, LOC-2 | `timsdata` |
| R-07 | Shimadzu: parity with msconvert, polarity, CE/dwell, 9030 in a new module `shimadzu-tof`, PDA | `shimadzu`, new `shimadzu-tof` | R-00, LOC-2 | msconvert |
| R-08 | Minor open formats: netCDF of LECO/Shimadzu/Thermo, mzData, `metodo/1` of others' mzML on real files; imzML evaluation only | `andi`, new `storici` | R-00 | none |
Each package touches only its own folder and area file, so they run in parallel. Ceilings: no chat raises an existing one (engine 1 703 936; modules as in `docs/agenti/readers-module.md`; `finder` 512 000; `chem` 569 344). A piece that does not fit goes to a new module of the same family with a proposed ceiling = measurement + 3 %, written by the chat and approved by the Coordinator. Oracle proofs live in tests and `tools/`: zero weight in the site.

## Decisions
1. Oracles: data stay on the Mac; ThermoRawFileParser runs there; msconvert and `timsdata` run on Windows (Parallels) by the maintainer. No oracle in GitHub Actions. Until an oracle exists: authors' mzML (V2); Bruker gets an internal comparison.
2. `.wiff2`: no reader; conversion guide and a written request to SCIEX; reopen if SCIEX documents the format.
3. Bruker BAF: no reader in the site nor the desktop app (`baf2sql` is a native SDK); guide only.
4. Waters lock mass: read as today (file m/z) by default; reference-function correction becomes a declared option, proven against the MassLynx SDK (R-05).
5. Converter centroids: two declared modes, "vendor centroids" (never redone) and "mzLab centroids" (`dataProcessing` names mzLab); the default stays as today.
6. New modules instead of higher size ceilings: `sciex-tof`, `shimadzu-tof`, `storici` (mzData, maybe MGF as a run).
7. CI with data: nightly workflow in the private repo (`rosina/WORKFLOW_NOTTE.md`); no data in the public repo.
8. imzML is out of scope (0.8 % of studies, a different view); a note in R-08 only.

## Done
R-00 bench: `tools/rosina/*.py` (truth from mzML, TRFP, `timsdata`, scandump; download; index), `crates/mzlab-lettori/tests/truth*.rs`, fuzz targets for the reader families, `tests/test_rosina_strumenti.py`; recorded in `docs/agenti/test.md` and `docs/agenti/scandump.md`.

## Open
- Packages R-01 (mzML converter), R-02 (Thermo), R-03 (SCIEX), R-04 (Agilent), R-05 (Waters), R-06 (Bruker), R-07 (Shimadzu), R-08 (minor open formats). Priority order: R-02, R-03, R-04, R-01, R-05, R-06, R-07, R-08.
- `.wiff2` and BAF remain refused (`docs/agenti/native-reading.md`); modules `sciex-tof`, `shimadzu-tof`, `storici` do not exist yet. No chat raises an existing ceiling. Sources: `rosina/FONTI.md`; sheets: `rosina/SCHEDE.md`.
- Next step: R-02 (Thermo TSQ and traps against ThermoRawFileParser truths), then R-03 and R-04.
