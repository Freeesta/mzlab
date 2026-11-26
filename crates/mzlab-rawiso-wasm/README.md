# mzlab-rawiso-wasm

wasm-bindgen bridge of `api/rawiso` (`mzlab_core::rawiso`, docs/agenti/raw-access.md): the net counts of M, M+1 and M+2 over the peak region of an m/z and a time
window, the observed ratios with their counting sigma, and the z scores against the pattern of a formula, read from the `.mzidx` in OPFS. The members and the `result`
are the bytes of `mzlab-cli raw isotopes`. A `.wasm` of its own, next to `raw` and `rawxic`: fetched by `mzlab/web/rust-raw.js` on the first `raw isotopes` request only.
Errors are JSON `{"error_key","params"}` strings.
