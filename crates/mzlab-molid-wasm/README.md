# mzlab-molid-wasm

wasm-bindgen bridge of the catalogue identity of a drawing: a molfile or SMILES and the catalogue CSV give `molid/1` (`mzlab_feat::chem::identity`), the catalogue entries that are the drawing (exact) or share its skeleton (near, with what differs).
A `.wasm` of its own (it carries the `mol/1` record of `mzlab-mol`), loaded by `mzlab/web/disegno-identita.js` on the first «Is this in the catalogue?»; the app answers natively (`feat/grafo`, `kind: "identity"`) and the CLI is `mzlab-cli mol-id`.
