# mzDraw: competitor study (editors, depictors, toolkits)

Report only: no code depends on this file. It studies the 2D structure editors and depiction toolkits that mzDraw (`mzdraw.md`) and `mol draw` (`agent-draw.md`) compete with, to decide what to build first and what to copy as an idea. Formats in depth: `mzdraw-formats.md`; priorities, risks, decisions and ready prompts: `mzdraw-plan.md`.
Sources: official docs, GitHub release pages, npm / PyPI / Maven / crates.io registries, vendor feature charts, read for this study; bundle sizes were measured on the published npm tarballs (`gzip -9`). `*` = not confirmed on a primary source (search snippet, blocked vendor page, or general knowledge): check before quoting it. Versions pin the facts.

## 1. The field in one table
Size = what a web page downloads for the editor or depictor (gzip). "Server" = features that need a remote service.
| tool | kind | version read | licence | size (gzip) | server needed for | layout | reuse verdict |
|---|---|---|---|---|---|---|---|
| Ketcher (EPAM) | web editor | 3.21.0 | Apache-2.0 | Indigo wasm 3.8 MB (11.9 MB raw); no-render 2.0 MB | OCR only (standalone mode) | Indigo | ideas; code compatible but C++/React, too heavy |
| Indigo (EPAM) | C++ toolkit, depict CLI | 1.49.0 | Apache-2.0 | see Ketcher | none | own, fast (41 µs median, `agent-draw.md`) | ideas; not linked (C++) |
| ChemDraw (Revvity) | desktop + JS + SaaS | desktop 26.x* | proprietary | not published | ChemDraw JS: Windows REST service for clean-up, conversion, names | reference quality* | ideas only |
| Marvin JS / Marvin (Chemaxon, Certara) | web editor | 25.3.0 (final major) | proprietary | not published | every format except MRV and MOL V2000; clean 2D; CIP | server | ideas only; end of life 2027-12-31 |
| BIOVIA Draw (Dassault) | Windows desktop | 2026 | proprietary, free academic with registration | n/a | database lookups | own | ideas only; CTfile reference |
| ACD/ChemSketch | Windows desktop | 2025 | proprietary; freeware discontinued | n/a | none | own | ideas only; end of sale (Revvity bought ACD/Labs, Jan 2026) |
| JSME | web editor (GWT) | 2024.4.29 | BSD-3-Clause | 156 KB (425 KB raw) | none (InChI needs an extra module) | own, basic | ideas; code licence-compatible, but GWT output |
| Kekule.js | web editor + widgets | 1.0.4 | MIT | 567 KB (2.66 MB raw); + Indigo/Open Babel wasm for SMILES and layout | none | needs Indigo wasm add-on | ideas |
| ChemDoodle Web | web components | 11.0.0 | GPL-3.0 or commercial | 420 KB raw* | SMILES, names, CDX/CDXML, InChI (iChemLabs cloud) | own | ideas only (GPL) |
| OpenChemLib (+ `openchemlib-js`) | toolkit + editor + depictor | Java 2026.11.1, JS 9.26.0 | BSD-3-Clause | 341 KB (1.10 MB raw, editor included) | none | `inventCoordinates`, good on drug-like* | ideas; already used by the Drawing page for properties |
| PubChem Sketcher | hosted editor | unversioned | NCBI service on CACTVS (commercial) | tiny client | every edit (server renders a PNG per gesture) | CACTVS | ideas only |
| MolView | web app | 2.4.6 | closed since 2023 (chemistry core from old AGPL Ketcher) | n/a | 3D, name lookup (NCI resolver, PubChem) | old Ketcher | nothing (closed) |
| RDKit (MolDraw2D, MinimalLib) | toolkit + depictor | 2026.09.2 | BSD-3-Clause | 2.4 MB (7.39 MB raw) | none | own or CoordGen, oracle of `agent-draw.md` | ideas; test oracle only (`mol.md`) |
| CDK depict | Java toolkit + web service | 2.14.1 | LGPL-2.1-or-later | n/a (Java) | public service `cdkdepict` | own, the 2016 quality reference | ideas only; never copy code (LGPL) |
| Open Babel | toolkit | 3.2.1 | GPL-2.0 | n/a | none | `gen2D`, slow | nothing (GPL); its test files are GPL too |
| SmilesDrawer | depictor only | 2.4.2 | MIT | 59 KB (197 KB raw) | none | own; weak on bridged and complex rings (README) | ideas |
| JChemPaint | Java desktop | 3.4 beta | LGPL-2.1 | n/a | CAS/NSC lookup | CDK | ideas only |
| WebMolKit / SketchEl | web editor (TS) | 2.0.13 | Apache-2.0 | ~1.6 MB unminified | none | own | ideas |
| Butlerov | web editor (TS, Konva) | 0.1.0 | MIT | not measured | none | own | ideas |
| Avogadro 2 | 3D builder | 2.1.0 | BSD-3-Clause | n/a | none | 3D (UFF) | ideas (CJSON); not a 2D competitor |
| chematic-depict (Rust) | crate | 1.2.1 | MIT OR Apache-2.0 | probe 292 KB (`agent-draw.md`) | none | rule-based; E/Z wrong 57/100 (`agent-draw.md`) | closest Rust competitor; ideas |
| cosmolkit (Rust) | crate, RDKit port | 0.6.2 | MIT | probe 712 KB | none | RDKit parity | ideas (parity-with-reference method) |
| omgkit-depict (Rust) | crate | 0.0.8 | BSD-3-Clause | not probed | none | E/Z wrong 46/100 | ideas |
Also seen, no primary source or low relevance: Molsketch (GPL-2.0, Qt), MoleditPy (GPL-3.0), ChemWriter (proprietary, inactive*), Chemozart (3D, 2015).

