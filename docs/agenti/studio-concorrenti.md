# Positioning study: mzLab and other LC-MS tools

Report only: no code depends on this file. It compares what mzLab does (from `docs/agenti/`) with the public documentation of other tools, to help choose what to build next.
Every external fact carries a source key `[n]` (list at the end); all pages were read on 2026-11-15. `*` = not confirmed on a page opened for this study (search snippet, blocked site or general knowledge): check before quoting it.
Not done here: no tool was installed or run, no result quality was compared (that is `mzfinder-benchmark.md`), vendor prices are not public.

## 1. What mzLab does (source: area files)
- **Formats read**: SCIEX `.wiff`/`.wiff.scan` (3200 QTRAP; other QTRAP partial), Thermo `.raw`, Agilent `.D`, Bruker `.d` TDF/TSF, Waters `.raw`, Shimadzu `.qgd`/`.lcd`, ANDI `.cdf`, mzML (with MS-Numpress), mzXML; in the site and in the app, by Rust readers, without vendor libraries (`lettura-nativa.md` "Recognised formats", `lettori-modulo.md`). Refused: SCIEX `.wiff2` and TOF instruments (`wiff-varianti.md`), Shimadzu Q-TOF (`lettori-modulo.md`).
- **Conversion to mzML**: every read format, one writer, compression choices, a `conv/1` record of what was carried and what was not (`cli-convert.md`).
- **Viewing and integration**: TIC, BPC, XIC, spectra, RT×m/z map, DAD/UV (`dati.md`, `dati-mappa.md`, `dad.md`); automatic or manual integration of XIC and MRM traces with S/N and quality measures, Excel/CSV export; calibration lines as the Flow step `quant.curve`, not yet in the view (`calibrazione.md`).
- **Triple quadrupole**: SIM/MRM transition table (CSV: `name;q1;q3;polarity;rt;rt_window;isomer_group;note;formula;adduct`), per-sample integration, RT candidates (`transizioni.md`); scheduled MRM is read (`wiff-varianti.md`); the acquisition method is read as `metodo/1` (`metodo.md`). No method is written for an instrument.
- **Non-targeted HR**: feature detection on centroid MS1 (`mzfeat.md`), a low-resolution full-scan pipeline (`mzfeat-lr.md`), RT correction off|piecewise|LOESS|RANSAC (`mzfeat-broad.md`), alignment and gap filling (`mzfeat.md`), a second pipeline with stage-by-stage parity to mzmine 3.9.0 (ADAP, local-minimum resolver, join aligner, gap filler, isotope grouper; `attributio-mz.md`), DIA pseudo-MS2 off by default (`dia.md`).
- **Ion grouping and formulae**: isotopes, ion families, correlation groups and ion identity networking with GNPS edge columns (`gruppi-ioni.md`); steps `formula.ms1`, `formula.ms2` (`cli-passi.md`).
- **Libraries**: `.mzlib`; parsers for MSP, MGF, MassBank `.txt`, MoNA/GNPS/mzMine JSON; entropy, cosine, modified cosine, analogue search, decoys and FDR estimate (`match-spettrale.md`, `misure.md`).
- **MS/MS interpretation**: in-silico candidates from a PubChemLite package (`insilico.md`), MS2 engine and MetFrag-like fragmentation shown as evidence only, off by default (`attributio-ms2.md`), MSn fragmentation graph and fragment formulae (`msn-grafo.md`, `msn-interpreta.md`), class rules (`classi.md`), Schymanski levels (`livelli.md`). No external service is called (`rust-step.md`).
- **Statistics**: Welch/ANOVA with Benjamini-Hochberg, PCA, KMD as native-only steps; no view shows p-values (`stat.md`).
- **Workflows and provenance**: `mzflow/2` plans checked before running, `prov/1` record with SHA-256 of inputs and every parameter marked estimated or fixed (`rust-flow.md`); `mzrun/1` record/preset, site and app routes still to come (`mzrun.md`); site, app and CLI give the same bytes (`parita.md`).
- **Reports and exports**: printable HTML and JSON study report `mzreport/1` (`dati-strumenti.md`); table CSV (`full`, `full_mzmine`, `quant`), mzTab-M (`cli-passi.md`, `mzfeat-tabella.md`); FBMN zip, SIRIUS `.ms`, MGF, MSP/MassBank (`mzfinder-untargeted.md`, `cli-passi.md`).
- **Tables read from other tools**: mzMine, FBMN, MS-DIAL alignment `.txt`, GNPS2 (`mzfinder-untargeted.md`).
- **Deployment**: static site with an offline copy (`esporta-dati.md`, `compat-safari.md`), Tauri desktop app (`rust-desktop.md`), `mzlab-cli` (`cli.md`); MIT licence (`LICENSE`); no account.

