# Casi di Test Nomi TP (Transformation Products)

Questa cartella contiene i dataset di test e gli script per la nomenclatura dei TP.

## Contenuto dei file
- `trasformazioni.csv`: 120 reazioni plausibili su 8 molecole madri (SMILES, formule neutrali e variazioni nette).
- `casi_nomi.csv`: 110 casi con formule TP, addotti ([M+H]+, [M-H]-), masse esatte/nominali, RT sintetici (`rt_min`), gruppi e categorie (`single`, `isomers`, `different_formula`).
- `genera_casi.py`: script deterministico che riproduce byte-per-byte entrambi i CSV e ne verifica la corrispondenza esatta.
- `controlla_casi.py`: script di validazione che verifica vincoli chimici, coerenza masse, assenza di collisioni errate e integrità dei gruppi.

## Metodologia e Dati Sintetici
- **Masse esatte**: calcolate con `pyteomics.mass` (massa monoisotopica esatta dell'addotto specificato, carica +1 o -1, arrotondata a 5 decimali).
- **Masse nominali**: `mz_nominal = round(mz_exact)`.
- **Tempi di ritenzione (`rt_min`)**: valori sintetici deterministici in intervallo plausibile (inclusi casi con RT inverso rispetto a m/z).

## Esecuzione e Verifica
```bash
KMP_DUPLICATE_LIB_OK=TRUE python3 tests/data/nomi_tp/genera_casi.py
KMP_DUPLICATE_LIB_OK=TRUE python3 tests/data/nomi_tp/controlla_casi.py
```
