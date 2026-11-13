#!/usr/bin/env python3
"""Cut an RT window out of a (centroid, zlib) mzML and anonymise it.

Keeps the scans inside [rt0, rt1] plus the MS1 parent of every kept MS2, drops
the chromatograms and the index, and removes sample names, paths, checksums and
spectrum titles. Usage:
    python3 tools/crop_mzml.py IN.mzML OUT.mzML --rt0 5.0 --rt1 6.4 [--id HRMS_t000]
"""
import argparse
import re
import sys
from pathlib import Path

SPECTRUM = re.compile(r"[ \t]*<spectrum .*?</spectrum>[ \t]*\n?", re.S)
RT = re.compile(r'name="scan start time" value="([-\d.eE+]+)"[^>]*?unitName="(minute|second)"')
PARENT = re.compile(r'<precursor spectrumRef="([^"]*)"')
NATIVE = re.compile(r'<spectrum index="\d+" id="([^"]*)"')


def scan_rt_min(sp):
    m = RT.search(sp)
    if not m:
        return None
    v = float(m.group(1))
    return v / 60 if m.group(2) == "second" else v


def crop(text, rt0, rt1, run_id="run"):
    spectra = [m.group(0) for m in SPECTRUM.finditer(text)]
    if not spectra:
        raise ValueError("no spectra found")
    head = text[:text.index("<spectrumList")]
    head = head[head.index("<mzML"):]
    head = re.sub(r"<sourceFileList.*?</sourceFileList>\s*", "", head, flags=re.S)
    head = re.sub(r'(<mzML\b[^>]*?) id="[^"]*"', rf'\1 id="{run_id}"', head, count=1)
    head = re.sub(r'(<run\b[^>]*?) id="[^"]*"', rf'\1 id="{run_id}"', head, count=1)
    head = re.sub(r' startTimeStamp="[^"]*"| defaultSourceFileRef="[^"]*"', "", head)
    tail_attr = re.search(r'<spectrumList count="\d+"([^>]*)>', text).group(1)

    keep = []
    ids = {}
    for sp in spectra:
        rt = scan_rt_min(sp)
        if rt is not None and rt0 <= rt <= rt1:
            keep.append(sp)
            ids[NATIVE.search(sp).group(1)] = True
    # an MS2 inside the window keeps its (possibly earlier) MS1 parent
    by_id = {NATIVE.search(sp).group(1): sp for sp in spectra}
    out, seen = [], set()
    for sp in keep:
        p = PARENT.search(sp)
        if p and p.group(1) in by_id and p.group(1) not in seen and p.group(1) not in ids:
            out.append(by_id[p.group(1)])
            seen.add(p.group(1))
        sid = NATIVE.search(sp).group(1)
        if sid not in seen:
            out.append(sp)
            seen.add(sid)
    out = [sp for sp in out if sp is not None]
    out.sort(key=lambda sp: scan_rt_min(sp) or 0)

    body = []
    for i, sp in enumerate(out):
        sp = re.sub(r'<spectrum index="\d+"', f'<spectrum index="{i}"', sp, count=1)
        sp = re.sub(r'[ \t]*<cvParam [^>]*name="spectrum title"[^>]*/>\s*\n', "", sp)
        body.append(sp if sp.endswith("\n") else sp + "\n")
    return (
        '<?xml version="1.0" encoding="utf-8"?>\n    ' + head
        + f'<spectrumList count="{len(body)}"{tail_attr}>\n' + "".join(body)
        + "      </spectrumList>\n    </run>\n</mzML>\n"
    )


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("src")
    ap.add_argument("dst")
    ap.add_argument("--rt0", type=float, required=True, help="start of the window (min)")
    ap.add_argument("--rt1", type=float, required=True, help="end of the window (min)")
    ap.add_argument("--id", default=None, help="run id written in the output (default: output name)")
    a = ap.parse_args(argv)
    text = Path(a.src).read_text(encoding="utf-8")
    res = crop(text, a.rt0, a.rt1, a.id or Path(a.dst).stem)
    Path(a.dst).write_text(res, encoding="utf-8")
    print(f"{a.dst}: {len(res) / 1e6:.1f} MB", file=sys.stderr)


if __name__ == "__main__":
    main()