## 2. Feature matrix
Cells: yes / partial / no / unknown (= not found in the pages read). CD = Thermo Compound Discoverer; SCIEX = SCIEX OS. Vendor columns rest partly on snippets `*`.

| Capability | mzLab | MZmine 4 | MS-DIAL 5 | XCMS (R) | OpenMS | Skyline | patRoon | CD | SCIEX |
|---|---|---|---|---|---|---|---|---|---|
| Vendor raw read directly | yes, 6 families, any OS and browser | partial: Thermo all OS; Bruker, Waters, SCIEX not macOS; Agilent, Shimadzu Windows via MSConvert [1] | yes in official builds [4][5] | no [6] | partial: Thermo, Bruker TDF [8] | yes, Windows [11] | no, converts first [10] | partial: Thermo only* [17] | partial: own only [18] |
| Open formats in | yes: mzML, mzXML, netCDF | yes [1] | partial: mzML, netCDF [4] | yes [6] | yes [8] | unknown | yes [10] | unknown | no; exports mzML [18] |
| TIC/XIC viewing | yes | yes* | yes* | partial: scripted plots [6] | yes: TOPPView [8] | yes [11] | partial* | yes* | yes [18] |
| Integration auto + manual | yes | partial* | partial* | yes [6] | partial: auto | yes* | partial* | unknown | yes [18] |
| Calibration, absolute quantitation | no | unknown [1] | unknown [4] | unknown [6] | partial* [8] | yes [11] | partial: predicted conc. [10] | unknown | yes [18] |
| MRM / triple-quad data | yes: read, integrate | partial: paid module [1] | no; MRMPROBS [4] | partial [6] | yes [8] | yes, with method export [11] | unknown | unknown | yes, with acquisition [18] |
| Non-targeted peak picking | yes | yes [1] | yes [4] | yes [6] | yes [8] | no [11] | yes, wraps others [10] | yes* [17] | partial* [18] |
| RT alignment | yes | yes [1] | yes [4] | yes [6] | yes [8] | no [11] | yes [10] | yes* [17] | unknown |
| Gap filling | yes | yes [1] | unknown | yes [6] | unknown | no | unknown | yes* [17] | unknown |
| Isotope / adduct grouping | yes | yes [1] | partial* | partial [6] | yes [8] | partial: user gives adduct [11] | yes [10] | yes* | yes [18] |
| Formula assignment | yes | yes [1] | partial: MS-FINDER [4] | unknown | partial [8] | no | yes [10] | yes* | yes* [18] |
| Spectral library search | yes, local | yes [1] | yes [4] | no | yes [8] | partial: `.blib` [11] | yes [10] | yes, incl. online mzCloud [17] | yes [18] |
| MS/MS interpretation | partial: evidence only | partial: networking, SIRIUS export [1] | partial: MS-FINDER [4] | no | partial: SIRIUS export [8] | no | yes: MetFrag, SIRIUS [10] | yes: FISh, mzLogic [17] | partial [18] |
| Ion mobility | no (`flow-mzmine.md`) | yes [1] | yes [4] | no [6] | partial [8] | yes [11] | yes [10] | unknown | unknown |
| Batch / saved workflow | yes: `mzflow/2`, `prov/1` | yes: `.mzbatch` [1] | partial: console method file [4] | yes: R script | yes: TOPPAS, Nextflow [8] | yes: document, SkylineCmd [11] | yes: R script | yes: node workflows [17] | partial* |
| Report export | yes: HTML/JSON, Excel, CSV, mzTab-M | partial: tables [1] | partial: tables [4] | partial: tables [6] | partial: tables [8] | yes: custom reports [11] | yes: HTML [10] | yes [17] | yes: Report Studio [18] |
| Scripting / CLI | yes: `mzlab-cli` | yes, with login token [1] | yes: console [4] | yes: R | yes: TOPP, pyOpenMS [8] | yes [11] | yes: R | partial: R/Python node [17] | partial: APIs [18] |
| Runs in a browser, no install | yes | no | no | no | partial: server-side web apps [8] | no | no | no | no |
| Works offline | yes | partial: account, offline licence possible [1] | yes | yes | yes | yes | yes | partial*: licence | yes*, licensed |
| Data stay local | yes | yes; account sign-in for the licence [1] | yes | yes | yes | yes; Panorama optional [11] | yes; SIRIUS web services optional [14] | partial: online mzCloud, optional cloud upload [17] | yes [18] |
| Platform | browser, desktop, CLI | Windows, macOS, Linux [2] | GUI Windows; console all [4][5] | R | all [8] | Windows [11] | R, all [10] | Windows* | Windows [18] |
| Licence and cost | MIT, free | MIT source; free for academia with account, paid plans otherwise [2][3] | LGPL v3, free [5] | GPL, free [6] | BSD, free [8] | Apache, free [11] | GPL v3, free [10] | paid, price not public [17] | paid; trial 30 days [18] |

