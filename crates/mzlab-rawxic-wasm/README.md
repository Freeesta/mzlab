# mzlab-rawxic-wasm

wasm-bindgen bridge of `api/rawxic` (`mzlab_core::rawxic`, docs/agenti/raw-access.md): at this mass and time, is there a peak? Per m/z the trace of the window goes
through the `quant targets` integrator and the ion counts of the file to a verdict `peak`, `trace`, `nothing` or `no_data`, read from the `.mzidx` in OPFS. The rows are
the bytes of `mzlab-cli raw xic`. A `.wasm` of its own, next to `raw` (`raw info` stays there): fetched by `mzlab/web/rust-raw.js` on the first `raw xic` request only.
Errors are JSON `{"error_key","params"}` strings.
