#!/usr/bin/env bash
# Builds the engine, hr and attributio-all .wasm WITH their name section (wasm-bindgen output, before wasm-opt) into <out>, for `twiggy` (report only, no gate):
# tools/twiggy_wasm.sh <out dir>. Used by .github/workflows/twiggy-wasm.yml; the sizes of the site come from tools/ci_wasm_dimensioni.py.
set -euo pipefail
out=${1:?out dir}
R=target/wasm32-unknown-unknown/release
mkdir -p "$out"
pack() { # <name> <wasm file>
  wasm-bindgen --target web --no-typescript --out-dir "$out/$1" "$2"
  mv "$out/$1/"*_bg.wasm "$out/$1.wasm"
  rm -rf "$out/$1"
}
cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-wasm
pack engine $R/mzlab_wasm.wasm
cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-feat-wasm --no-default-features --features detect,proofs,mrm,finder,finder_edge,finder_msn,flow_ms2
pack hr $R/mzlab_feat_wasm.wasm
cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-attributio-wasm --features all
pack attributio-all $R/mzlab_attributio_wasm.wasm
