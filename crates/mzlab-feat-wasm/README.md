# mzlab-feat-wasm

wasm-bindgen bridge of `mzlab-feat`: detection, alignment, hypotheses and proofs.
Runs only inside the Web Worker of mzFinder (`TP_Mine/files/feat-worker.js`), as a `.wasm` of its own; steps are shared with the native `run` module.
Area file: `docs/agenti/mzfeat-wasm.md`.
