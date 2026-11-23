# Reactivity validation set (`validazione.json`)

Molecule → experimental site, to measure the `crates/mzlab-chem` engine (test: `crates/mzlab-chem/tests/validation.rs`). Only data already in the repository: `docs/literature_cc/` (`experimental_sites.csv`, `isomer_pairs_rt.csv`, `tp_known/tp_known.csv`) and the table of draft CC-05 (`docs/bozze/teoria_cc05.md`). No lab data.

- `molecules`: name → SMILES. The `atoms` of a row are heavy atoms numbered from 0 in SMILES order (like `heavy_index` of `reatt/1`).
- `rows`: `reaction` (`OH`, `O3`, `metabolism`), `scope` (`ch` = unsaturated C–H site, must be in `ch_sites`; `atom` = any heavy atom), `tier` (`literature` = one DOI per row; `draft` = regiochemistry of draft CC-05, no DOI), `confidence` (`high`, `medium`, `draft`), `source`.
- `baseline`: top-1/3/5 counts per `reaction/tier` measured with the current engine; the test compares them one by one. If the engine changes on purpose: `cargo test --release -p mzlab-chem --test validation -- --nocapture`, `BASELINE` line, and update the field.
- `baseline_ph7`: the same counts with `{"protomer":"ph7"}` (pH 7 form, `crates/mzlab-chem/src/protomer.rs`); the test `experimental_sites_ranking_ph7` compares them one by one.
- Excluded (site cannot be derived): paracetamol from `tp_known` (inconsistent product name), transformations without a position (timolol, atenolol, terbuthylazine), triclosan «C6» (inconsistent description), chlorine (no site data in the repository).