Other tools read (not in the matrix):
- **ProteoWizard MSConvert**: conversion only; vendor reading on Windows, Linux build for Thermo, Bruker, Waters [12]. mzLab's converter covers the same need for its six families without Windows.
- **GNPS / GNPS2**: web services, upload and login required; accept mzML, mzXML, MGF only; no feature detection of their own [13].
- **SIRIUS**: formula and fragmentation trees run locally; CSI:FingerID, CANOPUS and database search send spectra and trees to a web service that needs an account (free for academia) [14].
- **XCMS Online**: free web service with account; files are uploaded; whether jobs still run was not verified [7].
- **MassCube**: Python package, centroid mzML/mzXML, detection, alignment, gap filling, MS/MS search, statistics; no GUI found [9].
- **El-MAVEN**: GPL v2 desktop tool (Windows, macOS) with aligner, peak detector, isotope and adduct calculators; formats not verified [15].
- **Mass Spec Studio**: structural proteomics (HX-MS), not a small-molecule LC-MS tool; out of scope [16].
- **Thermo FreeStyle**: free viewer for `.raw` with integration, library search and elemental composition, Windows only [17]. **TraceFinder**: Thermo's quantitation and screening tool [17].
- **SCIEX PeakView/MasterView, MultiQuant, Analyst**: qualitative review, screening and quantitation modules; Analyst is at end of support [18].
- **Agilent MassHunter Qualitative/Quantitative, Profinder** and **Waters MassLynx, UNIFI/waters_connect, Progenesis QI**: vendor sites could not be opened; only announcements and support notes were read [19][20]. MassLynx DataBridge exports netCDF without product-ion scans [20].

## 3. Findings
### (a) Ten things users of those tools rely on that mzLab lacks
Ranked by likely value for a university analytical lab. Effort S/M/L follows the repo's architecture: a function needs Rust engine code, a `step/1` descriptor, the `.wasm` and the native side, and a parity test (`parita.md`, `rust-step.md`); S = a new output or reader on existing data, M = a new engine module plus a view, L = new science or a new file format.

| # | Missing | Who has it | Value | Effort | Note from the docs |
|---|---|---|---|---|---|
| 1 | Calibration curve and quantitation: standards, weighting, internal standard, LOD/LOQ, concentration table | Skyline [11], SCIEX OS [18], vendor quant tools | high: the routine result of a triple-quad lab | M | integration and the transition table exist (`dati-integrazione.md`, `transizioni.md`); sample roles exist (`campioni.md`); the regression and its view do not |
| 2 | Reading back SIRIUS results (formula, structure ranks) | patRoon [10] | high for non-targeted identification | S-M | export exists, "No reading of SIRIUS results" (`attributio-export.md`); a table reader like those of `mzfinder-untargeted.md` keeps the rule that no step calls an external program |
| 3 | SCIEX `.wiff2` and TOF files, Shimadzu Q-TOF | SCIEX OS [18]; MZmine reads `.wiff2` [1] | high for labs with those instruments, none otherwise | L | refused today (`wiff-varianti.md`, `lettori-modulo.md`); no reference mzML for any TOF |
| 4 | Transition list / method export for the instrument, collision-energy optimisation | Skyline [11], SCIEX OS [18] | medium-high: closes the loop from screening to a targeted method | S (CSV list), L (native methods) | `export.inclusion` writes an inclusion list (`cli-passi.md`); no MRM list in a vendor or Skyline layout |
| 5 | Statistics in the site with visible p/q values, volcano | CD [17], MZmine [1], XCMS Online [7] | medium | S-M | engine exists; steps are native only and no view shows p-values by decision G1 (`stat.md`) |
| 6 | Search of any local compound database (CSV/SDF) | MetFrag [22], OpenMS [8], MZmine* | medium | S-M | `annot.regulatory` covers lists and suspects only (`flow-mzmine.md`); a molecule database exists (`molecules.md`) |
| 7 | Exports as Flow steps (FBMN, SIRIUS) so that batch and CLI runs produce them | MZmine batch [1], OpenMS TOPP tools [8] | medium: needed for unattended runs | S | interface-only today (`flow-mzmine.md`) |
| 8 | Templated PDF report | SCIEX Report Studio [18], patRoon [10], CD [17] | medium-low | S | printable HTML and JSON exist (`dati-strumenti.md`) |
| 9 | Lipid annotation | MS-DIAL [5], MZmine* | depends on the lab's field | L | "none" (`flow-mzmine.md`) |
| 10 | Ion mobility processing (CCS, mobility-resolved features) | MZmine [1], MS-DIAL [4], Skyline [11], patRoon [10] | low without an IMS instrument | L | the Bruker reader sums mobility (`bruker.md`) |

