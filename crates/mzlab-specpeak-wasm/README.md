# mzlab-specpeak-wasm

wasm-bindgen bridge of `api/specpeak` (`mzlab_core::specpeak`, docs/agenti/data-channels.md): `spectrum get --peak auto` over the `.mzidx` in OPFS. It finds the apex of
the product-ion chromatogram, sums a window of `factor` x FWHM, subtracts the background of both sides (per nominal mass, profile cell or high-resolution line) and answers a verdict (`peak`, `no_apex`, `no_data`, `window_only`)
with the evidence; the bytes are those of `mzlab-cli spectrum get --peak auto`. A `.wasm` of its own, next to `rawxic` (`raw xic` and `spectrum get --sum` stay there):
fetched by `mzlab/web/rust-raw.js` on the first `--peak auto` request only. Errors are JSON `{"error_key","params"}` strings.
