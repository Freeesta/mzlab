# ChemDraw feature survey and what mzLab's Draw page should take from it

Study for Federico (mass spectrometry teaching + HR non-targeted screening). It describes features. It copies no ChemDraw text, icons, assets or file-format claims. ChemDraw is a trademark of Revvity Signals Software. It is named here only to refer to the product.

## Sources
Short keys used in the table:
- **[DOT]** Revvity, *Full Dot Chart – ChemDraw Suite of Products* (May 2026), the feature list of ChemDraw Prime, Professional and Signals ChemDraw: https://assets.revvitysignals.com/asset/0b9656e8-339d-47ab-9834-6c08beb6abee/Full-Dot-Chart-ChemDraw-Suite-of-Products.pdf
- **[UG17]** *ChemDraw 17.1 User Guide* (PerkinElmer), university mirror: https://uni-ulm.de/fileadmin/website_uni_ulm/nawi.chemie/Downloads/ChemOffice/ChemDraw.pdf
- **[RN23]** *ChemDraw Suite 23.0 Release Notes*, university mirror: https://www.mn.uio.no/kjemi/tjenester/it/hjelp2/chemdraw/chemdraw-suite-v23.0-release-notes.pdf
- **[CD8]** ChemDraw 8 online help (legacy; spectrum import, mass fragmentation): https://people.stfx.ca/blynch/CS%20ChemOffice%202004/CS%20ChemDraw/CS%20ChemDraw%20Help/CD8ch814.html and https://people.stfx.ca/blynch/CS%20ChemOffice%202004/CS%20ChemDraw/CS%20ChemDraw%20Help/CD8ch1130.html
- **[R255]** Signals ChemDraw 25.5 release announcement: https://support.revvitysignals.com/hc/en-us/articles/43934842634004-Release-Announcement-Signals-ChemDraw-25-5
- **[JS]** ChemDraw JS sketcher flyer: https://revvitysignals.com/sites/default/files/2024-05/rs-flyer-chemdraw-javascript-sketcher.pdf

Tiers ("Prime" or "Pro") come from [DOT]. The text extraction of that PDF loses the dot columns, so the tier was read from the dot count. Treat each tier as indicative.

mzLab side:
- `docs/agenti/disegno.md` (Draw page).
- `mol.md` (`mol/1` record, SMILES/molfile readers, stereo sites, canonical hash).
- `msn-grafo-strutture.md` (cuts per MSn node).
- `attributio-revisione.md` (fragmenter, isotopes, review evidence).
- `alta-risoluzione.md` (elemental composition, isotope score).
- `reattivita.md` (3D viewer of Tabui).

Ketcher side: the vendored build in `mzlab/web/vendor/ketcher`, checked by grep only.

## Legend
- **mzLab today**: yes / partly / no, with where. "Hidden" means Ketcher has the feature, but `TOOLS` in `draw.js` hides its group by default (stereo, S/R groups, check/3D, mapping, generic atoms, aromaticity, biology).
- **Ketcher**: what the vendored editor already does with no mzLab code.
- **Value** for an MS teaching + HR screening site: H / M / L.
- **Effort**: S (one PR) / M (one PR + parity tests) / L (series).
- **Risk**: the teaching-principle risk.
  - LR = low-resolution world: the site never gives answers.
  - HR = the site gives scored candidates with evidence.

## Comparison table