Also absent but of lower value for a university lab: electronic signatures and a regulated audit trail (vendor suites [18][20]; Skyline keeps an audit log without tamper protection [11]); `prov/1` records a run but is not an audit trail.

### (b) Five strengths worth stating publicly
1. **No installation, no account, no upload**: the site reads six vendor families in the browser and keeps an offline copy (`lettura-nativa.md`, `esporta-dati.md`). Among the tools read, MZmine needs an account [1], GNPS and XCMS Online need an upload [13][7], and vendor reading elsewhere is tied to Windows or to MSConvert for part of the formats [1][8][12].
2. **Triple-quadrupole and non-targeted HR in one free tool**: MRM is a paid module in MZmine [1], a companion program for MS-DIAL [4], and Skyline states it is not meant for non-targeted work [11]. Limit to state with it: no calibration curve yet.
3. **Same bytes in site, app and CLI**, with parity tests and deterministic maths (`parita.md`).
4. **Provenance by default**: plan checked before the run, `prov/1` with SHA-256 of every input and each parameter marked estimated or fixed, `conv/1` listing what a conversion did not carry (`rust-flow.md`, `cli-convert.md`).
5. **Open in both directions**: reads mzMine, FBMN, MS-DIAL and GNPS2 tables; writes mzML, FBMN, SIRIUS `.ms`, mzTab-M, MSP/MassBank (`mzfinder-untargeted.md`, `cli-passi.md`); identifications are shown as evidence with Schymanski levels, never raised automatically (`livelli.md`).

### (c) Interoperability gaps
- **mzTab-M**: written without the SME section (`mzfeat-tabella.md`); jmzTab-m 1.0.6 finds no structural error and 4 semantic errors for empty mandatory fields (`misure.md`). The specification says SMF and SME SHOULD be present [21]; MS-DIAL writes SME when MS/MS is assigned [5] and SIRIUS writes mzTab-M [14]. No mzTab-M reader in mzLab.
- **SIRIUS**: `.ms` and MGF out only; five adducts accepted by the campaign export (`attributio-export.md`); results (`analysis_report.mztab`, TSV summaries [14]) are not read.
- **GNPS FBMN**: `quant.csv` in the MZmine layout, `ms2.mgf`, `iin_edges.csv` are written (`mzfinder-untargeted.md`). GNPS requires the feature id to equal `SCANS=` in the MGF [13]; the docs name `FEATURE_ID` only: to verify in the code and add to the doc.
- **MZmine**: full table read and `full_mzmine` written (`cli-passi.md`); the column names of MZmine's newer modular CSV are not in its docs [1], so compatibility with mzmine 4 exports should be tested on a file.
- **MS-DIAL**: alignment `.txt` read; no export in that layout (low value: FBMN accepts the MZmine layout [13]); the MS-DIAL library format LBM [4] is not read.
- **XCMS, OpenMS, patRoon, asari tables**: adapters exist only in the benchmark tools (`mzfinder-benchmark.md`), not in the program; OpenMS featureXML/consensusXML [8] are neither read nor written.
- **Skyline**: its small-molecule transition list uses columns such as Molecule List Name, Precursor Name, Precursor Formula, Precursor Adduct, Precursor RT, Product m/z [11]; mzLab's transition CSV has its own columns (`transizioni.md`): a converter in both directions is small work. Skyline `.blib` libraries are not read.
- **Libraries**: MSP, MGF, MassBank, MoNA/GNPS/mzMine JSON are read (`match-spettrale.md`); MZmine also reads JCAMP `.jdx` [1]; NIST binary libraries and mzCloud are vendor-side [17].
- **mzML out**: complete for all read families (`cli-convert.md`); this is the entry format of GNPS, SIRIUS, XCMS, MassCube, patRoon and OpenMS [13][14][6][9][10][8]. Conformance and known holes: `mzml-matrice.md`.
- **Docs to align**: `flow-mzmine.md` lists mzTab-M among exports with no step, while `cli-passi.md` and `flow-copertura-tabui.md` describe `export.mztab` as a step.

