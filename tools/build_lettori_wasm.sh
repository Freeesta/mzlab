#!/usr/bin/env bash
# The .wasm modules of the readers, one per family of formats (and `valida`, the mzML validator: no reader, its own module) (crates/mzlab-lettori-wasm; each loaded only when a file of its family arrives, never committed), each under its limit after
# wasm-opt, and site/static/wasm-lettori/manifest.json with {family: {sha256, byte}} (the worker checks it): tools/build_lettori_wasm.sh <site/static>
# Called by tools/build_feat_wasm.sh, which pages.yml, release.yml and sito-rust.yml already run.
set -eu
# family limit (bytes after wasm-opt; tests/test_lettori_wasm.py checks the table: no value goes up)
# mzlab-picchi at opt-level 3 (peaks 3.1x) moved agilent, sciex and bruker up by 79-228 B, rounded to 256 B
# zlib-rs (flate2: inflate x2.1 in wasm) weighs +30 KB per family that converts: andi, agilent, sciex, waters, shimadzu and bruker moved up by that, measured value rounded to 256 B
# every family carries the indexed-mzML writer (feature `indice` of mzlab-core, +1.9 to +3.2 KB) so that to_mzml is byte for byte the mzML of `mzlab-cli convert` (tests/test_lettori_parita.py): the limits of the line above moved up to the size measured by the CI, rounded to 256 B
tetti="aperti 561152
andi 344064
thermo 774144
agilent 606208
sciex 475136
waters 380928
shimadzu 380928
bruker 532480
valida 479744"
# the families that have readers (a family without readers is skipped and is born with its own package)
con_lettori="agilent sciex thermo aperti andi waters shimadzu bruker valida"
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
  wasm_pack "$out/$fam" target/wasm32-unknown-unknown/release/mzlab_lettori_wasm.wasm "$limit" "lettori $fam" @estesi
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
