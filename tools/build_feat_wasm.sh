#!/usr/bin/env bash
# The .wasm of mzFinder (crates/mzlab-feat-wasm: detect, proofs and mrm, the last one loaded only for a series of MRM files; fetched only when a series is analysed, never committed), each under 450 KB after wasm-opt -Oz,
# and site/static/wasm-feat/manifest.json with their SHA-256 (the worker checks it): tools/build_feat_wasm.sh <site/static>
set -eu
out=${1:-site/static}/wasm-feat
for which in detect proofs mrm; do
  cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-feat-wasm --no-default-features --features $which
  dir=$out/$([ $which = detect ] && echo feat || echo $which)
  wasm-bindgen --target web --no-typescript --out-dir "$dir" target/wasm32-unknown-unknown/release/mzlab_feat_wasm.wasm
  tools/wasm_opt.sh "$dir/mzlab_feat_wasm_bg.wasm" 460800 "mzFinder $which"
done
# the third one, loaded only when a table is shown or exported (views, report, CSV, MGF, .ms): crates/mzlab-feat-table-wasm
cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-feat-table-wasm
wasm-bindgen --target web --no-typescript --out-dir "$out/table" target/wasm32-unknown-unknown/release/mzlab_feat_table_wasm.wasm
tools/wasm_opt.sh "$out/table/mzlab_feat_table_wasm_bg.wasm" 460800 "mzFinder table"
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
m = json.load(open(f"{o}/manifest.json"))
m["table"] = {"js": "table/mzlab_feat_table_wasm.js", "wasm": "table/mzlab_feat_table_wasm_bg.wasm",
              "sha256": hashlib.sha256(open(f"{o}/table/mzlab_feat_table_wasm_bg.wasm", "rb").read()).hexdigest()}
m["mrm"] = {"js": "mrm/mzlab_feat_wasm.js", "wasm": "mrm/mzlab_feat_wasm_bg.wasm", "sha256": sha("mrm")}
m["grafo"] = {"js": "grafo/mzlab_feat_grafo_wasm.js", "wasm": "grafo/mzlab_feat_grafo_wasm_bg.wasm",
              "sha256": hashlib.sha256(open(f"{o}/grafo/mzlab_feat_grafo_wasm_bg.wasm", "rb").read()).hexdigest()}
json.dump(m, open(f"{o}/manifest.json", "w"), indent=1)
P
