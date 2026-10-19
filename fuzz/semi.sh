#!/bin/sh
# Builds fuzz/corpus/<target>/ from the synthetic files of the repository (no laboratory data): the golden mzML/mzidx and the `.wiff` pairs of tools/wiff_riferimento.py.
# Used by .github/workflows/fuzz.yml and by hand before `cargo +nightly fuzz run <target>` from fuzz/.
set -eu
cd "$(dirname "$0")"
C=corpus
G=../tests/golden/ref/msntree_rotta
W=$(mktemp -d)
trap 'rm -rf "$W"' EXIT
for t in ole xmltree_methodinfo mzmlhead idxread raw wiff mzlib_msp_mgf; do mkdir -p "$C/$t"; done
python3 ../tools/wiff_riferimento.py "$W" >/dev/null
for f in one two serie prodotto variante; do
  [ -f "$W/$f.wiff" ] || continue
  cp "$W/$f.wiff" "$C/ole/$f.wiff"
  cp "$W/$f.wiff" "$C/xmltree_methodinfo/$f.wiff"
  if [ -f "$W/$f.wiff.scan" ]; then
    # u32 LE length of the .wiff, the .wiff, the .wiff.scan
    python3 - "$W/$f.wiff" "$W/$f.wiff.scan" "$C/wiff/$f.bin" <<'P'
import struct, sys
w, s = open(sys.argv[1], "rb").read(), open(sys.argv[2], "rb").read()
open(sys.argv[3], "wb").write(struct.pack("<I", len(w)) + w + s)
P
  fi
done
cp "$G/msn.mzML" "$C/mzmlhead/msn.mzML"
cp "$G/msn.mzML" "$C/xmltree_methodinfo/msn.mzML"
cp "$G/msn.mzidx" "$C/idxread/msn.mzidx"
# the header of a Thermo `.raw` (Finnigan, UTF-16): the fuzzer grows the rest
printf '\001\241F\000i\000n\000n\000i\000g\000a\000n\000' >"$C/raw/header.raw"
# format byte (0 MSP, 1 MGF) and a record each
printf '\000Name: A\nPrecursorMZ: 100.5\nNum Peaks: 2\n50 10\n60 20\n\n' >"$C/mzlib_msp_mgf/a.msp"
printf '\001BEGIN IONS\nPEPMASS=100.5\n50 10\n60 20\nEND IONS\n' >"$C/mzlib_msp_mgf/a.mgf"
