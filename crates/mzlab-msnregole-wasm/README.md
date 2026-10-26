# mzlab-msnregole-wasm

wasm-bindgen bridge of the structure rules of the fragmentation graph: a SMILES gives the expectations `attese` of the rules whose conditions hold (`mzlab_feat::chem::regole`).
A `.wasm` of its own, loaded by `mzlab/web/rust-msngrafo.js` only when the user gives a SMILES; the page passes `attese` to `msn_grafo`.