| ChemDraw feature | What it does | mzLab today | Ketcher natively | Value | Effort | Teaching risk |
|---|---|---|---|---|---|---|
| Bond, chain and ring tools, hotkeys [DOT][UG17] | Drawing atoms and bonds with the mouse and the keyboard | yes (Ketcher; drop-down bar `disegno-barra.js`, touch and pen `touch.js`) | yes | H | – | none |
| Templates, functional groups, clipart (glassware) [DOT] | Ready-made fragments | partly (Ketcher rings and template library; no MS-specific set) | yes (template library, functional groups, salts and solvents) | M | S | none |
| Nicknames, Expand/Contract Label [UG17] | Abbreviations such as Boc, TMS or Ac stand for groups and can be expanded | partly (Ketcher superatoms; not surfaced in the bar) | yes (abbreviation expand/contract) | M | S | none |
| Text, captions, shapes [DOT] | Free text and shapes on the canvas | yes (Ketcher text, shapes, images) | yes | M | – | none |
| Style sheets (journal style, dark style) [DOT][UG17] | Fixed bond length, fonts and line widths for publication | partly (atom font size `disegno-font.js`) | partly (settings for bond length and font; no named presets) | M | S | none |
| Clean Up Structure [UG17] | Tidies bond lengths and angles | yes (Ketcher layout and clean) | yes | M | – | none |
| Check Structure, "explain this warning" [UG17] | Flags valence, charge, isotope and stereo-bond problems | no (Ketcher check is hidden; mzLab warns only about pasted names) | yes (Indigo check, hidden) | H | S–M | none: it reports mistakes in the drawing, not answers about data |
| Analysis window: formula, exact mass, average MW, m/z, elemental analysis [UG17][DOT] | Live values for the selection, which can be pasted as a caption | partly (labels `disegno-etichette.js` from `etich/1` in Rust: formula, exact and nominal mass, m/z if charged; OpenChemLib estimates of logP and TPSA; no average MW, no % composition) | yes ("Calculated values": formula, MW, exact mass, elemental analysis) | M | S | LR: labels are optional and editable; the student does the arithmetic (`#lb-a` off by default) |
| m/z with an isotope-abundance graphic [UG17] | Isotope pattern of the drawn formula | no on the Draw page (the Rust isotope engines exist: `isoscore.rs`, `review::isotopes`) | no | H | M | LR: show it only when the student asks, as theory and not a fit to data; HR: already scored in Composition |
| Mass Fragmentation tool (cut line, fragment formulas and masses) [UG17][CD8] | Draws a cut through bonds and labels each piece | partly (right-click on a bond `disegno-frammenti.js`, `frag/1`); the full tool is **in progress (DRAW-FRAG)** | no | H | – | covered by DRAW-FRAG |
| Radical, radical cation, lone pair symbols [UG17] | Charge, radical and lone-pair annotations | partly (Ketcher charge and radical atom property; radical dots **in progress (DRAW-FRAG)**) | partly (mono- and diradical, charge; no lone-pair glyph) | H | S | none |
| Electron- and radical-pushing curved arrows [UG17] | Arrows that show the movement of electron pairs and of single electrons | no | no (only reaction arrows; grep finds no electron-pushing tool) | H | M | none: the student draws the mechanism, and the tool only checks it is well formed |
| Orbitals tool [DOT] | s, p and σ orbital lobes | no | no | L | M | none |
| Reaction arrows, plus, reaction interpretation [DOT] | Reactant, reagent and product roles | yes (Ketcher arrows; optional Δm label above an arrow) | yes (many arrow types, multitail, retrosynthetic) | M | – | LR: the Δm label is off by default |
| Stoichiometry grid [UG17] | Equivalents, mmol and mass table | no | no | L | M | none |
| Atom-to-atom mapping [UG17] | Map numbers in reactions | hidden | yes (auto map) | L | S | none |
| Show Stereochemistry (CIP R/S, E/Z) [UG17] | Draws CIP labels | no (`mol/1` finds stereo sites but does not compute CIP labels); Ketcher stereo is hidden | yes (Indigo CIP labels) | M | M (CIP in Rust) | teaching point: MS does not see stereo; the labels must say so |
| Enhanced stereo (abs/and/or) [UG17] | Racemic and relative groups | partly (`mol/1` groups, molfile V3000 reader) | yes (hidden) | L | S | none |
| Query atoms and bonds, atom lists, Markush, variable attachment [UG17] | Generic structures for searching | partly (multicentre `MZLAB_ENDPTS` and partial structures with Δ in `disegno-parziali.js`) | yes (atom lists, query properties, R-groups, generic atoms; hidden) | M | – | HR: a partial structure is level 3, already encoded |
| R-group table [DOT] | Enumerate substituents | no | partly (R-group definition; no enumeration table) | L | M | none |
| Polymer brackets, SRU [DOT] | Repeating units | no | yes (S-groups: polymer, multiple, superatom, data) | L (M for polymer and surfactant series: PEG, PFAS homologues) | S | HR: homologue series belong to the KMD tools |
| Struct=Name / Name=Struct [UG17] | IUPAC naming and name parsing (Pro) | partly (name to structure through the molecule database `disegno-cerca.js`; no structure to name) | no naming (Indigo: none) | M | L (IUPAC naming) / S (lookup) | none if it is a lookup of the drawing's identity, never of the data |
| ChemNMR ¹H/¹³C prediction [UG17] | Additivity-rule shift estimates (Pro) | no | no | L | L | out of scope for an MS site |
| Chemical Properties: logP, pKa, tPSA, bp, mp, ClogP [UG17] | Predicted physicochemical values (licensed engines for ClogP and pKa) | partly (Crippen logP in Rust `mzlab-mol`; OpenChemLib estimates labelled as estimates) | no | M | S | HR: RT-logP is already evidence in Attributio; keep the word "estimate" |
| Spectrum import (JCAMP) as an object [CD8] | Spectrum drawn next to structures | no on the Draw page (spectra live in Data and in the MSn figure `msn-figura`) | no | H | M | LR: the annotation is the student's; HR: the evidence is the scored candidate's |
| TLC plate tool, gel tool [UG17][DOT] | Drawn TLC plates and gels | no | no | L | M | none |
| Biopolymers (HELM, peptide, DNA) [DOT] | Sequence drawing (Pro) | no (Ketcher macromolecule mode disabled on purpose, `ketcher_senza_monomeri.py`) | yes (disabled) | L | – | none |
| ChemFinder, database and substructure search [DOT] | Search structures in local databases | partly (text search of the molecule database: name, synonym, formula; no structure search) | no search engine (SMARTS query export only) | H | L | HR: substructure hits are suspects, not identifications |
| SciFinder and Reaxys search [DOT] | Online search from the drawing | no | no | L | – | conflicts with "files never leave the computer" |
| Chem3D, Get 3D model, MM2 [UG17][DOT] | 3D model and minimization | partly (Tabui 3D viewer `viewer3d.js`, conformers in `mzlab-chem`) | yes (Miew 3D viewer, hidden) | L | – | none |
| Structure Perspective [DOT] | Pseudo-3D tilt of a drawing | no | no | L | S | none |
| File formats: CDX, CDXML, MOL/SDF/RXN V2000/V3000, SMILES, InChI(Key) [DOT][RN23] | Read and write formats | partly (export PNG, JPEG, SVG, .ket; SMILES copy; V2000 and V3000 written by `disegno-molfile.js`; `mol/1` reads SMILES and molfile in Rust) | yes (mol, rxn, SDF, SMILES, InChI, InChIKey, SMARTS, CML, CDXML, CDX through Indigo) | M | S–M | none |
| Image export (PNG, SVG, EPS, TIFF) [DOT] | Figures | yes (PNG, JPEG, SVG with brackets, Δ and multicentre) | yes (PNG, SVG) | M | – | none |
| Atom numbering, highlight colouring [DOT] | Numbers and coloured atoms or bonds | partly (`TPDraw.mark` selects bonds for the MSn fragment pane) | yes (atom numbers in settings; highlight API) | M | S | HR: the colour must mean evidence; in the LR world only what the student marked |
| ChemScript, ChemDraw JS API [DOT][JS] | Scripting and embedding | yes in spirit (`mzlab-cli`, `TPDraw.*`) | yes (Ketcher API) | – | – | none |
| Cloud, collaboration, AI suggestions [RN23][DOT] | Cloud saving, notebooks, planned reaction-condition AI | no | no | L | – | conflicts with "files never leave the computer" |

