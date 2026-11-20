# mzlab-ticfondo-wasm

wasm-bindgen bridge of `api/tic_background` (`mzlab_core::ticbg_route`): the TIC or BPC without the chemical background, from the `.mzidx` of a file and of its blank.
A `.wasm` of its own, fetched by the engine worker on the first request. Errors are JSON `{"error_key","params"}` strings.
