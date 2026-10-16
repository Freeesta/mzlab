#!/usr/bin/env bash
# The .wasm modules of the readers, one per family of formats (crates/mzlab-lettori-wasm; each loaded only when a file of its family arrives, never committed), each under its limit after
# wasm-opt -Oz, and site/static/wasm-lettori/manifest.json with {family: {sha256, byte}} (the worker checks it): tools/build_lettori_wasm.sh <site/static>
# Called by tools/build_feat_wasm.sh, which pages.yml, release.yml and sito-rust.yml already run.
set -eu
# family limit (bytes after wasm-opt -Oz; tests/test_lettori_wasm.py checks the table: no value goes up)
tetti="aperti 401408
andi 262144
thermo 614400
agilent 458752
sciex 327680
waters 327680
shimadzu 327680
bruker 524288"
# the families that have readers (a family without readers is skipped and is born with its own package)
con_lettori="agilent sciex thermo aperti andi waters shimadzu bruker"
out=${1:-site/static}/wasm-lettori
mkdir -p "$out"
. tools/wasm_paralleli.sh   # the wasm-bindgen + wasm-opt of a family run in the background while cargo builds the next one
while read -r fam limit; do
  case " $con_lettori " in
    *" $fam "*) ;;
    *) echo "lettori $fam: no readers yet, skipped"; continue ;;
  esac
  cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-lettori-wasm --no-default-features --features "$fam" </dev/null
  # cargo writes the same file for every family: wasm_pack copies it at once
  wasm_pack "$out/$fam" target/wasm32-unknown-unknown/release/mzlab_lettori_wasm.wasm "$limit" "lettori $fam" --converge --low-memory-unused --strip-producers --vacuum --dae --merge-blocks --reorder-functions --duplicate-function-elimination --const-hoisting --simplify-globals-optimizing
done <<EOF
$tetti
EOF
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