## Prioritised integration proposal (12 items)
Every item follows the rules in CLAUDE.md:
- Logic goes in Rust: `mzlab-feat` chem, `mzlab-mol` or `mzlab-core`, with the same bytes from the `.wasm`, the desktop app and `mzlab-cli` (`--set` for Flow steps). The parity test is part of the same PR.
- JavaScript only draws.
- Texts come from `lang/it.js` and `lang/en.js`.
- No new heavy dependency.
- Each chart ships with its `TDATI` table.
- The tour stays valid.

The radical dots, the cut line and the separated pieces are not repeated here: they belong to the DRAW-FRAG chat.

1. **Structure check in Rust** (S–M, value H).
   - The check reports valence, charge, isotope and stereo-wedge errors for the drawing. Each warning gets a `draw.check.*` key with three parts: the problem, why it matters, and the next step.
   - A right-click item "Why?" explains the warning.
   - The check runs on the `mol/1` record (`validate` plus a valence table). It is exposed as `labels`-style JSON in the `grafo` `.wasm`, and as `mzlab-cli mol-check`.
   - For students it catches wrong valences before any mass is computed. Ketcher's own Indigo check stays hidden, because it is JavaScript-side logic.
2. **Analysis card: average MW, elemental % and an on-demand theoretical isotope pattern** (M, value H).
   - The card extends `etich/1` with average mass and % composition.
   - It adds a stick chart of the isotope pattern from `mzlab_core::nuclides`, with the abundance table in `TDATI` and Okabe-Ito colours. The CLI gives the same bytes.
   - In the LR world the pattern opens only on a click, and is titled as theory for the drawn formula, never compared with a spectrum.
   - In the HR world a link sends the formula to Composition, where the scoring already lives.
