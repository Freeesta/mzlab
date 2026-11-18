# Panel titles

A title says only what the panel shows, never a file name (the legend and the file list name the samples). The rules live in `mzlab/web/titoli.js` (`TITOLI.nome`, pure; `TITOLI.di(p)` reads the state) and are tested by `tests/test_titoli.py` and `tests_e2e/e2e_titoli.py`. A title typed by the student stays as it is; default titles are saved as codes (`@ms2tic`, `@ms2prec:X`, `@product:X`), never as translated text.

| Panel | Signal | Title (EN) |
|---|---|---|
| Chromatogram | TIC / BPC | `TIC` / `BPC` (with precursor: `TIC · precursor X`) |
| Chromatogram, MS² tab | file with a single precursor | `TIC · MS²` |
| Chromatogram, MS² tab | several precursors, none chosen | `TIC · MS² (all precursors)` |
| Chromatogram, MS² tab | several precursors, one chosen | `TIC · MS² · precursor X` |
| PDA chromatogram | all λ / one / a range | `PDA (all λ)` / `PDA 254 nm` / `PDA 250–260 nm` |
| XIC | one trace / several traces | `XIC · label` / `Extracted ion (XIC)` |
| MRM, SIM | one transition / all | `MRM · Q1>Q3` (`SIM · …`) / `MRM` (`SIM ions`) |
| Spectrum | MS¹ / MS² without precursor / MS² with precursor | `MS¹` / `MS²` / `MS² of X` (time and energy stay in the bar) |
| UV spectrum | at one time | `UV spectrum` |
| Map | m/z / λ | `RT-m/z map` / `RT-λ map (PDA)` |

Spectrum that follows a chromatogram with several overlaid files: the secondary label "spectrum of <file>" sits in the spectrum header (class `.pscan`), not in the chromatogram title. The file name of an export adds precursor (`precX`) and λ (`PDA-254nm`).
