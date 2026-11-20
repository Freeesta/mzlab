# mzDraw: formats, own document format, settings file (study)

Report only: no code depends on this file yet. It is the full format conformity matrix of mzDraw and `mol draw` (`mzdraw.md` holds the short version and the build slices), the proposal of the document format `mzdraw/1`, of the settings file `mzdraw-settings/1` and of the per-structure annotation slot. Competitors: `mzdraw-competitors.md`; priorities, decisions and ready prompts: `mzdraw-plan.md`.
Sources: the specifications linked in §1, the Ketcher and RDKit repositories (licence files read), the W3C Clipboard API draft. `*` = not confirmed on a primary source. Legal notes are a reading of the published terms, not legal advice.

## 1. Format conformity matrix
R/W: P0-P3 = priority in `mzdraw-plan.md` §1; "-" = never. Everything is in `mzlab-mol` or `mzdraw` (Rust), the same code in the page `.wasm`, the app and `mzlab-cli`; "lost" is what an exact round trip cannot keep, and every reader lists it in the import report (§6).
| format | R | W | lost on read (shown in the report) | lost on write | spec, owner, terms | public golden files (licence) |
|---|---|---|---|---|---|---|
| `mzdraw/1` | P0 | P0 | nothing | nothing | ours, this file; open | synthetic, `crates/mzdraw/tests/data/` |
| MOL V2000 | P0 | P0 | query atoms/bonds and S-groups other than SUP/DAT listed; `M  CHG/ISO/RAD` read | arrows, text, styles, annotations, enhanced stereo (chiral flag only); > 999 atoms switches to V3000 (`mol.md`) | BIOVIA "CTfile Formats" (free PDF, all rights reserved: implement, never copy text or examples) | Ketcher `ketcher-autotests/tests/test-data/Molfiles-V2000` (Apache-2.0, 326 files); RDKit `FileParsers/test_data` (BSD-3, check each file's origin) |
| MOL V3000 | P0 | P0 | as V2000; enhanced stereo groups kept; haptic and SCSR blocks listed | arrows, text, styles | same | Ketcher `Molfiles-V3000` (587); RDKit as above |
| SDF | P1 | P1 | as MOL; data fields to `meta.fields` of each object | one record per structure; arrows, text | CTfile ch. 10 | Ketcher `SDF` (84 + 25) |
| RXN V2000/V3000 | P1 | P1 | reactant/agent/product roles and atom maps kept; agent positions | arrow shape (one straight arrow), conditions text, plus signs placement, multi-step schemes | CTfile ch. 5 and 8 | Ketcher `Rxn-V2000` (106), `Rxn-V3000` (37) |
| RDF | P2 | P2 | as RXN; data fields | as RXN | CTfile ch. 11 | Ketcher `RDF-V2000/V3000` (149 + 81) |
| SMILES | P0 | P0 | coordinates (laid out by D1), wedge choice, arrows, text | everything graphic; `unknown` stereo not writable (`mol.md`) | Daylight Theory Manual (reproduction only in full) and OpenSMILES 1.0 (GNU FDL 1.2: implement freely, do not copy text or its `.smi` examples) | synthetic; Ketcher `SMILES` (50) |
| CXSMILES | P0 | P0 | coordinates, enhanced stereo, radicals, atom labels kept; S-group attachments partly | arrows, text, styles | Chemaxon documentation page (no licence statement*, implemented by RDKit and Indigo) | Ketcher `Extended-SMILES` (10); synthetic |
| SMARTS | P2 | P2 | query graph only, into the query layer of `SubQuery` (`mol.md`) | coordinates, graphics | Daylight Theory Manual ch. 4 | Ketcher `SMARTS` (24) |
| KET | P1 | P1 | macromolecule entities and unknown properties listed; text, arrows, plus, shapes kept | annotations (kept in a KET-allowed custom property, round trip with us only) | `schema.json` in `ketcher-core` (Apache-2.0, JSON Schema draft-07, `ket_version` + `root`) | Ketcher `KET` (2,500 files: the largest open corpus) |
| CDXML | P1 | P2 | structure, arrows, text, basic style; templates, TLC plates, spectra, OLE, nicknames and most attributes listed | everything ChemDraw has and we do not (no fidelity claim, `drawing-menu.md`) | public CDX SDK docs (IUPAC archive copy); `Glysade/chemdraw`, a BSD-3 copy of ChemDraw parser code, has an unconfirmed upstream licence and spec terms*: idea only, own reader from the spec | Ketcher `CDXML` (110); RDKit `test_data/CDXML` (76) and `External/ChemDraw/test_data` (117) |
| CDX (binary) | P2 | P3 | same subset as CDXML (one object table: CDX and CDXML map one to one per the SDK) | as CDXML | same SDK (`AllCDXObjects.htm`, `CDXConstants.h`) | Ketcher `CDX` (81) |
| MRV (Marvin) | P2 | P3 | structure, arrows, text, basic `MDocument` graphics | styles beyond colour and font | Chemaxon docs and XSD (no licence stated*) | RDKit `MarvinParse/test_data` (BSD-3, 644 files) |
| CML | P2 | P3 | structure, 2D/3D coordinates; dialects differ (CML, Ketcher CML) | graphics, styles | xml-cml.org, schema `BlueObelisk/cml-schema` (Artistic-2.0) | Ketcher `CML` (47) |
| XYZ | P3 | P3 | 3D projected to 2D, bonds perceived by distance, no charges or stereo (report says "bonds guessed") | bonds, charges, 2D layout | none (de facto 3-part layout) | generated from COD CIF files (CC0) or synthetic |
| PDB (HETATM/ATOM + CONECT) | P3 | - | bond orders guessed (CONECT is connectivity only), small molecules only; polymers refused with a reason | - | wwPDB "Legacy PDB File Format Guide 3.30" | wwPDB archive (CC0 1.0) |
| InChI, InChIKey | - (only through the molecule database, `mol.md`) | - (app/CLI later, decision 5 in `mzdraw-plan.md`) | - | - | IUPAC-InChI, MIT since 1.07 | `INCHI-1-TEST/` (MIT), for database tests only |
| HELM, FASTA, sequences | - | - | refused with a reason (out of scope, `mzdraw.md`) | - | Pistoia Alliance | - |
| SVG | P0 (own SVG with the embedded document only) | P0 | only an SVG written by us re-opens (§2) | not editable by others as chemistry | W3C | ours |
| PNG | P1 (own PNG with the embedded document only) | P1 | as SVG | raster | W3C PNG, `iTXt` chunks | ours |
| PDF | - | P2 | - | vector, base-14 fonts or embedded subset (decision 4) | ISO 32000 | ours |
| EMF / OLE | - | - | Windows desktop only, outside the browser; not planned | - | - | - |

The matrix is generated, not hand-kept, once the formats exist: `mzlab-cli mol formats --json` (catalog `formats/1`: per format `read`, `write`, `lost_on_read`, `lost_on_write`, `detect` rule, `mime`, `ext`), checked against the conformity tests (ready prompt P1 in `mzdraw-plan.md`); this table then points to it.

## 2. Images that carry their molecule, clipboard
- **SVG**: `<metadata>` holds `<mzdraw:doc xmlns:mzdraw="https://freeesta.github.io/mzlab/ns/mzdraw/1">` with the `mzdraw/1` JSON in CDATA, plus `<mzdraw:molfile>` with the V3000 molfile of each molecule for other programs. Opening an SVG reads the document; an SVG without it is refused ("not a structure file"), never traced. Idea from RDKit (`<rdkit:mol>` in metadata) and Open Babel (`-e`, CML in the SVG).
- **PNG**: `iTXt` chunks, keyword `mzdraw` (the document, zlib-compressed, UTF-8) and `molfile` (V3000 text); idea from RDKit's PNG metadata (`tEXt`/`zTXt`, its key prefix*). Writing an RDKit-readable chunk as well is cheap and is decision-free (one extra chunk), done when the prefix is confirmed by a test against RDKit in `tools/`.
- **PDF** (P2): the document as an embedded file (`/EmbeddedFiles`, `mzdraw.json`) so a figure in a report can be reopened.
- **Web clipboard** (W3C Async Clipboard API): `text/plain`, `text/html` and `image/png` are the only mandatory types; custom `web <mime>` types exist in Chromium only (since 104), up to about 100 per write, and native programs never see them.
  Copy therefore writes `text/plain` (molfile V2000 by default, SMILES by setting), `image/png` (with the embedded document), `text/html` (an `<img>` of the SVG), and in Chromium `web application/x-mzdraw+json`, `web chemical/x-mdl-molfile` and `web chemical/x-indigo-ket` (the last one so a paste into the Ketcher Drawing page keeps arrows and text: Ketcher reads `chemical/x-indigo-ket`, `chemical/x-mdl-molfile`, `chemical/x-mdl-rxnfile`, then `text/plain`, in that order).
  Paste reads the same, plus SMILES, CXSMILES, KET, CDXML, MRV and CML text sniffed by content (§6).
- **Desktop app** (Tauri, P3): the native clipboard can add Windows "ChemDraw Interchange Format" (raw CDX) once a CDX writer exists; ChemDraw also hides a second CDX in the comment records of the WMF/EMF picture it pastes (SDK `Clipboard.htm`), which the app could read. Stated limit: the page cannot do either.
- **mzLab links**: a document small enough (≤ 2 KB compressed) can travel in a URL fragment as base64url (idea from OpenChemLib's IDCode), opened by `draw2/#d=...`; nothing is sent to a server.

## 3. Golden files: sources and rules
- Allowed to copy, with a `SOURCE.md` (URL, commit, licence, file list) next to them in `crates/mzdraw/tests/data/third_party/<source>/`: Ketcher test data (Apache-2.0, keep the NOTICE text), RDKit test data (BSD-3, only files whose origin is RDKit's own or public domain; files taken from patents, ChEMBL or PubChem are left out), InChI tests (MIT), wwPDB and COD files (CC0), ChEBI structures (CC BY 4.0, attribution in `SOURCE.md`).
- Never copied: CDK (LGPL-2.1), Open Babel (GPL-2.0), OpenSMILES examples (GNU FDL), BIOVIA spec examples (all rights reserved), Daylight manual examples, PubChem bulk files (depositor terms; NCBI says PubChem content "may be protected"). For those cases a synthetic file is written by hand or by `tools/dati_sintetici.py`.
- No lab data ever (CLAUDE.md); a public structure file is not lab data, but the third-party folder is capped (decision 3 in `mzdraw-plan.md`: recommendation ≤ 2 MB, ≤ 300 files, chosen to cover each "lost" row of the matrix).
- Each golden file has an expected outcome in one JSON (`conformity.json`: file, format, expected `mol/1` core hash or `error` key, expected `lost` list); RDKit 2026.09 regenerates the chemistry expectations in `tools/` only (`mol.md` "Golden data from RDKit").

## 4. Settings file `mzdraw-settings/1`
One JSON file shared by the editor, the page and the CLI (`mol draw --set style=FILE|NAME`, `mzdraw.md`). Serde types in `mzdraw`, JSON schema generated from them (never by hand), unknown keys kept on write. Units are physical (pt) so a preset prints at its true size; the screen scales by zoom.
`{format, style, templates, abbreviations, shortcuts, editor}`:
- `style` (all optional, defaults = preset `mzlab`): `bond_length` pt, `line_width` pt, `bold_width` pt, `hash_spacing` pt, `bond_spacing` (fraction of the bond length), `margin` pt (gap between a label and its bonds), `chain_angle` degrees, `font` {family list, size pt, weight}, `subscript_scale`.
  Display: `colour` {mode `element|mono`, palette name or map element to colour, `background`, `highlight`}, `labels` {`carbon` none|terminal|all, `hydrogens` minimal|hetero|all (CDK's modes as idea), `placement` auto|left|right}, `aromatic` kekule|circle, `wedge` {`style` solid|fancy, `hash_style` lines|wedge}, `annotation` {font scale, colour, `level_badge` on|off} (§5), `arrow` {head length, width}.
- `templates`: named `mzdraw/1` fragments with an attachment atom or bond (rings 3-8, benzene, chair, steroid core; the lab adds its own).
- `abbreviations`: lines `"LABEL SMILES"` with the attachment atom first (format idea from CDK's list, own content): `Me C`, `Et CC`, `Ph c1ccccc1`, `Boc C(=O)OC(C)(C)C`, ...; rule `contract` never|on-hetero|auto (auto = only when more atoms are inside than outside).
- `shortcuts`: key to command id (`mzdraw.md` keyboard map is the default); a conflict is reported on load, never silently resolved.
- `editor`: snap angle, autosave delay, pointer radii, pen/finger behaviour.
- Built-in presets (each is a settings file like any other, in `crates/mzdraw/presets/`):
| preset | bond length | line | bold | hash | bond spacing | margin | font | source |
|---|---|---|---|---|---|---|---|---|
| `mzlab` | screen default (to tune in slice 2) | | | | | | system sans | ours |
| `acs-1996` | 14.4 pt (0.508 cm) | 0.6 pt | 2.0 pt | 2.5 pt | 18 % | 1.6 pt | Arial/Helvetica 10 pt, chain angle 120° | ACS "Document 1996" journal settings: 14.4 pt, 0.6 pt, 18 % and 10 pt confirmed in RDKit `setACS1996Options`; bold, hash and margin from the ChemDraw help and style guides* |
| `slides` | 1.5x `acs-1996` | 1.5 pt | 4 pt | 4 pt | 18 % | 2.5 pt | 16 pt | ours |
| `dark` | as `mzlab` | | | | | | | ours, `colour.background` dark, palette for contrast ≥ 4.5:1 |
- Other journals (RSC, Wiley, Elsevier, Nature): no preset is shipped until its numbers are read from the journal's current author guidelines; a user can always save their own.

## 5. Document format `mzdraw/1`
JSON, UTF-8, `mol/1` conventions (fixed key order, empty fields omitted, numbers rounded to 4 decimals, same document = same bytes; `mol.md`). Extends the outline in `mzdraw.md`.
```
{"format":"mzdraw/1","app":{"name":"mzDraw","version":"…"},"style":"acs-1996"|{…},
 "objects":[ {"kind":"mol","id":1,"mol":{<mol/1 core: atoms, bonds, stereo, groups>},
              "aid":[…],"bid":[…], "xy":[[x,y],…], "marks":{"<bond id>":"wedge|hash|wavy|bold"},
              "groups":[{"kind":"abbr","label":"Boc","atoms":[…],"contracted":true}],
              "rlabels":{"<atom id>":"R1"}, "query":{…}, "style":{…}, "ann":[…], "meta":{…}},
             {"kind":"arrow","id":2,"type":"forward|equilibrium|retro|resonance|failed|curved-full|curved-half",
              "pts":[[x,y],…],"from":[…],"to":[…],"above":<text>,"below":<text>},
             {"kind":"plus","id":3,"xy":[x,y]}, {"kind":"text","id":4,"xy":[x,y],"runs":[{"t":"H","sub":"2"},…]},
             {"kind":"shape","id":5,"type":"rect|ellipse|bracket|line","pts":[…]} ],
 "reactions":[{"reactants":[1],"agents":[],"products":[6],"arrow":2}],
 "unknown":{…}, "meta":{"title":…,"fields":{…}}}
```
- **Ids**: integers stable for the life of the document (never reused after a delete); `aid`/`bid` map the record's atom and bond indices to them, so per-atom marks survive a re-canonicalisation. Coordinates are in bond lengths (1.0 = one standard bond); the style turns them into pt.
- **Chemistry vs drawing**: the `mol/1` core is the chemistry; `xy`, `marks`, `groups` are the drawing. The rule of `mol.md` holds: wedges are drawn, `stereo` elements are derived at export, never the reverse.
- **Versioning**: `format` carries the major version. A reader of `mzdraw/N` reads every earlier version (one fixture per version kept forever); a new optional field does not bump the version; a change of meaning does. Unknown keys of any object go to that object's `unknown` and are written back unchanged, so an older editor never loses a newer file's data.
- **Round-trip guarantees**, each a test: (1) read then write gives identical bytes for every fixture; (2) any random command sequence then all its undos gives identical bytes (`mzdraw.md`); (3) `mzdraw/1` → SVG/PNG/PDF with the embedded document → `mzdraw/1` is identical; (4) `mzdraw/1` → MOL V3000 → `mzdraw/1` keeps the `mol/1` core hash and the coordinates (4 decimals), drops only the rows listed in §1; (5) wasm, app and CLI write the same bytes (parity test).
- **Annotations** `ann` (per structure, designed now, filled by the identification features later): `[{"level":"2b","scale":"schymanski-2014","evidence":[{"kind":"ms2-library|ms2-insilico|rt-standard|diagnostic-fragment|formula|exact-mass|context","ref":"…","note":"…"}],"scores":[{"name":"cosine","v":0.91,"from":"msn lib search"}],"source":{"tool":"mzlab-cli flow …","run":"<flow record id>"},"by":"chemist|auto","note":"…"}]`.
  `level` takes the strings of `levels.md` (`1`, `2a`, `2b`, `3a`, `3b`, `4`, `5`; `0` = detected, not annotated), not bare integers 1-5, so the sub-levels mzLab already uses fit; `by` keeps "the chemist decides" visible (an automatic level is marked, `levels.md` "auto-2b"). Several entries = history, the last one is current.
Rendering: a badge "L2b" at the lower right of the structure's box and an optional caption line (evidence kinds, best score) under it, both from `style.annotation`; written into SVG/PNG/PDF and into SDF as data fields (`MZLAB_LEVEL`, `MZLAB_EVIDENCE`); KET and CDXML carry it as plain text only.
- **Autosave**: OPFS `mzdraw/autosave.json` debounced (1 s) plus the last 10 versions in `mzdraw/history/` (ring buffer), offered on the next visit; page only, a stated limit (`mzdraw.md`).

## 6. Import: detection and report
- Detection by content, never by extension alone: `{` + `"format":"mzdraw/` / `"root"` (KET) / `M  V30` / `$RXN` / `$RDFILE` / `$$$$` / `<CDXML` / `VjCD0100` (CDX magic) / `<cml` / `<MDocument` / XYZ count line / `HETATM` / SVG and PNG with our metadata; a SMILES is the fallback for one line of SMILES characters. Ambiguity is reported with the candidates, never guessed silently.
- The import report `import/1` (shown after opening, returned by `mol draw --json`): `{format, version, read:{mols, arrows, texts}, lost:[{what, count, where}], warnings:[{key, params}], layout:"kept|computed"}`; `what` uses the stable keys of the `lost_on_read` lists of `formats/1`. A file with lost parts still opens: the user decides (CLAUDE.md).
