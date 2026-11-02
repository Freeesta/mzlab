# mzlab-rotte-wasm

wasm-bindgen bridge of the routes the engine `.wasm` leaves out so the first download and compile stay small: `stat_post` (statistics of the series, `api/stat/...`, `api/kmd`), `molecules`, `campioni`, `flow_command`, `flow_line`, `flow_sched`.
A `.wasm` of its own (`wasm-feat/rotte/`, key `rotte` of `manifest.json`), fetched and SHA-256 checked by `rust-worker.js` (the `load` of `mzlab/web/rust-msngrafo.js`) on the first request of one of them; the desktop app answers the same routes natively from `mzlab-core`.
