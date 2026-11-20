# MSn user manual: from the `.raw` infusion to the tree, the library and the table

For whoever launches the work from the shell or from a Flow plan; the graph design is in `msn graph.md`, the library in `msnlib.md`. The program gives no verdicts: sub-formulas are candidates with ppm and evidence.

## 1. Command line (`mzlab-cli`)
- **Graph**: `mzlab-cli msn graph INFUSIONE.raw --formula "C13H24N4O3S [M+H]+" --out graph.json`. The file can be a Thermo `.raw` or an mzML; `--formula` accepts the ion (`C13H25N4O3S+`, `C13H24N4O3S+H`, `"FORMULA [M+H]+"`); without it, the graph has only the masses. Useful options: `--activation cid|hcd` (one type only), `--ppm-formula 10`, `--isotopes`, `--pick MZ=ION` (fixes the formula of a node), `--smiles` (written into the JSON). `--summary` prints one line of numbers.
- **Library**: `mzlab-cli msn lib add lib.json INFUSIONE.raw --id ID --formula ION` adds the entry (the same entry twice stays as it is); `msn lib build MANIFEST.tsv` writes one per row (`id`, `neutral_formula`, `expected_adduct`, `file`).
- **Export as a library**: `mzlab-cli msn lib export lib.json --format msp|massbank [--clean] [--all] [--annotate]` (`--clean` keeps the peaks explained by a node; `--all` every MS level; `--annotate` writes formula and neutral loss of each peak).
- **Export as a table**: `mzlab-cli msn graph INFUSIONE.raw --formula ION --export csv|mgf|msp [--clean] [--node nK|all]`.
- **Search**: `mzlab-cli msn lib search lib.json graph.json [--ppm 10] [--tol 0.01] [--min-evidence 1] [--top 10] [--delta Da]` gives `msn-search/1`: three separate scores (MS2 entropy, tree, shared losses) with the evidence, without a verdict.

## 2. Flow steps
Plan `mzflow/2` with the `.raw` (or mzML) input and, in order:
- `spectra.msn_graph@2`: has the port `source` (the infusion), output `graph` (`msn-graph/1`). The compound is given with `formula` + `adduct`, or with `molecule` (name) and the port `molecules` (CSV list of molecules); the other parameters have the names of the `msn graph` flags (`ppm_formula`, `activation`, `isotopes`, `pick`, `smiles`, …); a plan saved at `@1` with the Italian names opens and runs the same.
- `msn.library_export@1` (`graph` → library): `format` msp|massbank, `levels` all|ms2, `clean`, `annotate`, `id` (record name).
- `msn.table_export@1` (`graph` → CSV table): `view` modifications|nodes|peaks, `clean`.
The neutral losses are in the `modifications` table, the sub-formulas in `nodes`, the peaks in `peaks`; the run's "MSn graph" view shows them without opening the CSV. Each parameter has its own `--set INSTANCE.NAME=VALUE` in `mzlab-cli flow run`.

## 3. How to read the result
- Each node is an ion with a proposed formula and ppm; the edge between two nodes is the neutral loss (formula and mass at 5 decimals).
- A fragment with several possible formulas keeps all the candidates: the choice is the user's (`--pick`).
- At unit resolution an m/z is a candidate, not an identification.

## 4. List of the MSn experiments of a DDA file

Code: `mzlab/web/msn-elenco.js` (ES module loaded at first use by `COMP.tree` in `composizione.js`), `DDA.get`/`DDA.showMs2` (`dda.js`). Test: `tests_e2e/e2e_hr_msnelenco.py`.

