# mzlab-mol-wasm

wasm-bindgen bridge of the drawing check of Draw: a molfile gives `check/1` (`mzlab_feat::chem::check`, on the mol/1 record of `mzlab-mol`).
A `.wasm` of its own (`wasm-feat/molcheck/`), loaded by `mzlab/web/disegno-verifica.js` only when the user asks for the check; `mzlab-cli mol-check` and the app give the same bytes.
