# mzlab-flowcal-wasm

wasm-bindgen bridge of the Flow steps `quant.calibrate@1` and `quant.curve@1`; the texts come from `mzlab_ops::run::table::quant` and `curve`, the same functions `flow run` calls, so site and CLI write the same bytes.
A `.wasm` of its own, loaded by the browser Flow only when one of these steps runs. Errors are JSON `{"error_key","params"}` strings.