3. **Mechanism notation: curved arrows for electron pairs and single electrons, lone pairs, and a "•+" radical-cation label** (M, value H).
   - The arrows are an overlay like `parentesi.js`, stored as Ketcher DAT S-groups so they survive `.ket` and the image export.
   - Rust checks that each arrow starts at a lone pair, a bond or a radical, and ends at an atom or a bond. The check never says which mechanism is right.
   - This is how McLafferty rearrangements and α-cleavages are taught. Coordinate the symbols with DRAW-FRAG.
4. **Drawing → catalogue identity** (S, value M).
   - The canonical hash `can()` and the skeleton of the drawing are looked up in the molecule database, and the matching entries are listed with their names: "this drawing is catalogue entry X".
   - It works offline. It identifies the drawing, not the data, so it is safe in the LR world too. This replaces Structure-to-Name without an IUPAC engine.
5. **Substructure search from the drawing** (L, a series of 3 PRs, value H for HR).
   - PR 1: a VF2-style matcher on `mol/1`, with atom lists from the Ketcher query atoms.
   - PR 2: search of the molecule database, libraries and suspect lists, with the CLI command `mol-search --sub`.
   - PR 3: a Flow step that flags features whose candidates contain the moiety (for example all sulfonamides, or all chlorophenols).
   - Every hit is a suspect with its evidence in the HR world. The search is not offered as a data search in the LR world.
6. **SDF and MOL export with mzLab fields** (S–M, value M).
   - A Rust writer from `mol/1` adds data items: formula, exact mass, m/z of the chosen ion, Δ and level of a partial structure, and the hash.
   - The same file comes from `mzlab-cli mol-export`.
   - The writer emits plain V2000 and V3000 molfiles. CDX and CDXML stay with Ketcher, with no compatibility claim.
