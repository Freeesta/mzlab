# `mol draw`: own 2D layout, the `draw/1` scene and the edit patch (`mzlab-mol::layout`, `mzlab-feat::chem::draw`, `chem::edit`)

A structure written as text (SMILES, molfile, a name or InChIKey of the molecule database) becomes a drawing: coordinates in angstrom, bond length 1.5, the same on the site (molkit `.wasm`), in the app (`feat/grafo`, `kind:"draw"`) and in the CLI (`mzlab-cli mol draw`). No code of other toolkits; RDKit is only the yardstick of the tests (`tools/draw_quality.py`, never a dependency).

## Layout (`crates/mzlab-mol/src/layout/`)
- `Mol::layout_2d()` and `layout_2d_fixed(&[Option<[f64;2]>])` (atoms given are kept, the rest is placed around them). Deterministic: no random numbers, no libm (`trig.rs` has its own `sincos`/`angle`), coordinates rounded to 4 decimals, so native and `.wasm` bytes are equal.
- `rings.rs`: SSSR and ring systems. `place.rs`: a ring system as regular polygons fused along shared bonds; bridges on arcs; a spiro ring hung on its atom; a ring of 8 or more atoms as one large polygon (a defined trans double bond in it gets a kink, `G::kink_at`). `grow.rs`: substituents and chains (120-degree zigzag, linear atoms straight, a crowded centre spread evenly). `clean.rs`: `fix_ez` redraws a defined double bond with the written cis/trans, then `tidy` pushes apart overlaps with a penalty search (atom pairs, atom on bond, crossings).
- Fragments (salts, mixtures): laid out one by one, left to right, largest first, 2 bond lengths between bounding boxes (the gate asks 1.5).
- Coordinates never decide tetrahedral stereo: `mol/1` keeps `par` and `refs`; `to_molfile` chooses the wedges from the reader's own parity (`Mol::written_bonds()` lists the bonds as written). E/Z is as written: for double bonds in rings of 8 or more atoms or outside rings the layout draws the `cis`/`trans` of the record.
- Reader rule that shapes the tests: a double bond in a ring of up to 7 atoms has no stereo element; elsewhere an unmarked double bond with substituents reads back as defined from the coordinates, so a round trip through a molfile is the same record only when the SMILES marks it. The sets avoid unmarked ones.

## The request and the scene (`chem/draw.rs`)
`draw_json(request)`, request `{"spec", "format":"molfile"|"scene", "layout":"keep"|"new", "db": CSV text, "name": title}`; answer: the scene (`draw/1`, `docs/schemi/draw-1.json`) or the molfile text.

| spec | meaning |
|---|---|
| `smiles:...` | a SMILES |
| `molfile:...`, a path or `-` whose text has `M  END` | a V2000/V3000 molfile; its 2D coordinates are kept (`layout=keep`, default); `layout=new` lays it out again; 3D coordinates or all atoms at the origin are laid out |
| `name:...`, `inchikey:...`, `InChI=...` | looked up in the `molecules/1` CSV of `--set db=PATH` (e.g. `mzlab/web/molecole/catalogo.csv`); an InChI string is not parsed: the hint says to give its InChIKey |
| no prefix | SMILES characters only: a SMILES; `InChI=` or 27 characters `XXXXXXXXXXXXXX-XXXXXXXXXX-X`: a key; anything else: a name |

Scene: `{schema, spec_kind, title, atoms:[{i,el,chg,iso,h,x_a,y_a}], bonds:[{a,b,o,wedge}], fragments, choices, warnings}`. `wedge`: 0 none, 1 up, -1 down, 2 either, on the first atom of the bond; bond orders are Kekule. `choices` logs every automatic decision (`spec_kind`, `layout`, `molfile_version`, `name_hit`/`key_hit` with its score); `warnings`: `name_approximate` (the name matched only as part of another), `name_ambiguous` (several names tie), `key_ambiguous`
(an InChIKey with several rows in the db; the first is drawn, the names are listed). In molfile mode the CLI prints them on stderr as `choice: key = value` and `warning: ...`.

