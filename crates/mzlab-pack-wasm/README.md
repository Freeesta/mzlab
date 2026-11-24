# mzlab-pack-wasm

wasm-bindgen bridge of `file pack`, `file unpack` and `file check-pack` (`mzlab_core::pack`, `pack_read`, docs/agenti/file-pack.md). The bytes of the parts, the manifest
and the `pack-run/1`, `unpack-run/1` and `verify-pack/1` documents are the ones of the command line: one code, called with JS callbacks that read and write OPFS files.
A `.wasm` of its own, fetched by `mzlab/web/rust-pack.js` on the first use. Errors are JSON `{"error_key","params"}` strings.
