# mzDraw: a molecule editor of our own (study and build plan)

mzDraw is a 2D structure editor written in Rust and compiled to wasm, made to stand alone and to leave this repository one day. The name lives in one place per layer: the crate names `mzdraw` / `mzdraw-wasm`, the constant `mzdraw::NAME` and the i18n key `draw2.name`.
The `qqq` teaching site and the Drawing page keep Ketcher (`drawing.md`); mzDraw has a page of its own, `mzlab/web/draw2/`, reached only by its exact link, absent from the navigation and from the `qqq` build.

## Scope
- **In**: organic molecules; atoms and elements, bonds (single, double, triple, wedge, hash, wavy, aromatic), charges, isotopes, radicals, labels, abbreviations (Ph, Me, Et, Boc, ...), R1/R2 placeholders, rings and templates (benzene, cyclohexane, rings 3-8), chains, select/move/rotate/copy/paste/delete, undo/redo, fragments, reaction arrows with text, text annotations, zoom/pan, mouse + touch + pen + keyboard.
  Also in: every exchange format of the conformity matrix, SVG/PNG/PDF out, clean-up (2D layout), live valence and charge check (`check/1`), CIP labels, styles and presets, per-structure annotations.
- **Out**: macromolecules (peptides, DNA, polymers), query features (atom lists, query bonds), S-groups other than simple abbreviations. A file carrying them opens with the unsupported parts listed in the import report, never silently dropped.

## What the Ketcher page does (to match, from `drawing.md` and `drawing-menu.md`)
| Drawing page feature | mzDraw |
|---|---|
| atoms, bonds, rings, chains, charge, radical, eraser, selection, arrows, text, shapes | slices 1-2 (shapes: later) |
| zoom in/out/reset, full canvas, tablet toolbars, 40 px touch targets, long-press names | slices 1-2 |
| "New" with confirmation, unsaved-changes guard | slice 2 (autosave in OPFS makes the guard a recovery prompt) |
| Quick draw: paste a SMILES, search a compound by name | SMILES slice 1; name search later (molecule database glue) |
| open/drop/paste molfile, KET, CDXML, CDX | matrix below; slice 1 molfile + SMILES |
| export images, `.ket`, MOL/SDF | slice 1 molfile + SVG, slice 3 the rest |
| right-click menu, touch copy/cut/paste | slice 2 (own clipboard, no Ketcher traps) |
| labels under structures, formula analysis, Information..., fragments, cuts, mechanism, partial structures, multicentre, evidence colours, substructure search, outside links, CIP card | later, one by one: every engine already takes a molfile (`label/1`, `analysis/1`, `frag/1`, `cut/1`, `mechanism/1`, `parz/1`, `highlight/1`, `substructure/1`, `link/1`, `cip/1`); mzDraw hands over the same molfile plus stable atom ids |
| `TPDraw` API for mzFinder and Tabui (`load`, `smiles`, `partial`, `mark`, `highlight`) | later; the wasm API is shaped for it (`Editor::export`, `select`, `overlay`) |

## Assets reused (nothing here is rewritten)
- `mzlab-mol` (deps: `serde`, `serde_json`, `sha2`): record `mol/1`, SMILES/CXSMILES and molfile V2000/V3000 readers and writers, `drawing_from_molfile` (wedges, query marks kept), valence table, Kekulé, aromaticity perception, stereo sites, canonical order, pieces.
- `mzlab-feat` `chem/` (features `check`, `sub`): `check/1`, `cip/1`, `highlight/1`, `sdf/1`, labels, analysis. mzDraw calls them only from the glue layer.
- Agent drawing (`agent-draw.md`, branch `claude/agent-draw`): the route "own Rust layout + SVG writer in `mzlab-mol`". Work in progress elsewhere and consumed here, never duplicated: `Mol::layout_2d` / `layout_2d_fixed` and `mol draw` (D1), the SVG writer (D2), the edit patches of `mol draw --set edit=` (D3), the KET and CDXML writers (D4), the CDXML reader (DRAW-CDXML).
- Until D1 exists, "Clean up" calls `mzdraw::layout::clean_up`, one function that returns the coordinates unchanged with `status: "unavailable"`; wiring D1 changes that function only.

