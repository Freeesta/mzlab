#!/usr/bin/env bash
# wasm-opt -Oz on a .wasm of the site, then the size gate: tools/wasm_opt.sh <file> <limit in bytes> <label> [extra wasm-opt flags]
# Used by pages.yml and sito-rust.yml, so the site that is tested is the site that is published.
# wasm-opt is binaryen version_132 from the official GitHub tarball (SHA-256 pinned below), installed once per $RUNNER_TEMP
# (under a lock, because tools/wasm_paralleli.sh runs several of these at the same time). If the tarball cannot be used it falls back to npm wasm-opt@1.4.0.
set -u
bg=$1; limit=$2; label=$3; shift 3
# `@estesi` stands for the extra passes below: the ONE list of them, used by the readers (tools/build_lettori_wasm.sh) and by the engine and libraries steps of pages.yml,
# sito-rust.yml, release.yml and notte.yml, so the site that is tested is the site that is published. `--low-memory-unused` assumes nothing reads or writes below address 1024:
# true for every Rust .wasm of the site, since rustc links wasm32-unknown-unknown with `--stack-first` (the data starts at the 1 MiB mark; tests/test_lettori_wasm.py checks it).
estesi="--converge --low-memory-unused --strip-producers --vacuum --dae --merge-blocks --reorder-functions --duplicate-function-elimination --const-hoisting --simplify-globals-optimizing"
args=()
for a in "$@"; do
  # shellcheck disable=SC2206
  if [ "$a" = "@estesi" ]; then args+=($estesi); else args+=("$a"); fi
done
set -- ${args[@]+"${args[@]}"}
mkdir -p "$RUNNER_TEMP"
base="https://github.com/WebAssembly/binaryen/releases/download/version_132"
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)  tarball=binaryen-version_132-x86_64-linux.tar.gz;  sha=195ddc94f9bc89f45abdabb0b9eea86023d727ba90eac8b35b80f2544fc30572; only=binaryen-version_132/bin/wasm-opt ;;
  Darwin-arm64)  tarball=binaryen-version_132-arm64-macos.tar.gz;   sha=98aad827847af7ef990ed7098d885725c8e5b5aae75073403635617ae4e259aa; only= ;;
  Darwin-x86_64) tarball=binaryen-version_132-x86_64-macos.tar.gz;  sha=40c3de90bb3766bd0282a895e139a6f50253dba49b4f5bb89e66faca162d832e; only= ;;
  *) tarball=; sha=; only= ;;
esac
sha256_of() { if command -v sha256sum >/dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1; }

wo="$RUNNER_TEMP/binaryen-132/bin/wasm-opt"
ok="$RUNNER_TEMP/binaryen-132/installato"   # written when the install has finished, so a half-extracted binary is never used
if [ -n "$tarball" ]; then
  (
    flock 9; [ -e "$ok" ] && exit 0
    dl="$RUNNER_TEMP/binaryen-132.tar.gz"
    curl -fsSL --retry 3 -o "$dl" "$base/$tarball" || { echo "::warning::binaryen 132 could not be downloaded"; exit 1; }
    got=$(sha256_of "$dl")
    # a wrong hash is an error, never a fallback: the tarball is not the one that was reviewed
    [ "$got" = "$sha" ] || { echo "::error::binaryen 132 SHA-256 mismatch ($tarball: got $got, expected $sha)"; exit 2; }
    mkdir -p "$RUNNER_TEMP/binaryen-132"
    # shellcheck disable=SC2086
    tar xzf "$dl" --strip-components=1 -C "$RUNNER_TEMP/binaryen-132" $only && touch "$ok"
  ) 9>"$RUNNER_TEMP/wo.lock"
  rc=$?
  [ "$rc" = 2 ] && exit 1
fi
if [ ! -e "$ok" ]; then
  wo="$RUNNER_TEMP/wo/node_modules/.bin/wasm-opt"
  ok="$RUNNER_TEMP/wo/installato"   # written when npm has finished: the .bin link exists before the postinstall has downloaded the binary
  echo "::warning::binaryen 132 not available here: using npm wasm-opt@1.4.0"
  ( flock 9; [ -e "$ok" ] || { npm install --prefix "$RUNNER_TEMP/wo" wasm-opt@1.4.0 && touch "$ok"; } ) 9>"$RUNNER_TEMP/wo.lock"
fi
if [ -e "$ok" ]; then
  "$wo" -Oz --enable-bulk-memory --enable-sign-ext --enable-mutable-globals --enable-nontrapping-float-to-int --enable-reference-types --enable-multivalue --enable-simd "$@" -o "$bg" "$bg" || exit 1
  size=$(stat -c %s "$bg"); echo "$label .wasm (optimised): $size bytes"
  if [ "$size" -ge "$limit" ]; then echo "::error::the $label .wasm is $size bytes (limit $limit)"; exit 1; fi
else
  echo "::warning::wasm-opt could not be installed: the $label .wasm is not optimised ($(stat -c %s "$bg") bytes)"
fi