## 2. The market
- **The free offline editors are disappearing.** Marvin JS has its last sale on 2026-12-31 and end of life on 2027-12-31; Chemaxon ended free individual academic licences in May 2026 and drops XYZ, MOL2 and its own V2000 dialect after LTS 26. ACD/ChemSketch freeware is discontinued after the Revvity purchase. ChemDraw needs a licence portal and its web version a Windows server. What is left that is free, offline and good: Ketcher (heavy) and BIOVIA Draw academic (Windows, registration).
- **Size is the clearest gap.** Permissive full editors weigh 156 KB (JSME, few features) to 567 KB gzip (Kekule, without layout); wasm toolkits weigh 2-4 MB gzip (RDKit, Indigo in Ketcher). The `mzdraw.md` target (< 250 KB gzip) with SMILES, MOL V2000/V3000, KET and CDXML in the browser would be unique.
- **Server dependence is the second gap.** Marvin JS (nearly every format), ChemDraw JS (conversion, clean-up, names), ChemDoodle Web (SMILES), PubChem Sketcher (every gesture), MolView (3D, lookup) all need a server. Files never leaving the computer (CLAUDE.md) is a real differentiator, not a slogan.
- **Not one competitor documents user shortcuts and autosave together**; only ChemDoodle advertises accessibility options (sounds, colour themes, keyboard control); no editor has a CLI that produces the same bytes as the editor.

