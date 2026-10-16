#!/usr/bin/env bash
# wasm-opt -Oz on a .wasm of the site, then the size gate: tools/wasm_opt.sh <file> <limit in bytes> <label> [extra wasm-opt flags]
# Used by pages.yml and sito-rust.yml, so the site that is tested is the site that is published.
# wasm-opt is installed once per $RUNNER_TEMP (under a lock, because tools/wasm_paralleli.sh runs several of these at the same time).
set -u
bg=$1; limit=$2; label=$3; shift 3
wo="$RUNNER_TEMP/wo/node_modules/.bin/wasm-opt"
if [ ! -x "$wo" ]; then
  mkdir -p "$RUNNER_TEMP"
  ( flock 9; [ -x "$wo" ] || npm install --prefix "$RUNNER_TEMP/wo" wasm-opt@1.4.0 ) 9>"$RUNNER_TEMP/wo.lock"
fi
if [ -x "$wo" ]; then
  "$wo" -Oz --enable-bulk-memory --enable-sign-ext --enable-mutable-globals --enable-nontrapping-float-to-int "$@" -o "$bg" "$bg" || exit 1
  size=$(stat -c %s "$bg"); echo "$label .wasm (optimised): $size bytes"
  if [ "$size" -ge "$limit" ]; then echo "::error::the $label .wasm is $size bytes (limit $limit)"; exit 1; fi
else
  echo "::warning::wasm-opt could not be installed: the $label .wasm is not optimised ($(stat -c %s "$bg") bytes)"
fi