7. **Structure and spectrum figure** (M, value H).
   - The figure puts the drawing and an MS² spectrum (from Data, MGF or a library) on one SVG. The student's chosen cuts link pieces to peaks with leader lines. The peak table goes in `TDATI`.
   - It reuses `msn-figura` and the fragment JSON (`frag/1`, `strutture`).
   - In the LR world only the links the student draws appear. In the HR world the candidate cuts with their ppm come from the fragmenter, labelled as candidates.
8. **Reaction arrow → transformation rule** (S–M, value M–H).
   - For a parent → product arrow, a right-click item "Save as transformation" writes Δformula and a name into the user's `liste/1` transformation list, ready for Tabui and suspect screening.
   - The Δ is computed in Rust (`parse_delta`, `fragments.rs`). The arrow label stays off by default for students.
9. **CIP labels with an MS caveat** (M, value M).
   - R/S and E/Z labels are computed in Rust on `mol/1` from the `stereo_sites` already there. Golden data come from RDKit, as for the other `mol/1` tests.
   - An optional toggle shows the labels, with the note "mass spectrometry cannot tell these apart". This is a teaching point, not a tool for answers.
10. **Publication style presets** (S, value M).
    - Two or three named presets, for example "Journal, single column", "Slide" and "Poster". Each sets the Ketcher bond length, font, line width and export size in one click.
    - The settings are only interface configuration, so the logic rule does not apply. The presets use our own names and values, not ChemDraw's style sheets.
11. **MS-relevant templates and abbreviations** (S, value M).
    - A small template set: derivatisation groups (TMS, TBDMS, PFB, dansyl), common counter-ions and labelled internal-standard motifs (D3-methyl, ¹³C6-phenyl).
    - The set is surfaced in the bar, with Ketcher's native expand/contract of abbreviations, so the formula and mass labels stay correct.
    - It is data only: a JSON file next to the catalogue.
12. **Evidence highlighting** (S, value M).
    - Atoms and bonds are coloured in Okabe-Ito colours on a request from other pages: cuts supported by peaks (MSn graph, Attributio), reactive sites (`reatt/1`), or a moiety matched by item 5.
    - A legend and a table list the atoms, so colour is never the only channel. In the LR world it colours only what the student selected.

Order of work:
- Items 1 to 4 are independent and can start now.
- Item 7 should follow DRAW-FRAG.
- Item 12 benefits from 5.

## Not proposed
- NMR prediction, the TLC and gel tools, the stoichiometry grid, biopolymers and HELM, orbitals, and Chem3D-style minimization: they have low value for MS. Several would need heavy engines.
- IUPAC structure-to-name generation: large effort. Item 4 covers the need offline.
- Online database search (SciFinder or Reaxys style) and cloud collaboration: files never leave the computer.
- Licensed property engines (ClogP or pKa from commercial vendors): proprietary, and not light.

## What we must NOT copy
- **No assets**: no ChemDraw icons, toolbar images, cursors, palettes, clipart, templates, style-sheet files or sample documents. Templates and presets are drawn and named by us.
- **No text**: no ChemDraw help pages, tooltips, menu labels or warning messages. Our wording lives in `lang/` and is written by us. Generic chemistry terms ("clean up", "cut line") are fine.
- **No trademark use as a feature name**: no "ChemDraw-style", "ChemDraw-compatible" or "Struct=Name" in the interface or in the docs aimed at users. Product names may appear only in study documents like this one, to refer to the product.
- **No format claims**:
  - CDX and CDXML read and write come from Indigo inside Ketcher. They are not ours and are not certified, so we never claim fidelity or compatibility with ChemDraw files.
  - mzLab writes and documents only open formats: molfile V2000/V3000, SDF, SMILES, InChI and `mol/1`.
  - No reverse engineering of `.cdx` in Rust.
- **No proprietary algorithms**: no reimplementation of licensed predictors (ChemNMR rules, BioByte ClogP, MOSES pKa) from their documentation. Our estimates come from published open methods (Crippen) and say "estimate".
- **No style-sheet names of third parties**: journal formatting is described by its numbers (bond length, font), not as a copy of a vendor file.
