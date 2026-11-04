# Build prompts for `docs/studi/OTTIMIZZAZIONE_CODICE.md`

Each file is a prompt ready for a DEV chat: copy everything after the «---» line. Files inside a wave are disjoint; `tools/build_feat_wasm.sh` (02) and `mzlab/web/carica.js` (03) have one owner per wave. Launch wave 2 after wave 1 is merged.

| Wave | Prompt | Zone | Gain (measured) | VISIBLE |
|---|---|---|---|---|
| 1 | 01 | dead Rust `pub fn` + the 8 warnings of the wasm builds | −210 lines, 0 bytes (LTO), warnings 8 → 0 | no |
| 1 | 02 | wasm exports nobody calls (`hr` MRM, `estensioni`, `detect_format`) | `hr` code −31.6 KB pre-opt (est. −22…30 KB site); 9 readers a few hundred B each | no |
| 1 | 03 | `riferimento.js` + `libreria.js` load on first use | start-up −62…64 KB of 1.87 MB | no |
| 2 | 04 | dead JS (`apertura-area.js`, `storia.js`) and CSS (`app.css`) | start-up −1.7 KB, CSS −1.1 KB | no |
| 2 | 05 | one std index sort for the engine's float keys | engine −15…20 KB site (est.; −24…29 KB pre-opt) | no |

Standing rules for every build chat: site = app byte parity; `wasm` caps and `MAX_TOTAL` are never raised (a shrink may propose a lower cap in the PR text, the Coordinator decides); never skip, disable or loosen a test; no `unsafe`; no new dependency; no change of any output byte; PR ready for review, body starts `VISIBLE: no`, 40-char SHA at green CI to the COO; PR events via the `steward` skill.
