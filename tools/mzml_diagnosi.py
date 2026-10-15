"""Looks at a big mzML the way the loader does, without the browser, and prints where it breaks: size, header, wrapper, the offsets of the index, the end of the file, the count of the
spectra, a full read in blocks (timing), the well-formedness of the first and the last spectrum and the cvParam accessions. No data is printed, only structure.
    python3 tools/mzml_diagnosi.py FILE.mzML"""
import re
import sys
import time
import xml.etree.ElementTree as ET
from pathlib import Path

p = Path(sys.argv[1])
n = p.stat().st_size
print("size", n, "bytes =", round(n / 1e6), "MB")
with open(p, "rb") as f:
    head = f.read(6000)
    print("starts with xml declaration:", head.lstrip()[:5] == b"<?xml", "| BOM:", head[:3] == b"\xef\xbb\xbf", "| indexedmzML wrapper:", b"<indexedmzML" in head)
    print("header (to the list of the spectra):")
    for line in head.decode("utf8", "replace").splitlines()[:40]:
        print("  ", re.sub(r"(/Users|/home|[A-Z]:\\)[^\s\"']*", "<path>", line)[:200])
        if "<spectrumList" in line:
            break
    f.seek(max(0, n - 4000))
    tail = f.read()
    print("tail has indexListOffset:", b"<indexListOffset>" in tail, "| fileChecksum:", b"<fileChecksum>" in tail, "| closes </indexedmzML>:", tail.rstrip().endswith(b"</indexedmzML>"))
    m = re.search(rb"<indexListOffset>(\d+)</indexListOffset>", tail)
    if m:
        off = int(m.group(1)); f.seek(off); at = f.read(40)
        print("indexListOffset", off, "points to <indexList:", at.startswith(b"<indexList"), "(", at[:20], ")")
    t0 = time.time(); cnt = 0; acc = set(); first = last = None; prev = b""
    f.seek(0)
    while True:
        b = f.read(1 << 24)
        if not b:
            break
        d = prev + b
        cnt += len(re.findall(rb"<spectrum ", d)); acc.update(re.findall(rb'accession="(MS:\d+|UO:\d+)"', d))
        prev = d[-20:]
    print("spectra", cnt, "| full read", round(time.time() - t0, 1), "s | distinct accessions", len(acc))
    for lab, pos in (("first", None), ("last", True)):
        f.seek(0 if pos is None else max(0, n - 40_000_000)); blob = f.read(40_000_000 if pos else n)
        i = blob.find(b"<spectrum ") if pos is None else blob.rfind(b"<spectrum ")
        j = blob.find(b"</spectrum>", i)
        try:
            ET.fromstring(blob[i:j + 11]); print(lab, "spectrum well-formed: True, length", j - i)
        except Exception as e:
            print(lab, "spectrum well-formed: False ->", str(e)[:120])
