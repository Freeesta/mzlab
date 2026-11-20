# Transformation record `trasf/1`: decided, partly done

Implemented in `crates/mzlab-mol/src/transform.rs`, `transform_chain.rs`, `delta.rs` (tests: `crates/mzlab-mol/tests/transform.rs`, summary in `docs/agenti/mol.md`). Wired to no consumer yet.

## Fields (JSON, `format: "trasf/1"`)
`parent`, `product` (core hashes), `delta` (`+O`, `-H2+O`), `rule`, `rule_set`, `rule_version`, `map` (`[parent atom, product atom or null]`), `added`, `centre`, `sites` (`{atom, p}`), `region` (`{atoms, kind}`), `stereo_rule`, `level`, `evidence` (`{kind, mz, atoms, on}`).

## Rules
- References, not copies: parent and product are separate `mol/1` records named by the SHA-256 of the canonical core (atoms, bonds, stereo, groups); redrawing does not change identity.
- `map` is written on the canonical indices of the two records, never on SMILES or molfile order.
- Certain product: `product` + complete `map` + `added` + `centre`, no `sites`, no `region`.
- Uncertain product (level 3): no `product`, no `map`, `added` or `centre`; `delta` + `sites` (distinct atoms, p in (0, 1]) + a `region` that includes them. A single structure is never invented.
- Chains compose maps; `null` means the atom is lost and stays lost.
- Stereo: a stereocentre at `centre` becomes unknown in the product unless `stereo_rule` is `retention` or `inversion`.
- Fragments and neutral losses are substructures of the product, on canonical indices; `map` carries them back to the parent.

## Do not
- Write indices of SMILES or molfile, or copy the structure into the record.
- Give a `product` to a level 3 transformation, or draw a "most likely" site as certain.
- Put Fukui values into `p` for C-H positions (only S and N heteroatoms where the validation allows).
- Take BioTransformer or patRoon rule files, or translate their code, without a licence check.

## Consumers still to wire
`interp/1` (read the record instead of rebuilding the delta), `posiz/1` (sites on canonical indices), Draw ("Use this structure" writes `mol/1` plus `trasf/1`); no `.wasm`, no CLI route.

## Next step
Wire `interp/1` and `posiz/1` to read `trasf/1`, then add the `.wasm` and a CLI route (or state the technical limit).
