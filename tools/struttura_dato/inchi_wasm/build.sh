#!/bin/bash
# Measure only (docs/studi/STRUTTURA_DATO_PROVE.md): builds the IUPAC InChI 1.07.5 C library (MIT) to wasm32-wasi with zig cc and runs it on molfiles.
# Nothing here goes into the site. Needs: git, node, pip install ziglang. Usage: bash build.sh <work dir> [<molfile dir>]
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
work="$1"
mkdir -p "$work" && cd "$work"
[ -d src ] || git clone -q --depth 1 --branch v1.07.5 https://github.com/IUPAC-InChI/InChI.git src
S=src/INCHI-1-SRC
SRCS=$(ls $S/INCHI_BASE/src/*.c $S/INCHI_API/libinchi/src/*.c $S/INCHI_API/libinchi/src/ixa/*.c | grep -v 'inchi_dll_main')
# shellcheck disable=SC2086
python3 -m ziglang cc -target wasm32-wasi -Oz -DTARGET_API_LIB -DCOMPILE_ANSI_ONLY \
  -I$S/INCHI_BASE/src -I$S/INCHI_API/libinchi/src -I$S/INCHI_API/libinchi/src/ixa \
  "$here/shim.c" $SRCS -mexec-model=reactor -Wl,--gc-sections -Wl,--strip-all -o inchi.wasm -w
echo "inchi.wasm $(wc -c < inchi.wasm) B, gzip -9 $(gzip -9 -c inchi.wasm | wc -c) B"
if [ $# -ge 2 ]; then cp "$here/run.mjs" . && node --no-warnings run.mjs "$2"; fi