## Architecture
```
mzlab-mol (record, readers, writers; later layout + SVG)      no mzLab dependency
   ^
mzdraw (document, scene, commands, undo, tools, hit-test,     depends on mzlab-mol, serde, serde_json only
        display list, settings, format registry)
   ^                          ^
mzdraw-wasm (wasm-bindgen)    mzlab-cli `mol draw` / mzlab-desktop
   + mzlab-feat[check]  = the thin mzLab glue: check/1, cip/1, i18n keys, OPFS autosave
```
- **Leaving the repo** = moving `crates/mzlab-mol` + `crates/mzdraw` (+ `mzdraw-wasm` without the `mzlab` cargo feature) and `mzlab/web/draw2/`. Guard: a test reads `cargo metadata` and fails if `mzdraw` gains a dependency outside `mzlab-mol`, `serde`, `serde_json`.
- **One wasm module** `mzdraw-wasm`, an entry of its own in `tools/moduli/` with its own cap. Targets: < 600,000 B raw, < 250,000 B gzip; the cap is the first measured size + 5 %. Basis: `molkit` (record + readers + `check/1` + substructure) measures 352,069 B; the editor core is commands and geometry with no tables, estimated 80-120 KB on top of the readers.
- **Render path: Canvas2D for the live view, SVG from Rust for files.** Measured (scratch Playwright bench, Chromium, deviceScaleFactor 2, synthetic 500 atoms / 520 bonds / 150 labels, full repaint, median / p95): Canvas2D 2.4 / 3.2 ms; SVG DOM rebuild 3.6 / 7.5 ms script only, 814 nodes, plus style and layout before paint. With CPU throttled 4x (tablet class): Canvas2D 16.2 / 21.4 ms, SVG 17.7 / 28.9 ms script plus a frame at 27.5 ms.
  Canvas keeps a full repaint inside one 60 Hz frame on a desktop and close to it on a tablet with no DOM to keep in sync; a drag repaints only the display list, which Rust rebuilds in place. Hit-testing never needs DOM nodes (it is in Rust). To re-measure on WebKit on a Mac and on a real tablet in slice 2 (the cloud container has no WebKit build).
- **Display list**: Rust turns document + style + view state into a flat list of primitives (`line`, `poly`, `text`, `arc`, `rect`, each with a style index and the id of the object it paints), as one `Float32Array` plus a string table. JS paints it and nothing else; the same list serialised by Rust is the SVG, so the screen and the file cannot disagree. When the D2 SVG writer exists the list goes through it (one SVG writer in the workspace).
- **Input model**: JS forwards `pointerdown/move/up/cancel`, `wheel`, `keydown` as numbers (x, y in CSS px, pointer type, pressure, buttons, modifiers, key code); Rust owns the tool state machine, hit-testing (uniform grid over atoms and bond midpoints; radius 8 px mouse, 16 px pen, 22 px finger), snapping (30° steps, standard bond length), hover and the preview. Each event returns flags `{repaint, changed, cursor, announce}`. Target: event to display list < 1 ms for 500 atoms in wasm; pointer-to-paint within one frame.
- **Document model** `mzdraw/1` (below): objects with stable ids; a molecule object is a `mol/1` core plus what the record does not hold (2D coordinates as the user drew them, wedge marks as drawn, aromatic-circle display, abbreviation groups, R labels). Stereo follows the `mol.md` rule: the wedges are the drawing, the `stereo` elements are derived by the record's molfile rules at export, never the reverse.
- **Undo** = a command log in Rust. Every edit is a command with an exact inverse (the removed atoms and bonds are stored in the inverse); undo and redo are two stacks of commands, a drag is one command (coalesced). Property tested: any random command sequence then all its undos gives the identical document bytes, and redo gives the identical final bytes.
- **Command log = edit patch language.** The log is the JSON that `mol draw --set edit=` takes (`mol-draw.md`, schema `draw_edit/1`): `[{"op":"atom.add","id":7,"el":"N","x":1.5,"y":0},{"op":"bond.add","a":3,"b":7,"o":1},{"op":"atom.set","id":7,"chg":1},{"op":"move","atoms":[3,7],"dx":0.5,"dy":0},{"op":"remove","atoms":[7]}]`, plus `bond.set`, `merge` and wedges. Ids are atom ids, bonds are named by their two atoms, coordinates are in bond lengths (`x`, `y`; `x_a`, `y_a` are angstrom). Rings and undo are not in the language: mzDraw keeps its own inverse commands.
- **Determinism**: same document + same command log = same bytes from wasm, app and CLI (golden parity test). Floats are rounded to 4 decimals at the document boundary; no hash maps in serialised paths; no time, no randomness.
- **Accessibility**: the canvas is `role="application"` with a label; every tool is a real `<button>` with `aria-pressed`; full keyboard editing (a focus ring on one atom or bond, arrows move the focus along bonds, typing an element sets it, Enter/Shift+Enter grow a bond, digits set the order); every change is announced through an `aria-live="polite"` line built from i18n keys ("Atom N added, bonded to C3"); a text outline of the structure (formula, pieces, warnings) sits next to the canvas for screen readers; `prefers-reduced-motion` and forced-colours are honoured.
- **i18n**: Rust returns keys and parameters, never sentences; the page resolves them in `mzlab/web/lang/en/` (`draw2.*`). Tooltips are a name and a shortcut, nothing longer.

