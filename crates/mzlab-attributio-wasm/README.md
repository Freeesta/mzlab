# mzlab-attributio-wasm

wasm-bindgen bridge of the Attributio table of levels (`mzlab-feat`, feature `attributio`).
A `.wasm` of its own, loaded by mzFinder in administrator mode when a campaign folder is chosen.
The page hands it the campaign files by `read(k, offset, length)`; errors are JSON `{error_key, params}`.
Area file: `docs/agenti/attributio.md`.
