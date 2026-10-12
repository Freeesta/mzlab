#!/usr/bin/env bash
# The two .wasm of mzFinder (crates/mzlab-feat-wasm; fetched only when a series is analysed, never committed), each under 450 KB after wasm-opt -Oz,
# and site/static/wasm-feat/manifest.json with their SHA-256 (the worker checks it): tools/build_feat_wasm.sh <site/static>
set -eu
out=${1:-site/static}/wasm-feat
for which in detect proofs; do
  cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-feat-wasm --no-default-features --features $which
  dir=$out/$([ $which = detect ] && echo feat || echo proofs)
  wasm-bindgen --target web --no-typescript --out-dir "$dir" target/wasm32-unknown-unknown/release/mzlab_feat_wasm.wasm
  tools/wasm_opt.sh "$dir/mzlab_feat_wasm_bg.wasm" 460800 "mzFinder $which"
done
# the structure graph, loaded only when the user gives a SMILES: crates/mzlab-feat-grafo-wasm
cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-feat-grafo-wasm
wasm-bindgen --target web --no-typescript --out-dir "$out/grafo" target/wasm32-unknown-unknown/release/mzlab_feat_grafo_wasm.wasm
tools/wasm_opt.sh "$out/grafo/mzlab_feat_grafo_wasm_bg.wasm" 460800 "mzFinder grafo"
python3 - "$out" <<'P'
import hashlib, json, sys
o = sys.argv[1]
sha = lambda d: hashlib.sha256(open(f"{o}/{d}/mzlab_feat_wasm_bg.wasm", "rb").read()).hexdigest()
json.dump({"feat": {"js": "feat/mzlab_feat_wasm.js", "wasm": "feat/mzlab_feat_wasm_bg.wasm", "sha256": sha("feat")},
           "proofs": {"js": "proofs/mzlab_feat_wasm.js", "wasm": "proofs/mzlab_feat_wasm_bg.wasm", "sha256": sha("proofs")}},
          open(f"{o}/manifest.json", "w"), indent=1)
P