## Keyboard map
| keys | action |
|---|---|
| `C N O S P F H B I`, `Shift+C` Cl, `Shift+B` Br, `Shift+S` Si | element on the hovered/focused atom, or the atom tool |
| `1` `2` `3` | bond order (on a bond) or bond tool; `W` wedge, `Shift+W` hash, `Y` wavy, `A` aromatic |
| `+` `-` | charge; `.` radical; `Shift+I` isotope dialog |
| `R` then `3`-`8` | ring; `Shift+R` benzene; `X` chain |
| `V` / `Esc` select, `E` eraser, `T` text, `Shift+A` arrow, `L` label/abbreviation |
| arrows, `Enter`, `Shift+Enter`, `Tab` | move focus along bonds, grow a bond, grow a double bond, next piece |
| `Ctrl/Cmd+Z`, `Shift+Ctrl/Cmd+Z` or `Ctrl+Y` | undo, redo |
| `Ctrl/Cmd+A/C/X/V`, `Del`/`Backspace` | select all, copy, cut, paste, delete |
| `Ctrl/Cmd+0/+/-`, `Space`+drag, two-finger drag, pinch | zoom to fit, in, out, pan |
| `Ctrl/Cmd+L` | clean up; `Ctrl/Cmd+S` save, `Ctrl/Cmd+O` open |
Every binding comes from the settings file and can be changed there.

## Own document format `mzdraw/1` (open, versioned, round-trips everything the editor draws)
JSON, compact, fixed field order, the `mol/1` conventions (empty fields omitted, same document = same bytes):
`{format:"mzdraw/1", style:<name or object>, objects:[...], meta:{...}}`. Object kinds: `mol` (`id`, `atoms` and `bonds` as in `mol/1` with a stable `id` each, `xy`, `marks` = wedge/hash/wavy/aromatic display per bond, `groups` = abbreviations and their expansion, `rlabels`, `ann`), `arrow` (kind, points, text above/below), `text` (position, runs with sub/superscript), `plus`.
- `ann` on a molecule is the per-structure annotation: `{level: 1-5, evidence: [...], scores: [{name, v, from}], source}` (identification confidence level after Schymanski). The slot and its rendering place (a badge and caption under the structure) are reserved from the first version; the identification features fill it later.
- A format change bumps the number; the reader of version N reads every earlier version (fixtures kept in `crates/mzdraw/tests/data/`). Unknown fields are kept and written back, so a newer file survives an older editor.
- Autosave: the page writes the document to OPFS (`mzdraw/autosave.json`, debounced 1 s after the last change, through the glue) and offers it on the next visit.

