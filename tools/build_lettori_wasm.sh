#!/usr/bin/env bash
# The .wasm modules of the readers, one per family of formats (and `valida`, the mzML validator: no reader, its own module) (crates/mzlab-lettori-wasm; each loaded only when a file of its family arrives, never committed), each under its cap (tools/moduli/lettori-*.toml) after
# wasm-opt, and site/static/wasm-lettori/manifest.json with {family: {sha256, byte}} (the worker checks it): tools/build_lettori_wasm.sh <site/static>
# Called by tools/build_feat_wasm.sh, which pages.yml, release.yml and sito-rust.yml already run.
set -eu
out=${1:-site/static}/wasm-lettori
mkdir -p "$out"
. tools/wasm_paralleli.sh   # the wasm-bindgen + wasm-opt of a family run in the background while cargo builds the next one
for n in $(python3 tools/moduli.py --elenco lettori); do
  IFS='|' read -r uscita pacchetto flags tetto opt <<<"$(python3 tools/moduli.py --riga "$n")"
  # shellcheck disable=SC2086
  tools/cargo_wasm.sh build --locked --release --target wasm32-unknown-unknown -p "$pacchetto" $flags </dev/null
  # cargo writes the same file for every family: wasm_pack copies it at once
  wasm_pack "${1:-site/static}/$uscita" target/wasm32-unknown-unknown/release/mzlab_lettori_wasm.wasm "$tetto" "lettori ${n#lettori-}" "$opt"
done
wasm_wait
python3 - "$out" <<'P'
import hashlib, json, os, sys
o = sys.argv[1]
m = {}
for fam in sorted(os.listdir(o)):
    w = f"{o}/{fam}/mzlab_lettori_wasm_bg.wasm"
    if os.path.isfile(w):
        b = open(w, "rb").read()
        m[fam] = {"sha256": hashlib.sha256(b).hexdigest(), "byte": len(b)}
json.dump(m, open(f"{o}/manifest.json", "w"), indent=1)
P
