# Decoy Parent Library and Transformation Rules (T15)

## 1. Purpose
This folder holds synthetic/negative reference data for benchmarking and validating the Tabui module of mzLab. The parent compounds included here act as **decoys (negative controls)** to measure the **false positive rate** on experimental or demo spectra:
- The 30 selected parent compounds are well-characterised public molecules (pesticides, drugs, industrial compounds) that are **not present** in the example Full Scan acquisition files of mzLab (`mzlab/web/esempi/FullScan_t*.mzML`).
- When a non-targeted search / screening with Tabui is run with one of these compounds as parent and the transformation rules applied, the expected number of real candidate transformation products (TPs) identified must be **zero** (or confined to background statistical noise with a negligible score).

## 2. Included files
- `decoy_parent.csv`: 30 public decoy compounds with name, canonical SMILES, Hill neutral molecular formula, theoretical monoisotopic mass of `[M+H]+` and `[M-H]-` (computed with Pyteomics, 4 decimals, or `N/A` if not ionisable in ESI-) and the demo-presence flag (`present_in_demo = no`).
- `trasformazioni_decoy.csv`: 15 generic transformation rules (names in plain words without underscores, net formula changes) not duplicated from the `tests/data/nomi_tp/trasformazioni.csv` set.

## 3. Selection and verification criteria
- Monoisotopic mass range strictly between 150 and 600 Da (from 279 Da to 585 Da).
- Instrumental check: for each `mz_pos` and `mz_neg` mass the extracted ion chromatogram (XIC, tolerance $\pm 0.5$ Da) was taken on all FullScan files of the demo time series. No compound shows real chromatographic peaks (all with $S/N \le 8.2$, well below the $S/N = 10$ threshold).
- All calculation and screening scripts were run outside the repository (in `~/Desktop/gemini_cc/script/`).
