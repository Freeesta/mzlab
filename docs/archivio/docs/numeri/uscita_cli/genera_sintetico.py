#!/usr/bin/env python3
"""Genera un file mzML sintetico zlib-encoded con picchi cromatografici gaussiani.
Nessun dato privato o reale: solo standard analitici noti a m/z e RT teorici.
"""

import base64
from pathlib import Path
import zlib
import numpy as np

PEAKS = [
    {"name": "paracetamolo", "formula": "C8H9NO2", "mz": 152.07060, "rt": 1.80, "height": 3.0e5, "fwhm": 0.12},
    {"name": "caffeina", "formula": "C8H10N4O2", "mz": 195.08765, "rt": 2.50, "height": 5.0e5, "fwhm": 0.15},
    {"name": "ibuprofene", "formula": "C13H18O2", "mz": 207.13795, "rt": 3.20, "height": 4.0e5, "fwhm": 0.14},
    {"name": "carbamazepina", "formula": "C15H12N2O", "mz": 237.10224, "rt": 4.00, "height": 6.0e5, "fwhm": 0.16},
    {"name": "atrazina", "formula": "C8H14ClN5", "mz": 216.10105, "rt": 4.60, "height": 3.5e5, "fwhm": 0.13},
]


def write_synthetic_mzml(file_path: Path, peaks_info: list[dict]) -> np.ndarray:
    def b64(a, f4=False):
        dt = "<f4" if f4 else "<f8"
        return base64.b64encode(zlib.compress(np.asarray(a, dt).tobytes())).decode()

    def arr(acc, name, data, f4=False):
        cv = "MS:1000521" if f4 else "MS:1000523"
        bits = "32-bit float" if f4 else "64-bit float"
        return (
            f'<binaryDataArray encodedLength="0">'
            f'<cvParam cvRef="MS" accession="{cv}" name="{bits}" value=""/>'
            f'<cvParam cvRef="MS" accession="MS:1000574" name="zlib compression" value=""/>'
            f'<cvParam cvRef="MS" accession="{acc}" name="{name}" value=""/>'
            f"<binary>{b64(data, f4)}</binary></binaryDataArray>"
        )

    rts = np.linspace(1.0, 5.5, 150)
    specs = []
    for i, rt in enumerate(rts):
        mzs, its = [], []
        for p in peaks_info:
            sigma = p["fwhm"] / 2.35482
            val = p["height"] * np.exp(-0.5 * ((rt - p["rt"]) / sigma) ** 2)
            if val > 100.0:
                mzs.append(p["mz"])
                its.append(val)
        if not mzs:
            mzs, its = [100.0], [10.0]
        a1 = arr("MS:1000514", "m/z array", mzs, False)
        a2 = arr("MS:1000515", "intensity array", its, True)
        specs.append(
            f'<spectrum index="{i}" id="scan={i+1}" defaultArrayLength="{len(mzs)}">'
            f'<cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="1"/>'
            f'<cvParam cvRef="MS" accession="MS:1000130" name="positive scan" value=""/>'
            f'<cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum" value=""/>'
            f'<scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" '
            f'name="scan start time" value="{rt:.5f}" unitCvRef="UO" '
            f'unitAccession="UO:0000031" unitName="minute"/></scan></scanList>'
            f'<binaryDataArrayList count="2">{a1}{a2}</binaryDataArrayList></spectrum>'
        )
    xml = (
        '<?xml version="1.0" encoding="utf-8"?>'
        '<mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0">'
        f'<run id="{file_path.stem}"><spectrumList count="{len(specs)}">'
        + "".join(specs) + "</spectrumList></run></mzML>"
    )
    file_path.write_text(xml, encoding="utf-8")
    return rts


if __name__ == "__main__":
    out_file = Path(__file__).resolve().parent / "sintetico.mzML"
    write_synthetic_mzml(out_file, PEAKS)
    print(f"Scritto {out_file} ({out_file.stat().st_size} bytes)")
