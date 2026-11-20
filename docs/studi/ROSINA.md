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
Truth file `verita/1`: one JSON per native file (one per sample in multi-sample containers) with the oracle name, version, options and date, counts, every scan, sample points, chromatograms and metadata. A new oracle adds a truth, never replaces one.

### F.2 Test levels (every reader, every corpus file)

| Level | Check |
|---|---|
| F0 | structure: recognised, samples, no panic, errors as keys |
| F1 | counts per level and chromatogram type, id sets |
| F2 | scans: RT 1e-4 min, level, polarity, precursor, isolation, CE, profile/centroid |
| F3 | points: HR 0.5 ppm (1 ppm with declared per-scan calibration), LR 1e-3 Da, relative intensity 1e-5 |
| F4 | chromatograms: TIC, BPC, SRM/SIM, analog, UV; times 1e-4 min, values 1e-6 |
| F5 | metadata: every truth field equal, mapped with declared conversion, or a defect |
| F6 | converter: XSD, `FileInfo -v`, `indexedmzML` offsets and SHA-1, round trip, profile/centroid flags |
| F7 | robustness: fuzz >= 300 s per family, truncation at 1/4, 1/2, 3/4, memory within limits |
| F8 | scale (local only): time and peak memory on the largest file |

Outcome per file: `L` (F0-F5 against an oracle or the authors' mzML), `P` (a level is missing, stated), `NV` (internal consistency only). The format row in `docs/agenti/native-reading.md` follows this outcome.

### F.3 Where it runs
- Public CI (`test.yml`): F0-F4 and F6 on synthetic and small public files; F5 on the synthetic `metodo/1` documents.
- Private nightly workflow in `mzlab-dati` (text in `rosina/WORKFLOW_NOTTE.md`): checks out `Freeesta/mzlab` `main`, runs `cargo test -p mzlab-lettori --features tutte --test 'truth_*'` (one test per family) on every `scratchpad/rosina/verita/**/*.verita.json` (files <= 90 MB); the format x level summary stays in an artefact, never public.
- Local (Mac): F8 and the files in the big-files folder, with the same test and `MZLAB_DATI_GRANDI=<path>`.

### F.4 Edge cases to find (an empty cell means no file yet)
Per-scan calibration or lock mass; vendor centroids against profile; polarity switching; multiple scan events or periods; ion mobility (FAIMS, SelexION, drift tube, timsTOF, cyclic); time and intensity units (min, s, ms; Bruker normalisation to 100 ms); format versions (Thermo v57-v66, Analyst 1.4-1.7 and SCIEX OS, Agilent B.0x-12, Waters 4.1/4.2, timsControl 1-5, Shimadzu 5.x); files over 4 GB; several samples per container; truncated or in-acquisition files (Bruker `-wal`); endianness. Fill the cell per vendor in the R-package that covers it.

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
