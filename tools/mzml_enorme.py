"""A synthetic DDA mzML of any size (GB) or any number of scans, written as a stream, to measure the memory of the loading without laboratory data.

    python3 tools/mzml_enorme.py OUT.mzML --gb 4 [--ms2 10] [--peaks1 1500] [--peaks2 150] [--indexed] [--seed 7]
    python3 tools/mzml_enorme.py OUT.mzML --scans 1200000 --peaks1 40 --peaks2 25 --indexed

One MS1 survey scan is followed by `--ms2` MS2 scans (data dependent). The size comes from `--gb` (the number of scans is computed from the average
size of a cycle, so the header count is right) or from `--scans`. Nothing but a buffer is held in memory: the arrays come from a small pool of
variants encoded once, only the header (scan number, RT, precursor) changes. `--indexed` wraps the file as an indexed-mzML (offset list and SHA-1).
"""
import argparse
import base64
import hashlib
import struct
import sys
import zlib
from pathlib import Path

import numpy as np

CV = '<cvParam cvRef="MS" accession="%s" name="%s" value="%s"%s/>'
MZ_UNIT = ' unitCvRef="MS" unitAccession="MS:1000040" unitName="m/z"'
HEAD = """<?xml version="1.0" encoding="utf-8"?>
<mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0" id="mzlab">
<cvList count="2"><cv id="MS" fullName="Proteomics Standards Initiative Mass Spectrometry Ontology" version="4.1.163" URI="https://raw.githubusercontent.com/HUPO-PSI/psi-ms-CV/master/psi-ms.obo"/><cv id="UO" fullName="Unit Ontology" version="09:04:2014" URI="https://raw.githubusercontent.com/bio-ontology-research-group/unit-ontology/master/unit.obo"/></cvList>
<fileDescription><fileContent><cvParam cvRef="MS" accession="MS:1000579" name="MS1 spectrum" value=""/></fileContent></fileDescription>
<softwareList count="1"><software id="mzlab" version="1.0"><cvParam cvRef="MS" accession="MS:1000799" name="custom unreleased software tool" value="mzLab"/></software></softwareList>
<instrumentConfigurationList count="1"><instrumentConfiguration id="IC1"><componentList count="0"></componentList><softwareRef ref="mzlab"/></instrumentConfiguration></instrumentConfigurationList>
<dataProcessingList count="1"><dataProcessing id="conversion"><processingMethod order="0" softwareRef="mzlab"><cvParam cvRef="MS" accession="MS:1000544" name="Conversion to mzML" value=""/></processingMethod></dataProcessing></dataProcessingList>
<run id="enorme" defaultInstrumentConfigurationRef="IC1">
<spectrumList count="%d" defaultDataProcessingRef="conversion">
"""
TAIL = "</spectrumList>\n</run>\n</mzML>\n"


def arrays(rng, n, lo, hi, zl):
    """Base64 text of a sorted m/z array (64-bit) and an intensity array (32-bit), optionally zlib-compressed."""
    mz = np.sort(rng.uniform(lo, hi, n)).astype("<f8").tobytes()
    ity = (rng.lognormal(8, 1.5, n)).astype("<f4").tobytes()
    if zl:
        mz, ity = zlib.compress(mz), zlib.compress(ity)
    return base64.b64encode(mz).decode(), base64.b64encode(ity).decode()


def binary_list(n, mzb, ib, zl):
    z = CV % ("MS:1000574", "zlib compression", "", "") if zl else CV % ("MS:1000576", "no compression", "", "")
    a = lambda acc, nm, bits, b64, extra: (
        f'<binaryDataArray encodedLength="{len(b64)}">{CV % (acc[0], bits, "", "")}{z}{CV % (acc[1], nm, "", extra)}<binary>{b64}</binary></binaryDataArray>\n')
    unit_i = ' unitCvRef="MS" unitAccession="MS:1000131" unitName="number of detector counts"'
    return ('<binaryDataArrayList count="2">\n'
            + a(("MS:1000523", "MS:1000514"), "m/z array", "64-bit float", mzb, MZ_UNIT)
            + a(("MS:1000521", "MS:1000515"), "intensity array", "32-bit float", ib, unit_i)
            + "</binaryDataArrayList></spectrum>\n")


def pools(rng, peaks1, peaks2, variants):
    p1 = [(arrays(rng, peaks1, 100, 1500, i % 2 == 0)) for i in range(variants)]
    p2 = [(arrays(rng, peaks2, 50, 1200, i % 2 == 1)) for i in range(variants)]
    return ([(n, binary_list(peaks1, m, y, i % 2 == 0)) for i, (n, (m, y)) in enumerate((peaks1, v) for v in p1)],
            [(n, binary_list(peaks2, m, y, i % 2 == 1)) for i, (n, (m, y)) in enumerate((peaks2, v) for v in p2)])


