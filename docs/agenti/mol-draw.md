# `mol draw`: own 2D layout, the `draw/1` scene, KET/CDXML files and SVG/PNG pictures (`mzlab-mol::layout`, `mzlab-feat::chem::draw*`, `draw_file`)

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

## CLI (`crates/mzlab-cli/src/mol_draw_cmd.rs`)
`mzlab-cli mol draw SPEC [--format molfile|scene|ket|cdxml|svg|png] [--set layout=keep|new] [--set db=PATH] [--set name=TITLE] [--set scale=PX] [--set cip=on|off] [--set highlight=LIST] [--out PATH] [--json]` (`--format F` is `--set format=F`). Plain output is the molfile, the KET, the CDXML, the SVG text or the PNG bytes; `--json` is the scene, with the KET or CDXML text in its `file` key when `--format` names one, or with svg/png the `draw-image/1` wrapper (`--json` with `--format molfile` is a usage error). SPEC is `-` for stdin; a path that exists is read.

| exit | meaning |
|---|---|
| 0 | ok |
| 1 | over 2000 atoms (`err.mol.draw.tooBig`, data, as every `*.tooBig`), an atom that is no element in KET or CDXML (`err.mol.draw.element`, data, like `error.grafo.element`) |
| 2 | unparseable SMILES, bad `--set`, a name/key without `--set db` (the hint names the flag), an InChI string |
| 4 | missing or unreadable file or db, a molfile or request that cannot be read (`err.mol.draw.molfile\|request`), a file with no `M  END` |
| 5 | not in the db (`err.mol.draw.not_found` with the nearest names in the hint, or `err.mol.draw.not_found_bare` when none is near) |
| 6 | unwritable `--out` |

Under `--json` a failure is one `error/1`. Error keys: `err.mol.draw.request|set|binary|smiles|molfile|needDb|inchi|db|not_found|not_found_bare|tooBig|element` (limit 2000 atoms), texts in `lang/en/errori-a-m.js`.

## KET and CDXML (`chem/draw_file.rs`)
Open formats written by our own code from their public documentation (the KET JSON schema, the CDX SDK object pages; field mapping as in the public mzDraw writers); not tested against ChemDraw or Ketcher, and nothing claims their fidelity. One molecule holds every atom and bond (a salt is one KET molecule or one CDXML fragment), `written_bonds()` gives the wedges (narrow end first), the title is one plain text label under the structure.
- KET: `{"root":{"nodes":[{"$ref":"mol0"}, text?]},"mol0":{"type":"molecule","atoms":[{label, charge, isotope, radical, implicitHCount, location:[x,y,0]}],"bonds":[{type, atoms:[a,b], stereo}]}}`, bond length 1, y up; `stereo` 1 up, 6 down, 4 either (3 on a double bond); `implicitHCount` only where the reader of molfiles would add another count.
- CDXML: `fragment` with `n` (`p`, `Element`, `NumHydrogens`, `Charge`, `Isotope`, `Radical`, a label for every atom but a bare carbon; `Radical` is the CDXML enumeration `Doublet` (one unpaired electron) or `Triplet` (two), never a number: the numeric codes belong to binary CDX, and RDKit's reader ignores them; KET keeps the molfile codes 2 and 3) and `b` (`B`, `E`, `Order`, `Display` `WedgeBegin|WedgedHashBegin|Wavy`), points, y down, 30 to the bond, one bond of margin; object ids are unique (font 1, page 2, fragment 3, atoms from 4, then bonds and the text).
- `choices` log `file_format`, `file_scale` and, when the title has more lines than the one written, `file_title`; an atom that is no element is refused with `err.mol.draw.element` by both writers (KET could carry the label, CDXML cannot, so they agree; never written as carbon); the molfile and SMILES readers do not produce one today; the bytes are fixed (integer-arithmetic numbers, ids in order).
- Tests: `draw_file_tests.rs` (test-only KET and CDXML readers into `mzlab-mol`: the 200 molecules of set A read back with the same core hash, stereo included); `tests/test_mol_draw_file.py` (JSON and XML well-formed; RDKit, only where installed, reads the CDXML as the same molecules); `tests/dati/draw_d1.json` carries `ket` and `cdxml` for every spec (CLI = `.wasm` = app route).

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
- Labels: carbon is a vertex unless charged, isotopic or alone; every other atom shows its symbol with its hydrogens (`OH`, `H2N`, `NH3+`), the hydrogens on the side without bonds; bonds stop at the label; wedges are solid, hashed or wavy; CIP marks (R/S/r/s, E/Z) go into the free angle beside their atom or bond; highlights are pale halos under the structure.
- `draw-image/1` (`docs/schemi/draw-image-1.json`) lists every label box in image pixels, so a caller can check overlaps; `tests/test_mol_draw_quality.py` does it on the 200 molecules of set A (no overlap, all inside the image).
- Bounded input: a molfile whose coordinates are not finite or pass 1000 angstrom is laid out again (`choices`: `layout`); a highlight list may hold as many items (atoms and bonds, duplicates counted) as the structure has atoms plus bonds, and a bond must exist in the structure (`err.mol.draw.set`); text of the title is cleaned of characters XML forbids; the PNG needs about twice its pixels in memory (50 MB of pixels at 4096 px a side).
- A picture over 4096 px on a side is drawn at a smaller scale instead (`choices`: `scale_reduced`, no refusal); the `images` of `draw_d1.json` include that case.

## Parity and tests
`tests/dati/draw_d1.json` (`python3 tools/draw_fixture.py`): 30 specs (SMILES, molfile keep/new/origin/3D, db name, approximate name, key, three errors) with the scene, molfile, KET, CDXML, SVG and PNG (length and CRC-32) the release CLI gives.
`images` holds picture requests with options (scale, CIP, highlights, the wrapper): the SVG text, and for a PNG its length and CRC-32. Read by `tests/test_molkit_wasm.py` (node + `.wasm` against the release CLI: bytes, the scene `file` key, and for the errors the key and exit code) and `crates/mzlab-desktop/tests/draw.rs` (route `feat/grafo`: bytes and error keys). The unit tests of `chem/draw_tests.rs`, `mol_draw_tests.rs` and `mzlab-mol/tests/layout.rs` use their own inputs.

## Large molecules
The size limit (2000 atoms) is checked before any layout work. The cleaning step counts its own steps and stops at a fixed budget (never a clock, so the bytes stay the same everywhere); when it stops, `choices` carries `layout_budget`. Native release: 650- and 1950-atom polyphenylenes and 1900-atom chains draw in 0.18 s median, 0.26 s at most (before the budget: 23 s and over 300 s).

## Sizes
molkit `.wasm`: 369440 bytes recorded (`tools/moduli/molkit.toml`, cap 512000 unchanged); the KET and CDXML writers add about 8.2 KB (+2.1 %: 388117 -> 396314 bytes after wasm-opt, same local toolchain, main ccd4680fc against this tree), so the next recorded measure is about 377.6 KB. Catalog `mzlab-cli commands --json` 32563 bytes (guard 33000 in `catalog_tests.rs`).
