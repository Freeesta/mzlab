# Periodic table for MS
Dialog «Periodic table» (`mzlab/web/tavola-periodica.js`, opened by `tables.js`): tiles by element class with symbol, atomic number, class code and monoisotopic mass; card with average and monoisotopic mass and the isotope table; legend that dims by class; arrow-key navigation.
- Data: `mzlab-core::periodic` (`elements/1`, the ONE place of names and classes; masses and abundances from `nuclides`/`isoscore`). Same bytes from `mzlab-cli elements --json`, the route `api/elements` (`.wasm` `motore`, op `lr_elements`) and the app; `parity_routes.rs` compares app and wasm.
- Keys: `mono` (nuclide table where the abundance tables lack the element), `avg` and `isotopes` only with abundances, `mono_only` (one isotope), `m2_rel` (M+2 / main, from 0.04).
- Colour is never the only signal: class letter code, thin pattern, Okabe-Ito hues; `tests_e2e/e2e28.py` checks contrast, `e2e_tavola_fissa.py` the fixed layout.
