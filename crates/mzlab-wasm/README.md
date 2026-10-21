# mzlab-wasm

wasm-bindgen bridge of `mzlab-core`, the engine of the site.
Runs only inside a Web Worker; every answer is written into a `Float64Array` supplied by the caller. Errors are JSON `{error_key, params}`.
Area file: `docs/agenti/rust-rotte.md`.
