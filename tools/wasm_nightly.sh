#!/usr/bin/env bash
# By-hand experiment (NOT the production build): engine, hr and attributio-all built with the stable toolchain and with a dated nightly (flags below), each through
# wasm-bindgen and the site's wasm-opt (tools/wasm_opt.sh @motore, caps not applied), printed as a table of bytes: tools/wasm_nightly.sh <work dir> <nightly, e.g. nightly-2026-10-20>
# Used by .github/workflows/wasm-nightly.yml. The flags changing what a program prints (-Zfmt-debug) are not tried: they would change every {:?} output.
set -euo pipefail
work=$(realpath -m "${1:?work dir}"); nightly=${2:?nightly}
: "${RUNNER_TEMP:=$work/tmp}"; export RUNNER_TEMP
BASE="-C link-arg=--initial-memory=33554432 -C link-arg=--max-memory=4294967296 -C target-feature=+simd128"
HR="detect,proofs,mrm,finder,finder_edge,finder_msn,flow_ms2"
mkdir -p "$work"
rustup toolchain install "$nightly" --profile minimal -c rust-src -t wasm32-unknown-unknown
rows=""
build() { # <variant> <toolchain> <extra rustflags> <extra cargo flags>
  local v=$1 tc=$2 rf=$3 cf=$4
  export CARGO_TARGET_DIR="$work/target-$v"
  local R=$CARGO_TARGET_DIR/wasm32-unknown-unknown/release
  one() { # <name> <crate> <artifact> <cargo args...>
    local name=$1 crate=$2 art=$3; shift 3
    # shellcheck disable=SC2086
    RUSTFLAGS="$BASE $rf" cargo +"$tc" build --locked --release --target wasm32-unknown-unknown $cf -p "$crate" "$@"
    wasm-bindgen --target web --no-typescript --out-dir "$work/$v/$name" "$R/$art.wasm"
    tools/wasm_opt.sh "$work/$v/$name/${art}_bg.wasm" 999999999 "$name/$v" @motore
    rows="$rows$name $v $(stat -c %s "$work/$v/$name/${art}_bg.wasm")"$'\n'
  }
  one engine mzlab-wasm mzlab_wasm
  one hr mzlab-feat-wasm mzlab_feat_wasm --no-default-features --features "$HR"
  one attributio-all mzlab-attributio-wasm mzlab_attributio_wasm --features all
}
build stable "$(sed -n 's/^channel *= *"\(.*\)"/\1/p' rust-toolchain.toml)" "" ""
build nightly "$nightly" "" ""
build loc "$nightly" "-Zlocation-detail=none" ""
build locimm "$nightly" "-Zlocation-detail=none -Zunstable-options -Cpanic=immediate-abort" "-Zbuild-std=std,panic_abort"
printf '%s' "$rows" | python3 tools/wasm_nightly_tabella.py
