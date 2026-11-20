# `mol draw`: own 2D layout, the `draw/1` scene, KET/CDXML files, SVG/PNG pictures and the edit patch (`mzlab-mol::layout`, `mzlab-feat::chem::draw*`, `draw_file`, `chem::edit`)

A structure written as text (SMILES, molfile, a name or InChIKey of the molecule database) becomes a drawing: coordinates in angstrom, bond length 1.5, the same on the site (molkit `.wasm`), in the app (`feat/grafo`, `kind:"draw"`) and in the CLI (`mzlab-cli mol draw`). No code of other toolkits; RDKit is only the yardstick of the tests (`tools/draw_quality.py`, never a dependency).

## Layout (`crates/mzlab-mol/src/layout/`)
- `Mol::layout_2d()` and `layout_2d_fixed(&[Option<[f64;2]>])` (atoms given are kept, the rest is placed around them). Deterministic: no random numbers, no libm (`trig.rs` has its own `sincos`/`angle`), coordinates rounded to 4 decimals, so native and `.wasm` bytes are equal.
- `rings.rs`: SSSR and ring systems. `place.rs`: a ring system as regular polygons fused along shared bonds; bridges on arcs; a spiro ring hung on its atom; a ring of 8 or more atoms as one large polygon (a defined trans double bond in it gets a kink, `G::kink_at`). `grow.rs`: substituents and chains (120-degree zigzag, linear atoms straight, a crowded centre spread evenly). `clean.rs`: `fix_ez` redraws a defined double bond with the written cis/trans, then `tidy` pushes apart overlaps with a penalty search (atom pairs, atom on bond, crossings).
- Fragments (salts, mixtures): laid out one by one, left to right, largest first, 2 bond lengths between bounding boxes (the gate asks 1.5).
- Coordinates never decide tetrahedral stereo: `mol/1` keeps `par` and `refs`; `to_molfile` chooses the wedges from the reader's own parity (`Mol::written_bonds()` lists the bonds as written). E/Z is as written: for double bonds in rings of 8 or more atoms or outside rings the layout draws the `cis`/`trans` of the record.
- Reader rule that shapes the tests: a double bond in a ring of up to 7 atoms has no stereo element; elsewhere an unmarked double bond with substituents reads back as defined from the coordinates, so a round trip through a molfile is the same record only when the SMILES marks it. The sets avoid unmarked ones.

## The request and the scene (`chem/draw.rs`)
`draw_json(request)`, request `{"spec", "format":"molfile"|"scene"|"ket"|"cdxml", "file":"ket"|"cdxml" (scene only), "layout":"keep"|"new", "db": CSV text, "name": title}`; answer: the scene (`draw/1`, `docs/schemi/draw-1.json`), the molfile, the KET or the CDXML text.

| spec | meaning |
|---|---|
| `smiles:...` | a SMILES |
| `molfile:...`, a path or `-` whose text has `M  END` | a V2000/V3000 molfile; its 2D coordinates are kept (`layout=keep`, default); `layout=new` lays it out again; 3D coordinates or all atoms at the origin are laid out |
| `name:...`, `inchikey:...`, `InChI=...` | looked up in the `molecules/1` CSV of `--set db=PATH` (e.g. `mzlab/web/molecules/catalogue.csv`); an InChI string is not parsed: the hint says to give its InChIKey |
| no prefix | SMILES characters only: a SMILES; `InChI=` or 27 characters `XXXXXXXXXXXXXX-XXXXXXXXXX-X`: a key; anything else: a name |