- **When**: the "MSn tree" window of a file with a `dda` element opens on the "Experiments" tab; the "Tree" tab exists only if the file has MS3 or beyond (the `/api/msntree` tree rejects DDA files with MS2 only) and is computed on first opening. The arrows switch between tabs.
- **Data**: `/api/dda` (Rust `hrroutes.rs`, reference `explore.dda`, parity `hr_parity`) with the columns `lev` (MS level) and `z` (charge, null if missing); the base peak intensity from `/api/scanlist?level=N`. `groups(D, tol)` groups the scans by the precursor of their MS2 ancestor (mode of the m/z within the file's tolerance).
- **Table**: `role=grid`, 24 px rows; in the DOM the visible ones plus one screen above and one below, redrawn when the margin falls below half a screen (fast scrolling shows no empty bands; 50 000 scans in under 0.2 s). Sortable headers (`aria-sort`; the activation column sorts the groups by number of scans), filters for m/z, RT, level and "expand all" (on up to 200 groups). Keyboard: arrows, Page, Home/End, Enter/Space, Left/Right close and open a group (`aria-activedescendant`).
- **Actions**: a click or Enter on a scan closes the window and shows it in the DDA trio (`DDA.showMs2`). Per group: "TIC" = chromatogram of the MS2 tab with `prec` and `ptol` (half of the precursor dispersion + `HR.tolDa`) and its linked spectrum (`addMs2Spec`); "XIC" = `xicDirect` of the m/z on the Full tab with the file's ppm window and a linked spectrum.
- **`ptol`** of `/api/chrom` (Rust `chrom_raw`): precursor window in Da, default 0.6; it is a property of the panel alone (`getChrom`, saved in the notebook), because `ms2_exps` rounds the precursors to 0.1.
- **Trap**: the filters listen only to `input`, not `change` (the `change` on blur rebuilt the rows in the middle of a click).

## 5. Product files at an MSn level other than 2 (MS3)

Code: `iteminfo.rs` (`msn_level`), `axisroutes.rs` (`ms2_transitions(level)`), `itemroutes.rs` (`item_method`), `explore.js` (`lv`, `getMs2Tr`), `titoli.js`, `spettro.js`, `pannelli.js`, `tabs.js`, `eventi.js`, `hr.js`. Tests: `crates/mzlab-core/tests/ms3_only.rs` (feature `sciex`, on the synthetic pair `ms3` of `tools/wiff_riferimento.py`), `tests/test_titoli.py`, `tests_e2e/e2e_ms3.py` (the `ms3` file in `tools/synthetic_data_msn.py` and a second MS3 file with two precursors), `tests_e2e/e2e_ms23.py` and the parity tests of `iteminfo`/`mzidx` (the `ms2_ms3` file).

- **File level**: the info of a file (`item_info`) has `msn_level` = the lowest level among its scans of level ≥ 2 (`null` if it has none); an MS3-only file has `kind` `ms2` and `msn_level` 3. The product m/z range (`mz2_min`/`mz2_max`) comes from the scans of that level.
- **Interface**: the product tab uses `lv` = the file's `msn_level` (2 if missing): precursor chromatogram, scan list, spectrum and precursor table (`/api/ms2tr?level=`, 2 if missing) ask for the section of that level, never an MS2 section the file does not have. The spectrum's level menu offers "MS3" for an MS3-only file.
- **Titles**: with `lv` 3 the titles read "TIC · MS³", "TIC · MS³ (all precursors)", "TIC · MS³ · precursor {prec}", "MS³ of {prec}" (the keys `panel.title.ticMs2`, `ticMs2All`, `ms2prec`, `productOf` have `{ms}` = "MS²" or "MS³").
- **Spectrum of a level > 1** (`spettro.js`): as for MS2, a precursor that the displayed file does not have leads to the loaded file that has it; "all precursors" of a low-resolution file gives one spectrum per precursor; the m/z axis fits the spectrum's peaks and the precursor + 5; the scan row gives the CE if the file states it.
- **Spectrum exports**: file name "spettroMS{n}" (`export.file.spectrumMsn`), description and metadata sheet "MS2"/"MS3" ("MS²"/"MS³") according to the panel's level.
- **Precursor**: of an MS3 scan it is the second isolated mass (the reader's one), as in the written mzML; the first is a reader userParam and does not enter the menus.
- **Method** (`/api/method`): the cycle times of a product file are counted on the scans of its lowest level ≥ 2.
- **File with MS2 and MS3 together**: `file_parts` (`iteminfo.rs`, `explore.py`) splits it into one part per level, `file#MS2` and `file#MS3` (`level` field of the part; `sniff` calls them "MS2" and "MS3"); each entry has its own `msn_level` and its label "· MS2"/"· MS3".
- **List and titles of a mixed file**: the product group in the file list names the levels present ("MS2/MS3 (Product Ion)", `msLv` in `explore.js`, also for the `hr.ms2` group with `files.group.ms2` = "{ms} (Product Ion)"). The chromatogram title uses the level of the files that have its precursor (`titoli.js`); choosing the MS3 precursor brings spectrum and chromatogram to level 3.
- **Out of scope here**: libraries and identification stay on MS2 spectra; the spectrum's MGF, MSP, MassBank and SIRIUS (`workflow.js`, `fair-azioni.js`) are also offered only for MS2, because they write the precursor as the sample ion; the MS2 panels of a DDA trio (`dda.js`) stay at level 2.

## 6. Figure and HTML of the MSn graph (`msn-graph/1` without opening the app)

- **Code**: `crates/mzlab-feat/src/msnfigure/` (feature `figura`; CLI and `.wasm` `msnfigura`): `mod.rs` (static SVG: same tree/radial `view` layout as the page, `completa` if missing, same text rule as `msngrafo-vista.js`: m/z at 4 decimals, % relative to 1, levels, up to 4 candidates (1 in the radial), loss as formula or "Δ x.xxx Da" with "(compatible with …)", rule "ids (weak), +n"; Okabe-Ito colors written in the SVG, no external CSS).
- **Other files**: `html.rs` + `html.html` + `page.js` (standalone HTML), `figure_text` (svg or html of the step); in `crates/mzlab-cli/src/`: `msn_png.rs` (PNG), `msn_outputs.rs` (formats, writing, command `figure`), `flow_msnfigure.rs` (step).
- **Root**: if the graph has a molfile with 2D coordinates the root box draws the molecule (`molecule`, from `mzlab-mol`); with SMILES or molfile but without coordinates it writes "no 2D coordinates"; without a structure, nothing.
- **HTML**: a single file, no external resources, no network requests (CSP in the meta); it contains the JSON, the drawer `msngrafo-vista.js` (without `export`), the Rust SVGs of the two views for the downloads, the data table with "Copy data", Tree/Radial view, zoom, keyboard (Tab on the ions, Enter for the candidates), axe 0 violations; 80 ions ≈ 390 KB (limit 2 MB).
- **PNG**: cargo feature `png` (not default, never in the `.wasm`; `resvg`, MPL-2.0, allowed by `deny.toml`) at scale 2 with the system fonts, so the PNG depends on the installed fonts; without the feature the error is `err.msnfigura.png` with `build=cargo build -p mzlab-cli --features png`. In the browser the PNG is made from the SVG with a canvas (`msngrafo-immagine.js`; the HTML also has "Download PNG").
- **Command line**: `mzlab-cli msn graph FILE --out STEM --figure svg,html[,png] [--view tree|radial]` (not with `--summary`, `--export`, `--structures`) writes `STEM.svg|html|png` next to the JSON; `msn graph figure IN.json [--figure …] [--view …] [--out STEM]` starts from an already made graph (default svg,html).
- **Flow**: step `msn.figure@1` (`graph` report → `figure` report; parameters `format` svg|html|png, `view` tree|radial), SEPARATE from `spectra.msn_graph` because a run gives a single artifact and each format is its own run; the same bytes as the command (test in `msn_figure_tests.rs`); each parameter has its own `--set`; descriptor in `docs/schemi/steps-cli-1.json`.
- **Flow in the browser**: svg and html from the `.wasm` `msnfigura` (`figure_text`, `flow-web-struct.js`), same bytes as `flow run` (`e2e_flow_web_struct.py`); the PNG stays on the computer.
- **Tests**: `msn_figure_tests.rs` (golden files in `tests/msn_figura/`, rewritten with `MZLAB_UPDATE_FIGURE=1`), `tests/msn_figura_parita.js` (the Rust texts = those of the drawer, also on the 80-ion graph of `msngrafo_80.json`), `tests_e2e/e2e_msn_figura.py` (file://, network, keyboard, download, axe).

## 7. Tests of the fragmentation rules (`mzlab-cli msn rules`)

Numbers and criteria only; no spectra. Command: `python3 tools/validate_rules.py` (MassBank 2026.03, Zenodo 19590880, CC0/CC BY records with SMILES; details in `msn-interpret.md`). Each record becomes a one-level graph (`graph_from_peaks`) with its formula and its SMILES; the fragment peaks exclude the precursor's.

### Criteria
- Layers separated by activation and polarity: `cid_trappola` (CID on a trap), `hcd`, `qtof` (CID on TOF, read as beam), `fascio` (CID on other instruments, read as beam). Never mixed.
- Coverage (a) peaks with a sub-formula; (b) more rules than formula alone; (c) more rules than structure. Intensity in % of the base peak. Records with (a) < 0.5 (nominal mass) are counted apart (`nominal`) and stay out of the 10 ppm test.
- Deterministic decoys (per-record seed = FNV-1a of the id): `ds` structure bonds shuffled with order-preserving swaps (same formula, same groups); `dp` spectrum of another record with the same nominal precursor and polarity and a different neutral formula (the same compound at another energy or an isomer does not count); `dl` losses shifted by −0.0137 Da.
- Coverage (c) passes the test if, for every applicable decoy, (c − decoy) ≥ 0.5·c. A rule with fewer than 20 true applications is "not measured". A rule's rate is the maximum, over the applicable decoys, of decoy applications / true applications; `ds` applies only to rules with `site` or `kind` cleavage (a rule that asks for a group or an element passes it by construction). Above 0.30 in a layer with at least 20 applications the rule drops to priority 3 in `fragrules-1.json`, with its why.
- Formula-only rules hold for rare adducts ([M+Na]+, [M+NH4]+, [M+Cl]-, [M+HCOO]-) and coverage (b) stays 0 in [M+H]+/[M-H]- spectra.

### Records and discards
Discarded: activation not read 10897, unsupported ion 250, unreadable SMILES 693, empty spectrum 375.

| layer | pol | records | nominal | (a) | (c) | c−ds | c−dp | c−dl | test |
|---|---|---|---|---|---|---|---|---|---|
| cid_trappola | − | 333 | 19 | 0.995 | 0.217 | 0.129 | 0.218 | 0.217 | yes |
| cid_trappola | + | 2514 | 89 | 0.998 | 0.283 | 0.168 | 0.284 | 0.283 | yes |
| fascio | − | 35 | 137 | 0.757 | 0 | 0 | 0 | 0 | yes |
| fascio | + | 42 | 326 | 0.845 | 0.049 | 0.034 | 0.049 | 0.049 | yes |
| hcd | − | 5742 | 705 | 0.994 | 0.084 | 0.048 | 0.086 | 0.084 | yes |
| hcd | + | 17562 | 1609 | 0.993 | 0.159 | 0.119 | 0.158 | 0.159 | yes |
| qtof | − | 1618 | 405 | 0.952 | 0.079 | 0.044 | 0.074 | 0.079 | yes |
| qtof | + | 5058 | 949 | 0.938 | 0.17 | 0.113 | 0.169 | 0.17 | yes |

The test passes in every layer; in the `fascio` layer (c) is at most 0.05: insufficient data, not a rejected rule. Where (c) is high (CID on a trap, 0.22-0.28) the `dp` and `dl` decoys zero it and `ds` reduces it to about 0.4.

### Rules downgraded to priority 3

| rule | layer, pol, true applications, rate |
|---|---|
| `ion_tropylium` | hcd + 130 0.454 |
| `loss_nh3_amide` | cid_trappola + 30 0.4; hcd + 133 0.361; qtof + 29 0.483 |

The compound pairs ("a+b") follow the rules that compose them. The other rules have a rate ≤ 0.30 or fewer than 20 applications (not measured): they stay at the priority of the table, without evidence in their favor.

### Protocol (fixed rules)
- Set: MassBank Europe at the release fixed by `tools/librerie_pacchetti.py` (SHA-256 checked), CID or HCD records with SMILES and a CC0 or CC BY license; in the repository only accessions and counts, the spectra stay in the `.verifica/` cache. Fragment within 10 ppm (20 below m/z 150), loss within 3 mDa, peaks above 2 % of the base peak; decoy generator in Rust with a fixed seed.
- The documented SMARTS are compared with RDKit only in a local tool, never in CI: the Rust evaluator is the only engine.
