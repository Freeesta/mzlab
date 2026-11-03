#!/bin/sh
# Builds what the proxy imports: the `llm` .wasm (crates/mzlab-llm-wasm, the same module and cap as the site) into wasm/ and the allow-list of the
# prompts, schemas and catalog of the repository into allowlist.json. Run by `npx wrangler deploy` ([build] in wrangler.toml); needs cargo, the
# wasm32 target, wasm-bindgen (version of Cargo.lock) and node.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/.." && pwd)
cd "$root"
tools/cargo_wasm.sh build --locked --release --target wasm32-unknown-unknown -p mzlab-llm-wasm
rm -rf "$here/wasm"
wasm-bindgen --target web --no-typescript --out-dir "$here/wasm" target/wasm32-unknown-unknown/release/mzlab_llm_wasm.wasm
limit=$(sed -n 's/^limit_llm=\([0-9]*\).*/\1/p' tools/build_feat_wasm.sh)
RUNNER_TEMP=${RUNNER_TEMP:-$(mktemp -d)} tools/wasm_opt.sh "$here/wasm/mzlab_llm_wasm_bg.wasm" "$limit" "assistant (proxy)" @motore
node "$here/build-allowlist.mjs" "$here/wasm" > "$here/allowlist.json"
