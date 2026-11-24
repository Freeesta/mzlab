# mzlab-raw-wasm

wasm-bindgen bridge of `api/rawinfo` (`mzlab_core::rawinfo`, docs/agenti/raw-access.md): what one sample of a raw file holds (polarity, RT, scans per level,
scan types, mass range, m/z grid, ion quantum, spectrum type, mass offset on request), read from its `.mzidx` in OPFS. The members are the bytes of `mzlab-cli raw info`.
A `.wasm` of its own (the engine has no room for it), fetched by `mzlab/web/rust-raw.js` on the first request. Errors are JSON `{"error_key","params"}` strings.
