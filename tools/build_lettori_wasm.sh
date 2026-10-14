#!/usr/bin/env bash
# The .wasm of the readers module (crates/mzlab-lettori-wasm: formats and variants the engine does not carry; fetched only when a file needs it, never committed)
# and site/static/wasm-lettori/manifest.json with its SHA-256 (the worker checks it): tools/build_lettori_wasm.sh <site/static>
# Called by the last line of tools/build_feat_wasm.sh, which pages.yml, release.yml and sito-rust.yml already run.
set -eu
# Limits after wasm-opt -Oz, bytes (tests/test_lettori_wasm.py checks them: they do not go up).
limit=1048576
# limit_tdf=655360   # lettori-tdf (Bruker TDF/TSF, with its minimal SQLite and zstd): build it here when the package that writes it lands
out=${1:-site/static}/wasm-lettori
cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-lettori-wasm
wasm-bindgen --target web --no-typescript --out-dir "$out" target/wasm32-unknown-unknown/release/mzlab_lettori_wasm.wasm
tools/wasm_opt.sh "$out/mzlab_lettori_wasm_bg.wasm" $limit lettori
# tools/wasm_opt.sh "$out-tdf/mzlab_lettori_tdf_wasm_bg.wasm" $limit_tdf lettori-tdf
python3 - "$out" <<'P'
import hashlib, json, sys
o = sys.argv[1]
json.dump({"lettori": {"js": "mzlab_lettori_wasm.js", "wasm": "mzlab_lettori_wasm_bg.wasm",
                       "sha256": hashlib.sha256(open(f"{o}/mzlab_lettori_wasm_bg.wasm", "rb").read()).hexdigest()}},
          open(f"{o}/manifest.json", "w"), indent=1)
P
