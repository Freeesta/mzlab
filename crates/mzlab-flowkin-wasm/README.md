# mzlab-flowkin-wasm

wasm-bindgen bridge of the Flow step `quant.kinetics@1`; the text comes from `mzlab_ops::run::table::kinetics::kinetics_text`, the same function `flow run` calls, so site and CLI write the same bytes.
A `.wasm` of its own (`flowkin`), loaded by the browser Flow only when the step runs. Errors are JSON `{"error_key","params"}` strings.
