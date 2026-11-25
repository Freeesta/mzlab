# mzlab-rawscan-wasm

wasm-bindgen bridge of `api/rawscan` (`mzlab_core::rawscan`, docs/agenti/raw-access.md): what is in the mean spectrum of a time window that is not in the background
beside it. Per bin the window's mean minus the mean of the scans on its two sides, with the ratio, the net ion counts and a verdict, read from the `.mzidx` in OPFS.
The members, summary and rows are the bytes of `mzlab-cli raw scan`. A `.wasm` of its own, next to `raw` and `rawxic`: fetched by `mzlab/web/rust-raw.js` on the
first `raw scan` request only. Errors are JSON `{"error_key","params"}` strings.
