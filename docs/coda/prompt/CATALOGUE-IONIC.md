Read docs/coda/BRIEF.md.

# CATALOGUE-IONIC: charged pesticides and salts in the built-in list

**Goal.** Add the pesticides that `tools/download_env_compounds.py` could not take because PubChem gives a salt, a cation or several components (diquat, paraquat, chlormequat, mepiquat, dodine, fosetyl, ziram, propineb, ...), as charged species with their own formula, charge and m/z.

**Why.** They are common in EU monitoring and are found as ions, not as neutrals; today a search for them finds nothing.

**Read first.** `grep -n 'molecules' docs/agenti/INDICE.md`, then `docs/agenti/molecules.md` (catalogue, inclusion rule, sources), `tools/download_env_compounds.py` (its stderr names every unresolved row), `tools/generate_molecules.py`, `tests/data/catalogue_must_have.txt`.

**Already on main (checked 27 Nov, re-check before coding).** `mzlab/chem/env_compounds.csv` and the generator; records have a `charge` column; the screening list takes `[M]+` ions only through the adducts column.

**Done means.** Each added species has a PubChem-backed formula, SMILES, InChIKey, charge and the adduct list that fits it (`[M]+` for a cation); the names are in the must-have test; `tests/test_catalogue_env.py` passes; the docs say how a cation is listed.

**Out of scope.** User-added compounds.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