## CLI (`crates/mzlab-cli/src/mol_draw_cmd.rs`)
`mzlab-cli mol draw SPEC [--set format=molfile|scene] [--set layout=keep|new] [--set db=PATH] [--set name=TITLE] [--set edit=JSON|@PATH] [--out PATH] [--json]`. Plain output is the molfile; `--json` is the scene (`--json` with `--set format=molfile` is a usage error). SPEC is `-` for stdin; a path that exists is read.

| exit | meaning |
|---|---|
| 0 | ok |
| 1 | over 2000 atoms (`err.mol.draw.tooBig`, data, as every `*.tooBig`) |
| 2 | unparseable SMILES, bad `--set`, a name/key without `--set db` (the hint names the flag), an InChI string, an edit patch that is not JSON or whose operation fails |
| 4 | missing or unreadable file, db or `edit=@PATH`, a molfile or request that cannot be read (`err.mol.draw.molfile\|request`), a file with no `M  END` |
| 5 | not in the db (`err.mol.draw.not_found`, hint: the nearest names) |
| 6 | unwritable `--out` |

Under `--json` a failure is one `error/1`. Error keys: `err.mol.draw.request|set|smiles|molfile|needDb|inchi|db|not_found|tooBig|edit|editJson` (limit 2000 atoms), texts in `lang/en/errori-a-m.js`.

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
- A `label` is a `label` of the scene atom and an alias line of a V2000 molfile (a V3000 one warns `label_not_in_molfile`, value: the number of labelled atoms).
- Limits: 1000 operations (`too_many_ops`, `index` 1000), a patch given as text up to 65536 bytes (`err.mol.draw.editJson`; the `edit=@PATH` file too), 2500 bonds (`too_many_bonds`), 2000 ids in a list (`bad_field`), 2000 atoms (`err.mol.draw.tooBig`). The same on the site, in the app and in the CLI.
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

## Parity and tests
`tests/dati/draw_d1.json` (`python3 tools/draw_fixture.py`): 30 specs (SMILES, molfile keep/new/origin/3D, db name, approximate name, key, three errors) with the scene and molfile the release CLI gives. Read by `tests/test_molkit_wasm.py` (node + `.wasm` against the release CLI: bytes, and for the errors the key and exit code) and `crates/mzlab-desktop/tests/draw.rs` (route `feat/grafo`: bytes and error keys).

`tests/dati/draw_d3.json` (`python3 tools/draw_d3_fixture.py`): 37 edit patches (every operation, wedges, dropped stereo, a molfile spec, eleven keyed errors, among them the limits) with the same two answers, read by the same two tests (`check_draw_fixture` in the first, the route test `draw_route_applies_the_edit_patches_of_the_shared_fixture` in the second).

The unit tests of `chem/draw_tests.rs`, `chem/edit_tests.rs`, `mol_draw_tests.rs` and `mzlab-mol/tests/layout.rs` use their own inputs.

## Large molecules
The size limit (2000 atoms) is checked before any layout work. The cleaning step counts its own steps and stops at a fixed budget (never a clock, so the bytes stay the same everywhere); when it stops, `choices` carries `layout_budget`. Native release: 650- and 1950-atom polyphenylenes and 1900-atom chains draw in 0.18 s median, 0.26 s at most (before the budget: 23 s and over 300 s).

## Sizes
molkit `.wasm` (`tools/moduli/molkit.toml`, cap 512000 unchanged), the patch language adds 31979 bytes (388199 to 420178, +8.2 %, after wasm-opt -O3; raw 842994 to 922625) on one base commit and one local build; it avoids the standard sort and BTree code, which cost 8 kB more. Catalog `mzlab-cli commands --json` 32489 bytes (guard 33000 in `catalog_tests.rs`).