def spectrum(idx, level, rt, prec, n, tail):
    """Header of one scan (indexes and ids are the only varying text) followed by the pooled arrays."""
    kind = ("MS:1000579", "MS1 spectrum", "MS:1000128", "profile spectrum") if level == 1 else ("MS:1000580", "MSn spectrum", "MS:1000127", "centroid spectrum")
    s = (f'<spectrum index="{idx}" id="controllerType=0 controllerNumber=1 scan={idx + 1}" defaultArrayLength="{n}">'
         + CV % ("MS:1000511", "ms level", level, "") + CV % (kind[0], kind[1], "", "") + CV % ("MS:1000130", "positive scan", "", "")
         + CV % (kind[2], kind[3], "", "") + CV % ("MS:1000285", "total ion current", "%.1f" % (1e6 + idx % 977), "")
         + f'<scanList count="1">{CV % ("MS:1000795", "no combination", "", "")}<scan>'
         + CV % ("MS:1000016", "scan start time", "%.5f" % rt, ' unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"')
         + "</scan></scanList>")
    if level == 2:
        s += ('<precursorList count="1"><precursor><isolationWindow>' + CV % ("MS:1000827", "isolation window target m/z", "%.4f" % prec, MZ_UNIT)
              + '</isolationWindow><selectedIonList count="1"><selectedIon>' + CV % ("MS:1000744", "selected ion m/z", "%.4f" % prec, MZ_UNIT)
              + "</selectedIon></selectedIonList></precursor></precursorList>")
    return s + tail


def cycle_bytes(p1, p2, ms2):
    """Average bytes of one survey scan with its MS2 scans, from the pooled arrays."""
    a1 = np.mean([len(spectrum(0, 1, 1.0, 0.0, 1, t).encode()) for _, t in p1])
    a2 = np.mean([len(spectrum(0, 2, 1.0, 500.0, 1, t).encode()) for _, t in p2])
    return a1 + ms2 * a2


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--gb", type=float)
    ap.add_argument("--scans", type=int)
    ap.add_argument("--ms2", type=int, default=10)
    ap.add_argument("--peaks1", type=int, default=1500)
    ap.add_argument("--peaks2", type=int, default=150)
    ap.add_argument("--variants", type=int, default=16)
    ap.add_argument("--indexed", action="store_true")
    ap.add_argument("--seed", type=int, default=7)
    a = ap.parse_args()
    if (a.gb is None) == (a.scans is None):
        sys.exit("give exactly one of --gb and --scans")
    rng = np.random.default_rng(a.seed)
    p1, p2 = pools(rng, a.peaks1, a.peaks2, a.variants)
    per = a.ms2 + 1
    scans = a.scans if a.scans else int(a.gb * 1e9 / cycle_bytes(p1, p2, a.ms2) * per)
    prefix = (b'<?xml version="1.0" encoding="utf-8"?>\n<indexedmzML xmlns="http://psi.hupo.org/ms/mzml">\n' if a.indexed else b"")
    sha, offsets, pos = hashlib.sha1(), [], 0

    def put(f, text):
        nonlocal pos
        b = text if isinstance(text, bytes) else text.encode()
        f.write(b)
        sha.update(b)
        pos += len(b)

    with open(a.out, "wb", buffering=8 << 20) as f:
        put(f, prefix)
        put(f, (HEAD % scans).replace('<?xml version="1.0" encoding="utf-8"?>\n', "", 1) if a.indexed else HEAD % scans)
        prec = 400.0
        for i in range(scans):
            level = 1 if i % per == 0 else 2
            n, tail = (p1 if level == 1 else p2)[(i // per + i) % a.variants]
            if level == 1:
                prec = 300.0 + (i * 7.31) % 700
            offsets.append(pos)
            put(f, spectrum(i, level, i * 0.0004, prec + (i % per) * 1.003, n, tail))
        put(f, TAIL)
        if a.indexed:
            at = pos
            idx = "".join(f'<offset idRef="controllerType=0 controllerNumber=1 scan={i + 1}">{o}</offset>\n' for i, o in enumerate(offsets))
            put(f, f'<indexList count="1">\n<index name="spectrum">\n{idx}</index>\n</indexList>\n<indexListOffset>{at}</indexListOffset>\n<fileChecksum>')
            put(f, sha.hexdigest() + "</fileChecksum>\n</indexedmzML>\n")
    size = Path(a.out).stat().st_size
    print(f"{a.out}: {scans} scans, {size / 1e9:.3f} GB, {size / scans:.0f} B per scan" + (", indexed" if a.indexed else ""))


if __name__ == "__main__":
    main()