Scene: `{schema, spec_kind, title, atoms:[{i,el,chg,iso,h,x_a,y_a}], bonds:[{a,b,o,wedge}], fragments, choices, warnings}`. `wedge`: 0 none, 1 up, -1 down, 2 either, on the first atom of the bond; bond orders are Kekule. `choices` logs every automatic decision (`spec_kind`, `layout`, `molfile_version`, `name_hit`/`key_hit` with its score); `warnings`: `name_approximate` (the name matched only as part of another), `name_ambiguous` (several names tie), `key_ambiguous`
(an InChIKey with several rows in the db; the first is drawn, the names are listed). In molfile mode the CLI prints them on stderr as `choice: key = value` and `warning: ...`.
The Draw page uses the route for its «Draw» button (`mzlab/web/disegno-smiles.js`): a SMILES goes as `{"spec":"smiles:...","format":"molfile","layout":"new"}` (site `.wasm` `draw`, app `feat/grafo` kind `draw`) and Ketcher adds the molfile next to the drawing. An InChI is not sent (`err.mol.draw.inchi`); a refusal (`err.mol.draw.smiles`, a reaction SMILES) or no engine makes the page add the text itself, as before, and Ketcher's own notice says if it drew nothing. Test `e2e_disegno_smiles`.

## CLI (`crates/mzlab-cli/src/mol_draw_cmd.rs`)
`mzlab-cli mol draw SPEC [--format molfile|scene|ket|cdxml|svg|png] [--set layout=keep|new] [--set db=PATH] [--set name=TITLE] [--set scale=PX] [--set cip=on|off] [--set highlight=LIST] [--set edit=JSON|@PATH] [--out PATH] [--json]` (`--format F` is `--set format=F`). Plain output is the molfile, the KET, the CDXML, the SVG text or the PNG bytes; `--json` is the scene, with the KET or CDXML text in its `file` key when `--format` names one, or with svg/png the `draw-image/1` wrapper (`--json` with `--format molfile` is a usage error). SPEC is `-` for stdin; a path that exists is read.

| exit | meaning |
|---|---|
| 0 | ok |
| 1 | over 2000 atoms (`err.mol.draw.tooBig`, data, as every `*.tooBig`), an atom that is no element in KET or CDXML (`err.mol.draw.element`, data, like `error.grafo.element`) |
| 2 | unparseable SMILES, bad `--set`, a name/key without `--set db` (the hint names the flag), an InChI string, an edit patch that is not JSON or whose operation fails |
| 4 | missing or unreadable file, db or `edit=@PATH`, a molfile or request that cannot be read (`err.mol.draw.molfile\|request`), a file with no `M  END` |
| 5 | not in the db (`err.mol.draw.not_found` with the nearest names in the hint, or `err.mol.draw.not_found_bare` when none is near) |
| 6 | unwritable `--out` |

Under `--json` a failure is one `error/1`. Error keys: `err.mol.draw.request|set|binary|smiles|molfile|needDb|inchi|db|not_found|not_found_bare|tooBig|element|edit|editJson` (limit 2000 atoms), texts in `lang/en/errori-a-m.js`.