## 3. Per-tool notes (what to copy as an idea, where mzDraw does better)
### Ketcher (EPAM)
- Reads and writes KET, MOL/SDF/RDF V2000 and V3000, SMILES, CXSMILES, CML, InChI (+AuxInfo; InChIKey write), CDXML, CDX (binary and base64); writes SVG, PNG; macromolecules (HELM, FASTA, sequences, IDT, AxoLabs, BILN). Only KET and RDF keep multi-tailed arrows; KET, CDX and CDXML keep text used as reagents.
- Features: templates and user templates, functional groups and salts as contractable abbreviations (SUP S-groups, not editable inside), R-groups, S-groups, query properties, reactions with mapping, CIP, structure check, clean-up, Miew 3D, OCR (remote), ACS-style render settings (2.26). No user shortcuts, no autosave in the docs; 771 open issues; async start (`window.ketcher` undefined until init).
- Copy: KET as an open JSON document with graphics; "Copy as MOL / KET / image" hotkeys; a functional-group library as first-class abbreviations. Do better: 15x smaller, synchronous start, no iframe, user shortcuts, autosave, same bytes from the CLI.
### Indigo
- Render options are one flat namespace shared by the API and the `indigo-depict` CLI (`render-bond-length`, `render-label-mode` all|terminal-hetero|hetero|none, `render-stereo-style`, `render-superatom-mode` expand|collapse, highlight colour); outputs PNG, SVG, PDF, EMF (Windows), partial CDXML; batch filename template `%s` for SDF input. A "norender" wasm halves the size.
- Copy: one option namespace for library, CLI and settings file; filename templates for batch output. Do better: a Rust vector writer instead of Cairo/FreeType.
### ChemDraw (Revvity)
- Feature chart: magic hotkeys on the hovered atom, label expand/contract, nicknames (user abbreviations), smart paste without overlap, templates (glassware too), generic structures, enhanced stereo, atropisomers, chemical warnings inline, calculations (MW, exact mass, m/z, fragmentation), NMR prediction and name/structure (Pro), style sheets (a new drawing starts from a style; dark style). Read and save CDX, CDXML, SKC, MOL/SDF/RXN V2000+V3000, RDF; save GIF, JPEG, BMP, PNG, TIFF, SVG, EPS; clipboard CDXML, MOL, SMILES, SLN, InChI, OLE (Windows), HELM.
- ACS Document 1996 numbers (bond 14.4 pt, line 0.6 pt, bold 2 pt, bond spacing 18 %, hash 2.5 pt, margin 1.6 pt, Arial 10 pt) are public journal guidance, also used by RDKit and Ketcher (`mzdraw-formats.md` §4).
- Copy as ideas: hover hotkeys, styles as documents, nicknames, smart paste, inline warnings, the m/z and fragmentation calculator next to the drawing (mzLab already has the engines). Do better: free, offline, CDXML read in the browser, exact ACS output to SVG and PDF, an open style file.
### Marvin JS / Marvin (Chemaxon)
- Marvin JS natively reads and writes only MRV and MOL V2000; every other format, clean 2D and CIP need JChem Web Services; image export PNG/JPEG. MarvinSketch desktop (Java) reads and writes many formats and even recognises structures in PDF. The new hosted Marvin has a universal search bar (name, SMILES, template or command in one box), live properties pinnable to the canvas, preset journal styles, partial WCAG 2.2 AA.
- Copy: the universal input bar; pinned live properties. Do better: all formats client-side; Marvin JS users need a replacement by 2027.
### BIOVIA Draw, ACD/ChemSketch
- BIOVIA owns the CTfile family (MOL, RXN, SDF, RDF, XDfile); Draw 2026 handles tetrahedral and axial stereo, Markush, mixtures, polymers, haptic bonds, HELM/BILN, SCSR. Copy: strict CTfile fidelity proven by round-trip tests (V3000 enhanced stereo, S-groups).
- ChemSketch 2025 added copy/paste interoperability with Ketcher and Marvin web apps on purpose. Copy: test clipboard exchange with the other editors, not only file exchange. Opportunity: its former student and teacher users.
### JSME
- One key per atom, ring and group (`3`-`8` rings, `1` phenyl, `a` COOH, `y` NO2, `t` tBu, `ft` CF3); SMARTS query builder; touch with long press; reads MOL V2000/V3000, RXN, SMILES, JME; writes those plus SMIRKS and SVG; one structure or one reaction. Copy: two-letter group keys. Do better: text and arrows, templates, KET/CDXML, accessibility.
### Kekule.js
- Document of mixed objects (molecules, arrows, text, images) with per-object styles; issue checker; stroke recognition for touch drawing. Native SMILES is a writer only; reading SMILES and layout need the Indigo or Open Babel wasm (5.5 / 4.1 MB). Copy: mixed-object document, issue inspector. Do better: SMILES read and layout native.
### ChemDoodle Web
- Accessibility options (sounds, colour themes, keyboard control); electron-pushing arrows with automatic layout; abbreviation expansion. GPL-3.0: never look at or copy code. Do better: client-side SMILES, permissive licence.
### OpenChemLib
- IDCode: a compact canonical string that can carry coordinates, used as a save and clipboard format; fragment/query mode as one editor flag; web component `<openchemlib-editor>`. Trap already recorded: it reorders atoms and drops explicit H (`mol.md`). Copy: one compact text form of the document for the clipboard and URLs (ours: `mzdraw/1` JSON, base64url-compressed in a link). Do better: richer UI, arrows and text, no GWT.
### PubChem Sketcher, MolView
- PubChem: a status line where any identifier (SMILES, InChI, CID) is typed or pasted and round-trips; every gesture is a server round trip with a 4 KB PNG back. MolView: sketch then 3D then database lookup in one flow; closed since 2023. Copy: the identifier line (merged with Marvin's universal bar; in mzLab the name lookup is the molecule database). Do better: local and instant.
### RDKit (depiction)
- MolDraw2D: SVG and Cairo PNG; ACS1996 mode (`DrawMoleculeACS1996`, 2022.09); options for fixed bond length, line width, comic mode, atom/bond notes, legends, highlights (circle-and-line or lasso), palettes (default, BW, Avalon, CDK); JSON options shared by Python, C++ and JS; molblock and reaction metadata embedded in PNG (since 2020.09) and SVG; ring-system templates as an external community CXSMILES file; CDXML reader (2022.09), CDX reader and ChemDraw document read/write in an optional build (2025.09), CDXML writer (2026.09).
- Copy: JSON options as the one API; images that carry their molecule; ring templates as a data file anyone can extend. It stays our oracle in `tools/` only.
### CDK depict
- Builder API with physical units (mm vector, px raster); standard generator parameters (bond separation, stroke ratio, symbol margin, wedge ratio, hash spacing, "fancy" wedges, outer-glow highlight, annotation distance and scale); abbreviations from an editable "SMILES label" list, contracted only when more atoms are inside than outside, `contract on hetero` (NH-Boc to NHBoc); relaxed SMILES that draws a partial input with a warning (2.14); hydrogen display modes minimal|input|smart|chiral|explicit; URL parameters equal to the library options (`cdkdepict`).
- LGPL: read the documentation and papers, never the code; CDK test files are not copied. Copy as ideas: the abbreviation list format and contraction rule, the hydrogen modes, partial drawing with a warning (fits "the user decides").
### Open Babel
- 146 formats in the docs; SVG writer flags for grids of many molecules and `-e` to embed CML in the SVG; `gen2D` slow (25 min on one ChEMBL case in the 2016 benchmark). GPL: nothing is reused, test files included.
### SmilesDrawer
- 59 KB gzip; renders every element with a `data-smiles` attribute; themes (light, dark, oldschool, solarized); reaction SMILES. Copy: auto-render of an attribute (useful for mzLab tables and reports), theme names. Do better: bridged and macrocyclic layout, molfile input with coordinates.
### Rust crates
- `chematic` / `chematic-depict` (one owner, 150 releases in 6 months): SVG, PDF via `svg2pdf`, EPS, PNG via `tiny-skia` (not in wasm), highlights, reaction SVG; E/Z drawn wrong on 57 of 100 (`agent-draw.md`). `cosmolkit`: an RDKit port with pinned parity tests against RDKit 2026.09.1, its method is the one to copy. `opensmiles`, `purr`, `sdfrust`, `pdbtbx`: parsers, none needed (`mzlab-mol` has its own).
- Output stack (pure Rust, permissive, maintained): `resvg`/`usvg` 0.50 (Apache-2.0 OR MIT) for PNG, already planned behind the `png` feature (`agent-draw.md`); `svg2pdf` 0.13 or `krilla` 0.8 (MIT OR Apache-2.0) for PDF; `lopdf` (MIT) low level. Native only; the page writes PDF with our own small writer on the display list or not at all (decision 4 in `mzdraw-plan.md`).

## 4. Layout quality and speed: the published bar
- Benchmark sets: Clark, Labute, Santavy, "2D Structure Depiction", J. Chem. Inf. Model. 2006 (28 structures, 10 obstacles); Mayfield, RDKit UGM 2016 slides "Higher Quality Chemical Depictions" (48 structures, 11 obstacles: avoidable and unavoidable overlaps, chain blocks, cis/trans, congested small rings, bonded counterions, spiro centres, macrocycles, ring templates, planar embedding, 3D ring systems). Rules: IUPAC 2008 "Graphical Representation Standards for Chemical Structure Diagrams" (Brecher, Pure Appl. Chem. 80(2)).
- 2016 timings on the 41 "fair" structures, mean per molecule*: RDKit 2 ms, Avalon, CDK, Indigo 1 ms, Open Babel 46 ms; on the whole 48 the slowest case took RDKit 10 s and Open Babel 25 min. Today (`agent-draw.md`, set A): 10-181 µs median for layout alone. No recent peer-reviewed head-to-head benchmark was found*.
- Consequence for mzDraw: microseconds per small molecule is achievable (chematic reaches 10 µs median) but only with a hard cap per molecule and a fallback for pathological cases; the quality gate is the 11 Mayfield obstacles plus set A, with RDKit as the oracle (ready prompt P3 in `mzdraw-plan.md`).

## 5. Licence verdict
| class | tools | what we may take |
|---|---|---|
| permissive (BSD, MIT, Apache) | Ketcher, Indigo, RDKit, OpenChemLib, JSME, Kekule.js, SmilesDrawer, WebMolKit, Butlerov, CoordGen, Avogadro, the Rust crates | ideas, documentation, published formats (KET schema); permissively licensed test files with attribution (`mzdraw-formats.md` §3). No code is copied either: the decision is "own Rust" (`agent-draw.md`) and the `mzdraw` crate keeps its own dependency guard |
| copyleft (GPL, LGPL, AGPL) | ChemDoodle Web, CDK, JChemPaint, Open Babel, Molsketch, old GGA Ketcher | ideas from documentation and papers only; never read their code, never copy their test files |
| proprietary | ChemDraw, Marvin, BIOVIA Draw, ChemSketch, PubChem Sketcher (CACTVS), MolView | ideas from public docs; no resource (icons, templates, wording, style sheets) is copied; published numbers such as the ACS 1996 settings are journal guidance, not their resource |

## 6. What to copy as an idea (ranked by value for a chemist)
1. Hover hotkeys and one-key groups (ChemDraw, JSME): type `N` over an atom, `a` for COOH; the fastest drawing there is.
2. Images that carry their molecule (RDKit PNG/SVG metadata, Open Babel `-e`): a figure pasted in a report can be reopened and edited (`mzdraw-formats.md` §2).
3. Styles as documents with journal presets in physical units (ChemDraw, RDKit ACS1996, CDK units): the same file drives the editor, the page and `mol draw`.
4. Abbreviations from an editable list with a contraction rule (CDK, Ketcher functional groups, ChemDraw nicknames): user-defined groups such as the lab's protecting groups.
5. One input bar for name, SMILES, InChI, template or command (Marvin, PubChem): in mzLab the name goes to the molecule database.
6. Inline chemical warnings, never blocking (ChemDraw, Kekule issue checker): `check/1` marks while drawing.
7. Live calculated values pinned to the canvas (Marvin): formula, exact mass, m/z of chosen adducts, already in `label/1`.
8. Copy-as hotkeys and tested clipboard exchange with other editors (Ketcher, ChemSketch).
9. Partial drawing with a warning for a damaged input (CDK 2.14).
10. Ring and template library as an external data file (RDKit templates, CoordGen user templates).
11. Mixed-object document (Kekule, KET): molecules, arrows, text, annotations in one file.
12. Auto-render of `data-smiles` elements (SmilesDrawer) for mzLab tables and reports.

## 7. Where mzDraw can be better than all of them
- One Rust code path for the editor (wasm), the app and the CLI: the same document and settings give the same SVG bytes everywhere; no competitor has this.
- Size under 250 KB gzip with layout and the main formats; synchronous start; works offline on a tablet.
- An open, versioned document format with an import report that lists what was lost, for every reader.
- Identification annotations (Schymanski level, evidence, scores) drawn under the structure: no editor has a slot for them (`mzdraw-formats.md` §5).
- Keyboard-first editing with a text description for screen readers, user shortcuts and autosave.
- A published benchmark (speed and quality, Mayfield obstacles + set A) with the method in the repo.
