Read docs/coda/BRIEF.md.

# PARENT-CARD: The parent card as a Flow input

**Goal.** One Flow input `parent`: from SMILES, InChI, molfile or a drawing, a card with formula, monoisotopic mass, expected adducts per polarity, isotope pattern, logP and pKa labelled "prediction", and an optional reference MS2 (file, SHA-256, scan). CLI: `mzlab-cli mol parent INPUT --json` prints the same `parent/1`.

**Why.** mzflow study: the owner builds a TP feature table step by step and sees each step. Study: `docs/studies/MZFLOW_STUDY.md` (row 4 of §5, §6 decisions) and `docs/studies/MZFLOW_SCREENS.md` §A (left column).

**Read first.** `docs/agenti/mol-draw.md`, `docs/agenti/reactivity.md` (protomer, typical pKa), `crates/mzlab-feat/src/chem/descriptors.rs`, `crates/mzlab-core/src/adduct_table.rs`, `mzlab_core::ionpattern`, `docs/recipes/kinetic-lr.parent.json`.

**Already on main (checked 26 Nov, re-check before coding).** `mol draw`, `mol measure`, `ion mz`, `ion pattern` and the adduct calculator exist; recipes take `parent.json` with a formula only.

**Done means.** 1. `parent/1` built in Rust from the existing functions (no new chemistry), every predicted value marked as such. 2. Steps that take a parent today still read the old `parent.json` (no byte change). 3. Site card with Draw, wasm + native + CLI byte test, agent catalogs.

**Out of scope.** Two parents; computed pKa.

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
