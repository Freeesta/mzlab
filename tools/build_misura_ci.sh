#!/usr/bin/env bash
# CI size step helper: builds the given feature modules (tools/moduli/<name>.toml) the way tools/build_feat_wasm.sh does (cargo_wasm.sh, wasm-bindgen,
# tools/wasm_opt.sh with the cap and wasm-opt set of the toml) into site/static, so the size step measures them after wasm-opt: tools/build_misura_ci.sh <name>...
set -eu
eval "$(python3 -c 'import sys; sys.path.insert(0, "tools"); import prepara_sito; [print(f"export {k}={v!r}") for k, v in prepara_sito.build_env("wasm32-unknown-unknown", commit=False).items()]')"
for n in "$@"; do
  IFS='|' read -r uscita pacchetto flags tetto opt <<<"$(python3 tools/moduli.py --row "$n")"
  # shellcheck disable=SC2086
  tools/cargo_wasm.sh build --locked --release --target wasm32-unknown-unknown -p "$pacchetto" $flags
  wasm-bindgen --target web --no-typescript --out-dir "site/static/$uscita" "target/wasm32-unknown-unknown/release/${pacchetto//-/_}.wasm"
  tools/wasm_opt.sh "site/static/$uscita/${pacchetto//-/_}_bg.wasm" "$tetto" "$n" "$opt"
done
