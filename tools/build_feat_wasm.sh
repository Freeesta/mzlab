#!/usr/bin/env bash
# The .wasm of mzFinder (crates/mzlab-feat-wasm: detect, proofs, mrm, loaded only for a series of MRM files, and finder, loaded by the worker of mzFinder for classify, formula_info, identity_graph, polarity_pairs and the Dig of a high-resolution series; fetched only when a series is analysed, never committed), each under 500 KB after wasm-opt -Oz,
# and site/static/wasm-feat/manifest.json with their SHA-256 (the worker checks it): tools/build_feat_wasm.sh <site/static>
set -eu
# Limit of every .wasm after wasm-opt -Oz, bytes (raised from 460800 to leave room to grow; the size diet is a package of its own; measured: detect 419435, proofs 458034, mrm 244480).
limit=512000
out=${1:-site/static}/wasm-feat
for which in detect proofs mrm finder; do
  cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-feat-wasm --no-default-features --features $which
  dir=$out/$([ $which = detect ] && echo feat || echo $which)
  wasm-bindgen --target web --no-typescript --out-dir "$dir" target/wasm32-unknown-unknown/release/mzlab_feat_wasm.wasm
  tools/wasm_opt.sh "$dir/mzlab_feat_wasm_bg.wasm" $limit "mzFinder $which"
done
# the third one, loaded only when a table is shown or exported (views, report, CSV, MGF, .ms): crates/mzlab-feat-table-wasm
cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-feat-table-wasm
wasm-bindgen --target web --no-typescript --out-dir "$out/table" target/wasm32-unknown-unknown/release/mzlab_feat_table_wasm.wasm
tools/wasm_opt.sh "$out/table/mzlab_feat_table_wasm_bg.wasm" $limit "mzFinder table"
# the structure graph, loaded only when the user gives a SMILES: crates/mzlab-feat-grafo-wasm
cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-feat-grafo-wasm
wasm-bindgen --target web --no-typescript --out-dir "$out/grafo" target/wasm32-unknown-unknown/release/mzlab_feat_grafo_wasm.wasm
tools/wasm_opt.sh "$out/grafo/mzlab_feat_grafo_wasm_bg.wasm" $limit "mzFinder grafo"
# the readers of the vendor files (.D, .wiff, .raw), loaded only when a dataset is opened through them: crates/mzlab-lettori-wasm
cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-lettori-wasm
wasm-bindgen --target web --no-typescript --out-dir "$out/lettori" target/wasm32-unknown-unknown/release/mzlab_lettori_wasm.wasm
tools/wasm_opt.sh "$out/lettori/mzlab_lettori_wasm_bg.wasm" 600000 "lettori"
# the low-resolution Dig of mzFinder (crates/mzlab-finder-lr-wasm), loaded by the worker of mzFinder only for a series of full-scan unit-resolution files
cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-finder-lr-wasm
wasm-bindgen --target web --no-typescript --out-dir "$out/finder-lr" target/wasm32-unknown-unknown/release/mzlab_finder_lr_wasm.wasm
tools/wasm_opt.sh "$out/finder-lr/mzlab_finder_lr_wasm_bg.wasm" $limit "mzFinder finder-lr"
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
m["finder"] = {"js": "finder/mzlab_feat_wasm.js", "wasm": "finder/mzlab_feat_wasm_bg.wasm", "sha256": sha("finder")}
m["grafo"] = {"js": "grafo/mzlab_feat_grafo_wasm.js", "wasm": "grafo/mzlab_feat_grafo_wasm_bg.wasm",
              "sha256": hashlib.sha256(open(f"{o}/grafo/mzlab_feat_grafo_wasm_bg.wasm", "rb").read()).hexdigest()}
m["lettori"] = {"js": "lettori/mzlab_lettori_wasm.js", "wasm": "lettori/mzlab_lettori_wasm_bg.wasm",
              "sha256": hashlib.sha256(open(f"{o}/lettori/mzlab_lettori_wasm_bg.wasm", "rb").read()).hexdigest()}
m["finder_lr"] = {"js": "finder-lr/mzlab_finder_lr_wasm.js", "wasm": "finder-lr/mzlab_finder_lr_wasm_bg.wasm",
                 "sha256": hashlib.sha256(open(f"{o}/finder-lr/mzlab_finder_lr_wasm_bg.wasm", "rb").read()).hexdigest()}
json.dump(m, open(f"{o}/manifest.json", "w"), indent=1)
P
