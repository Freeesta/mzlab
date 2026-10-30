#!/usr/bin/env bash
# wasm-opt (level in $livello, -O3) on a .wasm of the site, then the size gate: tools/wasm_opt.sh <file> <limit in bytes> <label> [extra wasm-opt flags]
# Used by pages.yml and sito-rust.yml, so the site that is tested is the site that is published.
# wasm-opt is binaryen version_133 from the official GitHub tarball (SHA-256 pinned below), installed once per $RUNNER_TEMP
# (under a lock, because tools/wasm_paralleli.sh runs several of these at the same time). If the tarball cannot be used it falls back to npm wasm-opt@1.4.0.
# The ONE level for every module: -O3 runs the formula compositions 34-35 % faster in Firefox and WebKit than -Oz on the same rustc output, within the caps (docs/agenti/wasm-tetti.md).
# MZLAB_WASM_OPT_LEVEL overrides it for tools/banco_wasm.py only (the -Oz baseline of the bench); the workflows never set it.
# Never enable the relaxed vector (SIMD) proposal: its results depend on the CPU and site = app byte for byte (tests/test_guardie_regole.py).
set -u
livello=${MZLAB_WASM_OPT_LEVEL:--O3}
bg=$1; limit=$2; label=$3; shift 3
# `@estesi` (the readers, tools/build_lettori_wasm.sh) and `@motore` (the engine and libraries steps of pages.yml, sito-rust.yml, release.yml, notte.yml, tools/lab/qqq-pages.yml) stand
# for the extra passes below: the ONE place of the lists, so the site that is tested is the site that is published. The engine gets only the passes that pay on it, measured one by one on
# the CI pipeline: `--converge` -1 KB for CI time, `--const-hoisting` grows the gzip. Never `--traps-never-happen` or `--ignore-implicit-traps` (the crash test hook and the OOM supervisor need traps). `--low-memory-unused` assumes nothing reads or writes below address 1024:
# true for every Rust .wasm of the site, since rustc links wasm32-unknown-unknown with `--stack-first` (the data starts at the 1 MiB mark; tests/test_lettori_wasm.py checks it).
estesi="--converge --low-memory-unused --strip-producers --vacuum --dae --merge-blocks --reorder-functions --duplicate-function-elimination --const-hoisting --simplify-globals-optimizing"
motore="--low-memory-unused --reorder-functions --strip-producers"
args=()
for a in "$@"; do
  # shellcheck disable=SC2206
  if [ "$a" = "@estesi" ]; then args+=($estesi); elif [ "$a" = "@motore" ]; then args+=($motore); else args+=("$a"); fi
done
set -- ${args[@]+"${args[@]}"}
mkdir -p "$RUNNER_TEMP"
base="https://github.com/WebAssembly/binaryen/releases/download/version_133"
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)  tarball=binaryen-version_133-x86_64-linux.tar.gz;  sha=2dc9c7813f5375db93d96ead4b78222fcc3e2677bbb832297af4797782a37489; only=binaryen-version_133/bin/wasm-opt ;;
  Darwin-arm64)  tarball=binaryen-version_133-arm64-macos.tar.gz;   sha=ad66da82ac13f163e424b1643f16c6dfcccc98b5966296b43e52d3cab04f84a8; only= ;;
  Darwin-x86_64) tarball=binaryen-version_133-x86_64-macos.tar.gz;  sha=13a9b90be775c6389ce3d1f879cb8627bea56708ba8c122983941d53a8199b95; only= ;;
  *) tarball=; sha=; only= ;;
esac
sha256_of() { if command -v sha256sum >/dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1; }

wo="$RUNNER_TEMP/binaryen-133/bin/wasm-opt"
ok="$RUNNER_TEMP/binaryen-133/installato"   # written when the install has finished, so a half-extracted binary is never used
if [ -n "$tarball" ]; then
  (
    flock 9; [ -e "$ok" ] && exit 0
    dl="$RUNNER_TEMP/binaryen-133.tar.gz"
    curl -fsSL --retry 3 -o "$dl" "$base/$tarball" || { echo "::warning::binaryen 133 could not be downloaded"; exit 1; }
    got=$(sha256_of "$dl")
    # a wrong hash is an error, never a fallback: the tarball is not the one that was reviewed
    [ "$got" = "$sha" ] || { echo "::error::binaryen 133 SHA-256 mismatch ($tarball: got $got, expected $sha)"; exit 2; }
    mkdir -p "$RUNNER_TEMP/binaryen-133"
    # shellcheck disable=SC2086
    tar xzf "$dl" --strip-components=1 -C "$RUNNER_TEMP/binaryen-133" $only && touch "$ok"
  ) 9>"$RUNNER_TEMP/wo.lock"
  rc=$?
  [ "$rc" = 2 ] && exit 1
fi
if [ ! -e "$ok" ]; then
  wo="$RUNNER_TEMP/wo/node_modules/.bin/wasm-opt"
  ok="$RUNNER_TEMP/wo/installato"   # written when npm has finished: the .bin link exists before the postinstall has downloaded the binary
  echo "::warning::binaryen 133 not available here: using npm wasm-opt@1.4.0"
  ( flock 9; [ -e "$ok" ] || { npm install --prefix "$RUNNER_TEMP/wo" wasm-opt@1.4.0 && touch "$ok"; } ) 9>"$RUNNER_TEMP/wo.lock"
fi
if [ -e "$ok" ]; then
  "$wo" "$livello" --enable-bulk-memory --enable-sign-ext --enable-mutable-globals --enable-nontrapping-float-to-int --enable-reference-types --enable-multivalue --enable-simd "$@" -o "$bg" "$bg" || exit 1
  size=$(stat -c %s "$bg"); echo "$label .wasm (optimised): $size bytes"
  if [ "$size" -ge "$limit" ]; then echo "::error::the $label .wasm is $size bytes (limit $limit)"; exit 1; fi
else
  echo "::warning::wasm-opt could not be installed: the $label .wasm is not optimised ($(stat -c %s "$bg") bytes)"
fi
