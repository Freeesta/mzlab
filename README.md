# mzLab

A teaching program to explore LC-MS data and look for the **transformation products (TP)** of a pollutant degraded over time (triple-quadrupole experiment of the pollutant analysis laboratory, UniTO). The student chooses what to extract, integrates, annotates and draws: the program shows the data, **it does not give the answers**.

## Use

Open **https://freeesta.github.io/mzlab/** in a recent browser (Chrome, Edge, Firefox, Safari) on a computer or tablet. Nothing to install: the calculation runs **inside the browser**. The first time about 10-15 MB are downloaded; after that it also works without internet and it can be installed as an app (Chrome/Edge: «Install» in the address bar; Safari: Add to Dock or to the Home screen).

- **Your files never leave your computer**: the page sends them to no server and collects no visitor data. Session, drawings and progress stay in the browser's storage and are restored when you reopen the page (same computer, same browser).
- Drag the `.mzML` files (and, if you like, the method `.dam`), check times and types, press **Load data**. With no files: **«Try the example files»**.
- Thermo `.raw`, SCIEX `.wiff` (with its `.wiff.scan`) and Agilent `.D` folders also open directly, in the site and in the desktop app. Waters (`.raw` folders), Shimadzu (`.qgd`/`.lcd`) and Bruker (`.d` timsTOF/TSF folders) files are supported through dedicated native Rust reader modules (`crates/mzlab-lettori/`), fetched on demand in the browser or built into the desktop app.
- **On a smartphone** Data and Draw should be used from a computer or tablet.

## The tabs

- **Data**: chromatograms (TIC, BPC, XIC, MRM), spectra, RT×m/z map, separate Full Scan / MS2 / MRM tabs, high-resolution files (Orbitrap, Q-TOF) and DDA; peak integration with a table exportable to Excel (.xlsx), background correction, m/z calculator, periodic table, adducts, neutral losses, comparison of MS2 with spectral libraries. The tables and charts of the report are built in Excel.
- **Draw**: Ketcher chemical editor for molecules, fragments and transformation pathways, with formula and mass under each structure and Δm above each arrow; the visible tools are chosen in «Editor tools»; export to PNG, JPEG, SVG and .ket.
- The **«i»** at the top opens the information.

With unit resolution **an m/z is a candidate, not an identification**.

## Development

Everything needed by whoever modifies the program (rules, architecture, tests, decisions taken) is in [`CLAUDE.md`](CLAUDE.md) (core) and in [`docs/agenti/`](docs/agenti/) (one file per area). In short: for ordinary use of the static site or of the tests no compilation is needed (classic JavaScript and ES modules, Python for the test suite); for development of the Rust engine (`crates/`) it is compiled to WebAssembly for the web or to a native binary for the desktop version and the parity tests. `python3 tools/verifica.py` runs all the checks (JS syntax, pytest, browser tests with Playwright); the site is built with `python3 tools/build_site.py` and is published automatically on every push to `main`.

## Use of artificial intelligence

The code, the tests and the technical documentation are developed with the assistance of generative AI tools (Claude Code, Anthropic), directed by the author; commits written with that assistance list it as co-author. What enters `main` is decided by the maintainer, and only after the mandatory automated tests. The program itself, when you use it, **contains and calls no AI model**: all the calculation happens in the browser (or in the desktop app) with the rules written in the code.

## Licences and credits

The code is under the **MIT** licence (`LICENSE`, Federico Cristaudo). Third-party material, each with its own licence (texts in [`LICENZE-TERZI.md`](LICENZE-TERZI.md), also copied into the site):

- **Ketcher** 3.18.0 (EPAM Systems), Apache-2.0: `mzlab/web/vendor/ketcher/`, https://github.com/epam/ketcher
- **OpenChemLib JS** 9.25.1 (Zakodium / Actelion), BSD-3-Clause: `mzlab/web/vendor/openchemlib.js`, https://github.com/cheminfo/openchemlib-js
- **Example files** (`mzlab/web/esempi/`): synthetic data, CC0 1.0.
- **Element masses and abundances** (`elements.js`): masses from OpenChemLib, atomic weights from Ketcher, IUPAC abundances.

## How to cite

If you use mzLab in the course or in a publication, cite the software with the data of [`CITATION.cff`](CITATION.cff) (GitHub's «Cite this repository» button formats it):

> Cristaudo, F. *mzLab: a browser-based teaching tool for LC-MS data analysis and the search for transformation products*. University of Turin. https://freeesta.github.io/mzlab/ (code: https://github.com/Freeesta/mzlab, MIT licence). Zenodo DOI: to be assigned at the first release; cite the concept DOI for the software in general and the version DOI for a given version.

## Known limits

- Not yet validated against the instrument software on several compounds.
- MS-Numpress compression in mzML is supported through the open-formats module (`crates/mzlab-lettori/`).
- Tested thoroughly with Chromium; Firefox and Safari only with the essential automated tests.
