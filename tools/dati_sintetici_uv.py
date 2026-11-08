"""Synthetic LC-MS file with a DAD in the layout the `sciex` readers module writes for a variant `.wiff` (`mzmlwrite/uv.rs`): the MS scans, then one
`MS:1000804` spectrum per instant of the detector (wavelength array in nm, absorbance in mAU), then a TIC and the `TWC` absorption chromatogram.
Every number is invented (fixed seed). Used by `tools/bench_uv.py` (timings at a realistic size) and `tests_e2e/e2e_uv_overlay.py` (small).

    python3 tools/dati_sintetici_uv.py OUT.mzML [--uv 2400] [--wl 300] [--scans 1500] [--peaks 500] [--seed 1] [--minutes 20]
"""
from __future__ import annotations

import argparse
import base64
from pathlib import Path

import numpy as np


def _cvp(acc, name, value="", unit=""):
    v = f' value="{value}"' if value != "" else ' value=""'
    return f'<cvParam cvRef="MS" accession="{acc}" name="{name}"{v}{unit}/>'


_MIN = ' unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"'


def _arr(acc, name, a, bits, extra=""):
    """One uncompressed array; the time array in minutes."""
    b = base64.b64encode(np.asarray(a, "<f8" if bits == 64 else "<f4").tobytes()).decode()
    return (f'<binaryDataArray encodedLength="{len(b)}">{_cvp("MS:1000523" if bits == 64 else "MS:1000521", f"{bits}-bit float")}'
            f'{_cvp("MS:1000576", "no compression")}{_cvp(acc, name, "", _MIN if acc == "MS:1000595" else "")}{extra}<binary>{b}</binary></binaryDataArray>')


def compounds(seed):
    """(RT min, sigma min, height mAU, band centre nm, band sigma nm) of five invented compounds; the seed moves the heights (another sample)."""
    rng = np.random.default_rng(seed)
    base = [(3.1, 0.08, 400, 254, 14), (6.4, 0.07, 900, 280, 18), (8.5, 0.09, 650, 230, 12), (12.2, 0.08, 300, 320, 20), (15.7, 0.10, 800, 260, 16)]
    return [(t, s, a * rng.uniform(0.5, 1.5), c, w) for t, s, a, c, w in base]


def uv_matrix(times, wl, seed):
    """`(len(times), len(wl))` absorbance in mAU: a baseline, the bands of `compounds`, a little noise."""
    t, w = np.asarray(times, float), np.asarray(wl, float)
    m = np.full((t.size, w.size), 2.0)
    for rt, s, a, c, sw in compounds(seed):
        m += a * np.outer(np.exp(-0.5 * ((t - rt) / s) ** 2), np.exp(-0.5 * ((w - c) / sw) ** 2))
    m += np.random.default_rng(seed + 7).normal(0, 0.3, m.shape)
    return m.astype("<f4")


def write(path, n_uv=2400, n_wl=300, n_scans=1500, n_peaks=500, seed=1, minutes=20.0):
    """Writes the file spectrum by spectrum; returns (times of the detector, wavelengths)."""
    rng = np.random.default_rng(seed)
    times = np.linspace(0.0, minutes, n_uv)
    wl = 190.0 + np.arange(n_wl, dtype=float)
    mat = uv_matrix(times, wl, seed)
    ms_t = np.linspace(0.02, minutes, n_scans)
    tic = []
    with open(path, "w", encoding="utf-8") as f:
        f.write(f'<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0"><run id="{Path(path).stem}">')
        f.write(f'<spectrumList count="{n_scans + n_uv}">')
        for i, rt in enumerate(ms_t):
            mz = np.sort(rng.uniform(80, 600, n_peaks)); it = rng.lognormal(np.log(3e3), 0.8, n_peaks); tic.append(float(it.sum()))
            b = int(np.argmax(it))
            f.write(f'<spectrum index="{i}" id="sample=1 period=1 cycle={i + 1} experiment=1" defaultArrayLength="{n_peaks}">'
                    + _cvp("MS:1000579", "MS1 spectrum") + _cvp("MS:1000511", "ms level", 1) + _cvp("MS:1000130", "positive scan") + _cvp("MS:1000127", "centroid spectrum")
                    + _cvp("MS:1000504", "base peak m/z", f"{mz[b]:.4f}") + _cvp("MS:1000505", "base peak intensity", f"{it[b]:.6g}")
                    + _cvp("MS:1000285", "total ion current", f"{tic[-1]:.6g}")
                    + f'<scanList count="1">{_cvp("MS:1000795", "no combination")}<scan>{_cvp("MS:1000016", "scan start time", f"{rt:.6f}", _MIN)}</scan></scanList>'
                    + '<binaryDataArrayList count="2">' + _arr("MS:1000514", "m/z array", mz, 64) + _arr("MS:1000515", "intensity array", it, 32)
                    + "</binaryDataArrayList></spectrum>\n")
        for i, t in enumerate(times):
            f.write(f'<spectrum index="{n_scans + i}" id="uv={i}" defaultArrayLength="{n_wl}">' + _cvp("MS:1000804", "electromagnetic radiation spectrum")
                    + _cvp("MS:1000016", "scan start time", f"{t}", _MIN) + '<userParam name="intensity unit" value="mAU"/><binaryDataArrayList count="2">\n'
                    + _arr("MS:1000617", "wavelength array", wl, 32) + _arr("MS:1000515", "intensity array", mat[i], 32) + "</binaryDataArrayList></spectrum>\n")
        f.write('</spectrumList><chromatogramList count="2">')
        f.write(f'<chromatogram index="0" id="TIC" defaultArrayLength="{n_scans}">{_cvp("MS:1000235", "total ion current chromatogram")}<binaryDataArrayList count="2">'
                + _arr("MS:1000595", "time array", ms_t, 64) + _arr("MS:1000515", "intensity array", tic, 32) + "</binaryDataArrayList></chromatogram>")
        f.write(f'<chromatogram index="1" id="TWC" defaultArrayLength="{n_uv}">{_cvp("MS:1000812", "absorption chromatogram")}<binaryDataArrayList count="2">'
                + _arr("MS:1000595", "time array", times, 64) + _arr("MS:1000515", "intensity array", mat.sum(axis=1), 32) + "</binaryDataArrayList></chromatogram>")
        f.write("</chromatogramList></run></mzML>")
    return times, wl


if __name__ == "__main__":
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("out"); ap.add_argument("--uv", type=int, default=2400); ap.add_argument("--wl", type=int, default=300)
    ap.add_argument("--scans", type=int, default=1500); ap.add_argument("--peaks", type=int, default=500); ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--minutes", type=float, default=20.0)
    a = ap.parse_args()
    write(a.out, a.uv, a.wl, a.scans, a.peaks, a.seed, a.minutes)
