# mzlab-mol-wasm

wasm-bindgen bridge of the molecule kit of Draw on the mol/1 record of `mzlab-mol` (linked once): `check` gives `check/1` (`mzlab_feat::chem::check`).
A `.wasm` of its own (`wasm-feat/molkit/`), loaded by `mzlab/web/disegno-verifica.js` only when the user asks for the check; `mzlab-cli mol-check` and the app give the same bytes.