### (d) Risks
- **Citation**: reviewers expect a citable method. The tools read are cited by a paper: MZmine 3 doi:10.1038/s41587-023-01690-2, MS-DIAL doi:10.1038/nmeth.3393, XCMS doi:10.1021/ac051437y, OpenMS 3 doi:10.1038/s41592-024-02197-7, Skyline (small molecules) doi:10.1021/acs.jproteome.9b00640, patRoon doi:10.1186/s13321-020-00477-w, SIRIUS 4 doi:10.1038/s41592-019-0344-8, GNPS doi:10.1038/nbt.3597, MSConvert doi:10.1038/nbt.2377.
  mzLab has `CITATION.cff` but no DOI («Zenodo DOI: to be assigned at the first release», `README.md`) and no paper. No vendor publishes a how-to-cite page; papers name software and version*.
- **Validation in public**: parity with mzmine 3.9.0 is proved stage by stage on golden data (`attributio-mz.md`) and a benchmark harness against mzmine, XCMS, OpenMS, asari and patRoon exists (`mzfinder-benchmark.md`), but its plan, datasets and results live in the private repository. A user cannot yet read how mzLab's tables compare.
- **Described purpose**: `README.md` and `CITATION.cff` present mzLab as «a browser-based teaching tool» while `CLAUDE.md` calls it a professional analysis program; a reader deciding whether to trust it for a paper sees the first.
- **Single author and young code**: one author in `CITATION.cff`, version 1.0.0 (`Cargo.toml`, `CHANGELOG.md`); the compared open tools have been cited for years (papers above). Reverse-engineered vendor readers can break on new instrument firmware; partial reads are reported (`estendere.md` `losses`), unread content is listed (`grezzi-non-estratti.md`).
- **Quantitation claims**: without calibration, areas are not concentrations; stating «triple-quad support» without that limit invites misuse.
- **Regulated use**: no electronic signatures or audit trail; vendor suites sell these as add-ons [18][20]. mzLab should not be presented for accredited or GxP work.
- **Licence position of others can change**: MZmine requires an account, and a paid plan for non-academic users [1][3]; mzCloud's legacy service retires for older Compound Discoverer versions [17]. A local, MIT-licensed tool is not exposed to this, which is a point users in public bodies may value.
- **Limits of this study**: Agilent and Waters pages were blocked, skyline.ms was unreachable (Skyline facts come from its source repository), MS-DIAL gap filling, MZmine and MS-DIAL viewers and several vendor cells are marked `*` or unknown.