## Format conformity matrix
R = read, W = write. "Slice" is when it lands; "who" names the code that owns it.
| format | R | W | lost or refused | slice / who |
|---|---|---|---|---|
| `mzdraw/1` | yes | yes | nothing | 1 / `mzdraw` |
| MOL V2000 | yes | yes | R: query atoms and bonds reported; W: arrows, text, styles, annotations; abbreviations expanded | 1 / `mzlab-mol` |
| MOL V3000 | yes | yes | as V2000; enhanced stereo groups kept | 1 / `mzlab-mol` |
| SDF | yes (every record, data fields into `meta`) | yes | as MOL; one record per structure | 3 / `sdf/1` |
| SMILES, CXSMILES | yes (laid out by D1) | yes | coordinates, wedge choice, arrows, text; `unknown` stereo not writable | 1 read with stub layout, 3 full |
| RXN, RDF | yes | yes | text and styles; arrow shape (always one straight arrow) | later / new reader on the molfile blocks |
| KET | yes | yes | macromolecule and query parts refused; our annotations not written | 3 / D4 writer, reader in `mzdraw` |
| CDXML | yes | yes | R: only structure, arrows, text, basic style (the open schema; no fidelity claim, `drawing-menu.md`); W: annotations as plain text | 3 / DRAW-CDXML reader, D4 writer |
| CDX (binary) | later | no | same subset as CDXML once the tag table is read; never written | later, to decide by measurement of the reader size |
| MRV (Marvin) | later | later | structure, arrows, text only | later |
| CML | later | later | structure only | later |
| XYZ, PDB | later | no | read as 3D atoms projected to 2D, bonds perceived by distance (PDB: `CONECT`); organic small molecules only | later, where sensible |
| InChI | no | later | no InChI reader is written (`mol.md`); writing needs an InChI engine outside the capped wasm | later / app and CLI only |
| SVG | no | yes | not re-readable; the `mzdraw/1` document can be embedded in `<metadata>` to make the SVG re-openable | 1 |
| PNG | no | yes | raster; same embedded document in a `tEXt` chunk | 3 (page: canvas raster of the SVG; CLI: `resvg` behind the `png` feature) |
| PDF | no | yes | vector, text as text; no embedded fonts beyond the base 14 | later / small own writer on the display list |
| clipboard | yes | yes | writes `text/plain` molfile, `image/svg+xml`, `web chemical/x-mdl-molfile`, own `web application/x-mzdraw+json`; reads the same plus SMILES text, KET and CDXML text | 2 |
Every reader returns an import report (`{read, lost: [{what, count}]}`) shown after opening: the user sees what did not come through.

## Customisation: one settings file `mzdraw-settings/1`
One JSON file shared by the editor, the page and the CLI (`mol draw --set style=FILE|NAME`): `{format, style, templates, shortcuts}`.
- `style`: bond length (pt), line width, bold width, hash spacing, double-bond gap, margin around labels, font family and size, subscript scale, atom colours (per element or all black), background, label placement (auto/left/right), explicit-H policy, terminal-carbon labels, aromatic rings as circles or alternating bonds.
- Built-in presets: `mzlab` (screen default), `journal-1996` (the ACS Document 1996 numbers: bond length 14.4 pt, line width 0.6 pt, bold width 2 pt, hash spacing 2.5 pt, double-bond gap 18 % of the bond length, Arial/Helvetica 10 pt, fixed length), `slides` (thick lines, large labels), `dark`. A preset is a settings file like any other; the numbers are public journal guidelines, no resource of another program is copied (`drawing-menu.md`).
- `templates`: named `mzdraw/1` fragments with an attachment atom or bond (rings 3-8, benzene, chair, common protecting groups); `shortcuts`: key to command. The file is validated by a JSON schema generated from the Rust types.

## CLI integration (`mzlab-cli mol draw`, with D1-D3)
- Whatever the editor produces, the CLI produces with the same Rust functions and the same bytes: `mol draw IN|- --set format=svg|png|pdf|molfile|ket|cdxml|mzdraw --set style=... --set edit=LOG`, `--json` with stable keys, exit codes per `cli-agenti.md`, data on stdout, progress on stderr, `--dry-run` = parse + check without output. Batch: several inputs or a stream on stdin (one SMILES or one JSON document per line, one result per line), no browser anywhere in the path.
- Replaying an editor session: `mol draw DOC --set edit=LOG` applies the same command log the editor recorded; the golden parity test compares its bytes with the wasm editor's.
- Speed gates (native, release, one thread, criterion-style bench in `crates/mzdraw/benches`, numbers written to this file when measured): command apply < 5 µs; document to display list < 50 µs and display list to SVG < 50 µs for a 30-atom molecule; layout + SVG for a small molecule in the tens of µs to low hundreds (the reference engines measure 10-180 µs median for layout alone, `agent-draw.md`); 500 atoms to display list < 1 ms.

