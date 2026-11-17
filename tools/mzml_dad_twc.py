"""Synthetic mzML with optical (MS:1000804) spectra and the standard ProteoWizard 1D DAD trace `<chromatogram id="TWC">` (MS:1000813).

Every number is invented. The Rust test `proteowizard_twc_chromatogram_fills_the_trace` (mzml_dad_tests.rs) builds the same shape inline.

    python3 tools/mzml_dad_twc.py OUT.mzML
"""
from __future__ import annotations

import base64
import struct
import sys

NT, WL = 4, (200.0, 201.0, 202.0)


def _b64(v):
    return base64.b64encode(struct.pack(f"<{len(v)}d", *v)).decode()


def _arr(acc, name, v):
    return (f'<binaryDataArray encodedLength="{len(_b64(v))}"><cvParam cvRef="MS" accession="MS:1000523" name="64-bit float"/>'
            f'<cvParam cvRef="MS" accession="{acc}" name="{name}"/><binary>{_b64(v)}</binary></binaryDataArray>')


def build(nt: int = NT) -> str:
    rows = "".join(
        f'<spectrum index="{i}" id="uv={i}" defaultArrayLength="{len(WL)}"><cvParam cvRef="MS" accession="MS:1000804" name="electromagnetic radiation spectrum"/>'
        f'<cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{i * 0.6:g}" unitName="second"/>'
        f'<binaryDataArrayList count="2">{_arr("MS:1000617", "wavelength array", WL)}'
        f'{_arr("MS:1000515", "intensity array", [float(i * len(WL) + j) for j in range(len(WL))])}</binaryDataArrayList></spectrum>'
        for i in range(nt))
    chrom = (f'<chromatogramList count="1"><chromatogram index="0" id="TWC" defaultArrayLength="{nt}">'
             '<cvParam cvRef="MS" accession="MS:1000813" name="absorption chromatogram"/>'
             f'<binaryDataArrayList count="2">{_arr("MS:1000595", "time array", [i * 0.01 for i in range(nt)])}'
             f'{_arr("MS:1000515", "intensity array", [10.0 + i for i in range(nt)])}</binaryDataArrayList></chromatogram></chromatogramList>')
    return f'<?xml version="1.0" encoding="utf-8"?><mzML><run id="run"><spectrumList count="{nt}">{rows}</spectrumList>{chrom}</run></mzML>'


if __name__ == "__main__":
    open(sys.argv[1], "w", encoding="utf-8").write(build())