## Sources (read 2026-11-15)
- [1] MZmine documentation: https://mzmine.github.io/mzmine_documentation/latest (pages `data_conversion`, `commandline_tool`, `services/users`, `module_docs/io/feat-list-export`, `tool_integration`, `module_docs/id_spectral_library_search/spectral_library_search`, `module_docs/mrm_mrmtoscans/mrm-to-scans`, `visualization_modules/statistics_dashboard/statistics_dashboard`, `workflows/batch_processing/batch-processing`)
- [2] https://github.com/mzmine/mzmine (README, releases: 4.11.0)
- [3] https://mzio.io/plans/ · https://mzio.io/academic-program/
- [4] MS-DIAL 5 tutorial: https://systemsomicslab.github.io/msdial5tutorial (index, `consoleapp.html`, `utils.html`)
- [5] https://systemsomicslab.github.io/compms/msdial/main.html · https://systemsomicslab.github.io/compms/license/main.html · https://github.com/systemsomicslab/MsdialWorkbench
- [6] https://bioconductor.org/packages/release/bioc/html/xcms.html · https://sneumann.github.io/xcms/reference/index.html · https://github.com/sneumann/xcms
- [7] https://xcmsonline.scripps.edu/xcmsonline/faq.php · https://xcmsonline.scripps.edu/xo-docs/usermanual.html
- [8] https://openms.readthedocs.io/en/latest (`getting-started/file-formats.html`, `vendor-formats.html`, `toppas-get-started.html`) · https://archive.openms.de/openms/Documentation/nightly/latest/html/TOPP_documentation.html · https://github.com/OpenMS/OpenMS · https://github.com/OpenMS/streamlit-template
- [9] https://huaxuyu.github.io/masscubedocs · https://github.com/huaxuyu/masscube
- [10] https://rickhelmus.github.io/patRoon/handbook_bd (`features.html`, `annotation.html`, `report.html`, `sample-analyses.html`) · https://github.com/rickhelmus/patRoon
- [11] https://github.com/ProteoWizard/pwiz/tree/master/pwiz_tools/Skyline (README, `CommandArgUsage.resx`, `Documentation/Tutorials/` SmallMolecule, SmallMoleculeQuantification, SmallMoleculeMethodDevCEOpt, HiResMetabolomics, SmallMoleculeIMSLibraries, AuditLog; `Model/DocSettings/AbsoluteQuantification/`) · https://pmc.ncbi.nlm.nih.gov/articles/PMC7127945/
- [12] https://github.com/ProteoWizard/pwiz (README) · https://pmc.ncbi.nlm.nih.gov/articles/PMC4113728
- [13] https://ccms-ucsd.github.io/GNPSDocumentation/ (`fileconversion`, `faq`, `featurebasedmolecularnetworking`, `featurebasedmolecularnetworking-with-mzmine2`, `featurebasedmolecularnetworking-with-ms-dial`, `librarysearch`) · https://wang-bioinformatics-lab.github.io/GNPS2_Documentation/accounts/
- [14] https://github.com/sirius-ms/sirius · https://v6.docs.sirius-ms.io/io/ · https://v6.docs.sirius-ms.io/account-and-license/ · https://boecker-lab.github.io/docs.sirius.github.io/admins · https://bright-giant.com/
- [15] https://github.com/ElucidataInc/ElMaven
- [16] https://pmc.ncbi.nlm.nih.gov/articles/PMC4391204 · https://chronect.trajanscimed.com/autohx
- [17] https://www.thermofisher.com/xcalibur · https://thermofisher.com/compounddiscoverer · https://www.thermofisher.com/mzCloud · https://www.thermofisher.com/tracefinder · https://documents.thermofisher.com/TFS-Assets/CMD/manuals/FreeStyle1.8SP2_RelNotes.pdf
- [18] https://sciex.com/products/software/sciex-os-software · https://sciex.com/content/dam/SCIEX/pdf/flyers/sciex-os-software-comparison-matrix.pdf · https://sciex.com/content/dam/SCIEX/pdf/brochures/sciex-os-lcms-software.pdf · https://sciex.com/content/dam/SCIEX/pdf/eula/eula-trial-licenses.pdf · https://sciex.com/products/software/peakview-software · https://sciex.com/br/products/software/masterview-software · https://sciex.com/products/software/multiquant-software · https://sciex.com/products/software/analyst-software
- [19] https://community.agilent.com/technical/mass-spectrometry-software/b/announcements/posts/what-s-new-masshunter-qualitative-analysis-12-0 · https://community.agilent.com/technical/mass-spectrometry-software/b/announcements/posts/what-s-new-masshunter-quantitative-analysis-12-1
- [20] https://support.waters.com/KB_Inf/MassLynx/WKB57100_How_to_export_raw_data_files_to_an_open_file_format_using_a_post_run_process · https://support.waters.com/KB_Inst/Mass_Spectrometry/WKB1709_When_will_Windows_10_be_supported_with_MassLynx_42 · https://support.waters.com/KB_Inf/UNIFI/WKB78812_How_to_set_up_MSConvert_with_the_UNIFI_Web_API
- [21] https://hupo-psi.github.io/mzTab/2_0-metabolomics-release/mzTab_format_specification_2_0-M_release.html
- [22] https://ipb-halle.github.io/MetFrag/ · https://systemsomicslab.github.io/compms/msfinder/main.html