## KET and CDXML (`chem/draw_file.rs`)
Open formats written by our own code from their public documentation (the KET JSON schema, the CDX SDK object pages; field mapping as in the public mzDraw writers); not tested against ChemDraw or Ketcher, and nothing claims their fidelity. One molecule holds every atom and bond (a salt is one KET molecule or one CDXML fragment), `written_bonds()` gives the wedges (narrow end first), the title is one plain text label under the structure.
- KET: `{"root":{"nodes":[{"$ref":"mol0"}, text?]},"mol0":{"type":"molecule","atoms":[{label, alias?, charge, isotope, radical, implicitHCount, location:[x,y,0]}],"bonds":[{type, atoms:[a,b], stereo}]}}`, bond length 1, y up; `stereo` 1 up, 6 down, 4 either (3 on a double bond); `implicitHCount` only where the reader of molfiles would add another count.
- CDXML: `fragment` with `n` (`p`, `Element`, `NumHydrogens`, `Charge`, `Isotope`, `Radical`, a label for every atom but a bare, unlabelled carbon; `Radical` is the CDXML enumeration `Doublet` (one unpaired electron) or `Triplet` (two), never a number: the numeric codes belong to binary CDX, and RDKit's reader ignores them; KET keeps the molfile codes 2 and 3) and `b` (`B`, `E`, `Order`, `Display` `WedgeBegin|WedgedHashBegin|Wavy`), points, y down, 30 to the bond, one bond of margin; object ids are unique (font 1, page 2, fragment 3, atoms from 4, then bonds and the text).
- `choices` log `file_format`, `file_scale` and, when the title has more lines than the one written, `file_title`; an atom that is no element is refused with `err.mol.draw.element` by both writers (KET could carry the label, CDXML cannot, so they agree; never written as carbon); the molfile and SMILES readers do not produce one today; the bytes are fixed (integer-arithmetic numbers, ids in order).
- Tests: `draw_file_tests.rs` (test-only KET and CDXML readers into `mzlab-mol`: the 200 molecules of set A read back with the same core hash, stereo included); `tests/test_mol_draw_file.py` (JSON and XML well-formed; RDKit, only where installed, reads the CDXML as the same molecules); `tests/dati/draw_d1.json` carries `ket` and `cdxml` for every spec (CLI = `.wasm` = app route).

## Edit patch (`chem/edit.rs`, schema `draw_edit/1`)
`--set edit=JSON` (or `@PATH`; the request member `edit` is the text or the list itself) applies a patch to the drawing after it is laid out: a list of operations, or `{"schema":"draw_edit/1","ops":[...]}`, in order, all or none. Atoms keep their id while the patch runs: the index in the drawing it starts from, a new atom takes the next free id (or the `id` given); bonds are named by their two atoms. The final record renumbers the atoms in order. A misspelt key is an error, never ignored.

| op | fields |
|---|---|
| `atom.add` | `el`, optional `id`, `chg`, `iso`, `rad`, `h`, `label`, a position `x`,`y` (bond lengths) or `x_a`,`y_a` (angstrom); without a position the atom is placed once at the end, around the fixed atoms |
| `atom.set` | `id` and any of `el`, `chg`, `iso`, `rad`, `h`, `label` (an empty label clears it) |
| `bond.add`, `bond.set` | `a`, `b`, `o` (1-3, single by default), `wedge` (0, 1, -1, 2: on the first atom `a`); `bond.set` needs `o` or `wedge` |
| `remove` | `atoms` (their bonds go too) and/or `bonds` as `[a,b]` pairs |
| `move` | `atoms` and the shift `dx`,`dy` (bond lengths) or `dx_a`,`dy_a` (angstrom) |
| `merge` | `from`, `into`: `from` is fused into `into`, which keeps its element and charge; two pieces are first moved so that the atoms coincide |

- Units are in the key: a bare `x`, `y`, `dx`, `dy` is in bond lengths (1.0 = 1.5 angstrom, as the mzDraw editor), a name ending in `_a` is angstrom; one operation never mixes them (`units_mixed`).
- Coordinates not touched by the patch stay byte-identical (the layout runs only when an atom has no position, and only the free atoms are taken from it). Hydrogens of an atom whose element, charge, radical or bonds changed are counted again from the lowest allowed valence unless the patch gives `h`.
- Stereo: elements of atoms whose bonds changed are dropped and logged (`choices`: `edit_stereo_dropped`). A `wedge` is applied as a mark on a V2000 molfile and read back by the reader, so the stereo follows its own rule (up and down give the two enantiomers); another mark starting at the same atom goes (`edit_wedge_dropped`). The molfile then shows the writer's choice of wedges (at most 999 atoms and bonds, no stereo groups: `wedge_size`). The hydrogen counts of the record survive the read-back.
- A `label` is a `label` of the scene atom, an alias line of a V2000 molfile (a V3000 one warns `label_not_in_molfile`, value: the number of labelled atoms) and is drawn everywhere else:
  - SVG and PNG show the text instead of the symbol and the hydrogens; a labelled carbon is written too; charge and isotope stay beside it; the text runs away from the bonds with the character next to the bonds on the atom (the first one, or the last one when the bonds come from the right).
  - KET writes it as `alias` beside the element `label` (KET's own place for a shown text, as the molfile alias line); the element is kept, so the file reads back as the same molecule.
  - CDXML writes it as the text of the node, whose `Element` and `NumHydrogens` stay.
  - The PNG stroke font has no glyphs for characters such as `[ ] = ' _ # :` or non-ASCII ones: they are blank in the PNG and real text in the SVG, KET and CDXML. A label on an atom with bonds on both sides runs to the right and may cross a bond. XML 1.0 forbids U+FFFE and U+FFFF: both writers drop them.
- Limits: 1000 operations (`too_many_ops`, `index` 1000), a patch of up to 65536 bytes (`err.mol.draw.editJson`): as text (the `edit=@PATH` file too) or as a list, which is counted as its compact JSON text with a number as 8 bytes, 2500 bonds (`too_many_bonds`), 2000 ids in a list (`bad_field`), 2000 atoms (`err.mol.draw.tooBig`). The same on the site, in the app and in the CLI.
- `choices` (values are data, lists of scene atom indices `i` of the answer, ascending, atoms removed by the patch left out): `edit` (the number of operations), `edit_added`, `edit_placed` (laid out around the fixed atoms), `edit_h_recounted` (hydrogens counted again from the lowest allowed valence), `edit_stereo_dropped` (`i` or `i-j` of a double bond), `edit_wedge_dropped` (bonds `i-j` whose own mark went). `warnings`: `edit_valence` (changed atoms whose bonds, radical and hydrogens go over every allowed valence: accepted, named).
- Failure: `err.mol.draw.edit` with `index` (0-based), `op`, `reason` and `detail` (echoes cut at 64 bytes); reasons `bad_op`, `unknown_op`, `unknown_field`, `missing_field`, `bad_field`, `units_mixed`, `unknown_atom`, `atom_exists`, `unknown_bond`, `bond_exists`, `self_bond`, `unplaced` (a move of an atom not yet placed; a merge leaves such atoms to the layout), `too_many_ops`, `too_many_bonds`, `wedge_size`, `molfile`; nothing is drawn.
- Text that is not a patch, or too long, is `err.mol.draw.editJson` (also an `edit=@PATH` file over the limit or not UTF-8; a missing one is `err.cli.file`, exit 4).
- Not in the language: rings, templates, undo (the mzDraw editor keeps its own inverse commands; its command log is this patch).

## Quality gates (`tests/test_mol_draw_quality.py`, `tools/draw_quality.py`)
Set A `tests/dati/draw_set_a.tsv`: 200 invented molecules in 9 families (chain, monocycle, fused, bridged_spiro, macrocycle, charged, stereo, salt, druglike). Set B `draw_set_b.tsv`: 100 E/Z double bonds with the expected cis/trans. `draw_reference.json`: RDKit's counts on the same metrics (RDKit is needed only to rewrite the sets: `python3 tools/draw_quality.py --set-out --reference`, seed 20261120). A drawing is clean when it has no clash (atoms closer than 0.45 bond), no atom on a bond (0.3), no crossing, bond-length CV under 0.15 and a mean ring-angle error under 10 degrees.

| gate | limit | measured |
|---|---|---|
| set A clean | at least 90 % and, per family, at least RDKit | 184/200 (RDKit 184); bridged_spiro 4 = RDKit 4, macrocycle 20 = 20 |
| set B E/Z wrong | 0/100 | 0/100 |
| molfile read back, same `mol/1` core (stereo included) | 300/300 | 300/300 |
| fragments apart | at least 1.5 bonds | 0 under |
| atom on a bond | at most 9 | 4 |
| native release p95 per molecule | at most 1 ms (best of 3 runs) | 0.18 ms |

What stays unclean: bridged and spiro cage molecules (cubane and adamantane types: ring-angle error 13-20 degrees, some crossings); RDKit draws the same ones unclean. `python3 tools/draw_quality.py --report` prints the table.

## Pictures (`chem/draw_build.rs` figure, `draw_label.rs`, `draw_cip.rs`, `draw_hl.rs`, `draw_svg.rs`, `draw_png.rs`, `draw_font.rs`)
One display list (lines, polygons, discs, stroked text) in pixels, built once from the layout and written twice: as SVG text and as PNG pixels, so both show the same figure. Only `+ - * /` and `sqrt` are used (angles from `mzlab_mol::trig`, numbers written with `mzlab_mol::fixed`), so site, app and CLI give the same bytes. No font file: the text is drawn with an own stroke font (small capitals for lowercase); the PNG is 4x4 supersampled RGB with stored zlib blocks.
- Request keys: `format` (`svg`, `png`), `scale` (pixels per angstrom, 8 to 80, default 24), `cip` (`on`/`off`: R/S marks of stereocentres, E/Z of double bonds, both from the record), `highlight` (`0,1,0-1@red;5@blue`, or a JSON list: atoms, bonds `a-b`, a colour name: red orange yellow green cyan blue purple pink gray), `json` (wrap as `draw-image/1`).
- Labels: carbon is a vertex unless charged, isotopic, alone or labelled by an edit patch; every other atom shows its symbol with its hydrogens (`OH`, `H2N`, `NH3+`), the hydrogens on the side without bonds; bonds stop at the label; wedges are solid, hashed or wavy; CIP marks (R/S/r/s, E/Z) go into the free angle beside their atom or bond; highlights are pale halos under the structure.
- `draw-image/1` (`docs/schemi/draw-image-1.json`) lists every label box in image pixels, so a caller can check overlaps; `tests/test_mol_draw_quality.py` does it on the 200 molecules of set A (no overlap, all inside the image).
- Bounded input: a molfile whose coordinates are not finite or pass 1000 angstrom is laid out again (`choices`: `layout`); a highlight list may hold as many items (atoms and bonds, duplicates counted) as the structure has atoms plus bonds, and a bond must exist in the structure (`err.mol.draw.set`); text of the title is cleaned of characters XML forbids; the PNG needs about twice its pixels in memory (50 MB of pixels at 4096 px a side).
- A picture over 4096 px on a side is drawn at a smaller scale instead (`choices`: `scale_reduced`, no refusal); the `images` of `draw_d1.json` include that case.

## Parity and tests
`tests/dati/draw_d1.json` (`python3 tools/draw_fixture.py`): 30 specs (SMILES, molfile keep/new/origin/3D, db name, approximate name, key, three errors) with the scene, molfile, KET, CDXML, SVG and PNG (length and CRC-32) the release CLI gives.
`images` holds picture requests with options (scale, CIP, highlights, the wrapper): the SVG text, and for a PNG its length and CRC-32. Read by `tests/test_molkit_wasm.py` (node + `.wasm` against the release CLI: bytes, the scene `file` key, and for the errors the key and exit code) and `crates/mzlab-desktop/tests/draw.rs` (route `feat/grafo`: bytes and error keys).

`tests/dati/draw_d3.json` (`python3 tools/draw_d3_fixture.py`): 43 edit patches (every operation, wedges, dropped stereo, a molfile spec, eleven keyed errors, among them the limits) with the scene and the molfile; 15 of them (labels, charge, element, bond order, added atoms, merged and joined fragments, a wedge, a removed atom) also carry the `ket`, `cdxml`, `svg` and `png` of the edited drawing.
Read by the same two tests (`check_draw_fixture` in the first, the route test `draw_route_applies_the_edit_patches_of_the_shared_fixture` in the second); `draw_file_tests.rs` reads edited KET and CDXML back with the test-only readers (same molecule).

The unit tests of `chem/draw_tests.rs`, `chem/edit_tests.rs`, `mol_draw_tests.rs` and `mzlab-mol/tests/layout.rs` use their own inputs.

## Large molecules
The size limit (2000 atoms) is checked before any layout work. The cleaning step counts its own steps and stops at a fixed budget (never a clock, so the bytes stay the same everywhere); when it stops, `choices` carries `layout_budget`. Native release: 650- and 1950-atom polyphenylenes and 1900-atom chains draw in 0.18 s median, 0.26 s at most (before the budget: 23 s and over 300 s).

## Sizes
molkit `.wasm` (`tools/moduli/molkit.toml`, cap 512000, recorded value 369440 on the CI toolchain): the patch language adds 32318 bytes to a build that already holds the KET and CDXML writers (+7.3 %; 440233 to 472551 bytes after wasm-opt -O3, raw 953686 to 1034960, one base commit and one local build); the writers' own share is about 8.2 KB, inside the 440233.
The patch code avoids the standard sort, BTree and float-formatting code, which cost 8 kB and 23 kB more. Catalog `mzlab-cli commands --json` is 32641 bytes (guard 33000 in `catalog_tests.rs`).