## Interaction design: what the other editors teach
| editor | worth taking | to avoid |
|---|---|---|
| ChemDraw | hotkeys on the hovered atom (type `N` over an atom), click a bond to cycle its order, drag from an atom to sprout a bond at 30° steps with fixed length, journal presets, chemistry-aware labels typed as text (`OMe`, `CO2H`) | no resource or wording is copied; CDX fidelity is not claimed |
| Ketcher | open KET format, template library, floating right-click menu | iframe + worker + 12 MB Indigo; clipboard traps, a paste that rezooms, `localStorage` without guards (`drawing.md`) |
| Marvin JS | abbreviation groups that expand/contract, clear valence-error underline, good import dialog | server-side services for clean-up and naming |
| JSME | one-click simplicity and tiny size (a single small script), works on touch | no undo depth, no reactions with text, dated look |
| Kekule.js, ChemDoodle Web | object model with separate render and IO layers; ChemDoodle's lasso and hover feedback | size, licences (GPL for ChemDoodle Web) |
| OpenChemLib editor | fast, compact clean-up; stereo recognition while drawing | it reorders atoms and drops explicit H (`mol.md` trap) |
| PubChem Sketcher, MolView | import by name, immediate SMILES/InChI round trip in the page | server round trips for every edit |
Rules taken: (1) everything on the canvas is reachable in one gesture and reversible with one undo; (2) hover shows what a click will do (preview in a lighter stroke); (3) a wrong valence is marked while drawing, never blocked ("the user decides"); (4) no modal dialog for atom properties: a small popover next to the atom, usable from the keyboard; (5) a tap target is 40 px on touch; pen = precise tool, finger = tool or pan by setting, palm rejected while the pen is near (as `touch.js`). The full competitor matrix comes from the architecture study and replaces this table when it exists.

## Build order and estimates (1 unit = one small PR, about half a day of an agent)
| slice | content | units |
|---|---|---|
| 1 | crate `mzdraw` (document, ids, atom/bond add/remove/move/set, ring templates, charge, undo/redo, command log, display list, hit-test, select/erase tools), `mzdraw-wasm` + cap, page `draw2/` (canvas, palette, SMILES/molfile in, molfile/SVG/`mzdraw/1` out, live `check/1` marks), unit + property + parity + e2e tests | 3 |
| 2 | copy/paste and clipboard formats, rotate/flip, wedge/hash/wavy with stereo through the record, labels and abbreviations, R1/R2, text, reaction arrow and plus, chains, touch and pen gestures, full keyboard editing, accessibility pass, settings file with presets, OPFS autosave | 4 |
| 3 | KET and CDXML in/out (D4, DRAW-CDXML), SDF, PNG, clean-up wired to D1, CIP marks, `mol draw` replay of the log (D3), speed bench in this file, comparison page next to Ketcher (test only) | 3 |
| later | RXN/RDF, MRV, CML, XYZ/PDB, CDX, PDF, shapes, template library editor, name search, annotations rendering with the identification features, the Drawing page panels (labels, analysis, fragments, cuts, mechanism, partials, evidence colours, substructure, links), `TPDraw` API, desktop window | 1-2 each |

## Limits stated
- The page rasterises PNG from the SVG with a canvas; the CLI uses `resvg`: parity is on SVG bytes, not PNG bytes.
- The OPFS autosave and the system clipboard exist only in the page; the CLI reads and writes files and stdin/stdout.
- Text metrics: label widths come from a fixed table of advance widths for the preset fonts in Rust, so the layout is identical in wasm, app and CLI; a font outside the table falls back to the table of the nearest generic family.
