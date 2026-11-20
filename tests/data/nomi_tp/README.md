# TP Name Test Cases (Transformation Products)

This folder holds the test datasets and the scripts for TP nomenclature.

## Contents
- `trasformazioni.csv`: 120 plausible reactions on 8 parent molecules (SMILES, neutral formulas and net changes).
- `casi_nomi.csv`: 110 cases with TP formulas, adducts ([M+H]+, [M-H]-), exact/nominal masses, synthetic RTs (`rt_min`), groups and categories (`single`, `isomers`, `different_formula`).
- `genera_casi.py`: deterministic script that reproduces both CSVs byte for byte and checks that they match exactly.
- `controlla_casi.py`: validation script that checks chemical constraints, mass consistency, absence of wrong collisions and group integrity.

## Method and synthetic data
- **Exact masses**: computed with `pyteomics.mass` (exact monoisotopic mass of the given adduct, charge +1 or -1, rounded to 5 decimals).
- **Nominal masses**: `mz_nominal = round(mz_exact)`.
- **Retention times (`rt_min`)**: deterministic synthetic values in a plausible range (including cases with RT inverse to m/z).

## Run and check
```bash
KMP_DUPLICATE_LIB_OK=TRUE python3 tests/data/nomi_tp/genera_casi.py
KMP_DUPLICATE_LIB_OK=TRUE python3 tests/data/nomi_tp/controlla_casi.py
```
