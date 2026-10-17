"""Synthetic experiment of the mzFinder e2e tests (no real data): `make_demo(folder)` writes the series and returns its files.

Truth built into the data (unit-resolution centroids shifted by +0.30 Da, like the real instrument): parent [M+H]+ 241.0641 decays;
hydroxylation (+O, 257.059), N-deisopropylation (-C3H6, 199.017) and dehydrogenation (-H2, 239.048) rise over time; an ion near 273 is a contaminant
present everywhere (blank included); an ion with the +O mass elutes at another RT in every file. MS2 of the +O ion: 175, 215 (= parent fragment 199 + 16), 239; MS2 of the parent: 133, 199."""
from __future__ import annotations

import base64
import zlib
from pathlib import Path

import numpy as np

PARENT_MZ = 241.0641
SHIFT = 0.30
TIMES = [0, 5, 10, 15, 30, 45, 60]


def _gauss(rt, c, s):
    return np.exp(-0.5 * ((rt - c) / s) ** 2)


def _b64(a: np.ndarray) -> bytes:
    z = zlib.compress(np.asarray(a, "<f8").tobytes())
    return base64.b64encode(z)


def _spectrum(i: int, level: int, rt: float, mz: np.ndarray, it: np.ndarray, prec: float | None = None, ce: float | None = None) -> str:
    prec_xml = ""
    if prec is not None:
        prec_xml = (f'<precursorList count="1"><precursor><selectedIonList count="1"><selectedIon>'
                    f'<cvParam cvRef="MS" accession="MS:1000744" name="selected ion m/z" value="{prec}"/></selectedIon>'
                    f'</selectedIonList><activation><cvParam cvRef="MS" accession="MS:1000045" name="collision energy" '
                    f'value="{ce}"/></activation></precursor></precursorList>')
    flt = f"+ p ESI Q1MS [100.000-400.000]" if level == 1 else f"+ p ESI Product ion ({prec}) CE{ce}"
    m, a = _b64(mz), _b64(it)
    return (f'<spectrum index="{i}" id="sample=1 period=1 cycle={i + 1} experiment={level}" defaultArrayLength="{len(mz)}">'
            f'<cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="{level}"/>'
            f'<cvParam cvRef="MS" accession="MS:1000130" name="positive scan" value=""/>'
            f'<cvParam cvRef="MS" accession="MS:1000285" name="total ion current" value="{float(it.sum()):.6g}"/>'
            f'<scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{rt:.5f}" '
            f'unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/>'
            f'<cvParam cvRef="MS" accession="MS:1000512" name="filter string" value="{flt}"/></scan></scanList>'
            f'{prec_xml}<binaryDataArrayList count="2">'
            f'<binaryDataArray encodedLength="{len(m)}"><cvParam cvRef="MS" accession="MS:1000523" name="64-bit float" value=""/>'
            f'<cvParam cvRef="MS" accession="MS:1000574" name="zlib compression" value=""/>'
            f'<cvParam cvRef="MS" accession="MS:1000514" name="m/z array" value=""/><binary>{m.decode()}</binary></binaryDataArray>'
            f'<binaryDataArray encodedLength="{len(a)}"><cvParam cvRef="MS" accession="MS:1000523" name="64-bit float" value=""/>'
            f'<cvParam cvRef="MS" accession="MS:1000574" name="zlib compression" value=""/>'
            f'<cvParam cvRef="MS" accession="MS:1000515" name="intensity array" value=""/><binary>{a.decode()}</binary></binaryDataArray>'
            f'</binaryDataArrayList></spectrum>')



def _profile(t, blank):
    """(true m/z, RT centre, sigma, amplitude) of the compounds present, for a treatment time t."""
    out = [(273.20, 6.4, 0.07, 3.0e5), (301.15, 2.0, 0.08, 3.0e5), (157.09, 3.0, 0.08, 1.5e5)]       # in every file (blank included)
    if blank or t is None:
        return out
    out.append((PARENT_MZ, 9.0, 0.07, 3.0e6 * np.exp(-0.05 * t)))
    out.append((257.0590, 7.4, 0.07, 1.2e6 * (1 - np.exp(-0.06 * t)) * np.exp(-0.01 * t)))
    out.append((199.0172, 6.2, 0.07, 6.0e5 * (1 - np.exp(-0.04 * t))))
    out.append((239.0485, 10.1, 0.07, 3.0e5 * (1 - np.exp(-0.03 * t)) ** 2))
    out.append((257.0590, 4.1, 0.07, 4.0e5))        # same mass, other RT, present at every time (interference)
    return out


def write_mzml(path: Path, t, blank: bool, rng, ms2: bool = False) -> None:
    comps = _profile(t, blank)
    specs, i = [], 0
    for n, rt in enumerate(np.arange(0.5, 14.0, 0.05)):
        mz = list(rng.uniform(100, 400, 40))                                   # chemical noise
        it = list(rng.lognormal(np.log(3e3), 0.8, 40))
        for m, c, s, a in comps:
            v = a * _gauss(rt, c, s) * rng.normal(1, 0.04)
            if v > 100:
                mz.append(m + SHIFT + rng.normal(0, 0.03))
                it.append(v)
        o = np.argsort(mz)
        if not ms2:
            specs.append(_spectrum(i, 1, float(rt), np.array(mz)[o], np.array(it)[o]))
            i += 1
        elif not blank and t is not None:                                       # product-ion scans of the +O ion
            sig = 1.2e6 * (1 - np.exp(-0.06 * t)) * np.exp(-0.01 * t) * _gauss(rt, 7.4, 0.07)
            f_mz = np.array([175.0, 215.0, 239.0]) + SHIFT
            f_it = np.array([0.5, 1.0, 0.2]) * sig * 0.05 + rng.uniform(50, 200, 3)
            specs.append(_spectrum(i, 2, float(rt) + 0.01, f_mz, f_it, prec=257.1, ce=20.0))
            i += 1
            psig = 3.0e6 * np.exp(-0.05 * t) * _gauss(rt, 9.0, 0.07)               # product-ion scans of the parent too
            p_it = np.array([0.4, 1.0]) * psig * 0.05 + rng.uniform(50, 200, 2)
            specs.append(_spectrum(i, 2, float(rt) + 0.02, np.array([133.0, 199.0]) + SHIFT, p_it, prec=241.1, ce=20.0))
            i += 1
    head = ('<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0">'
            f'<run id="{path.stem}"><spectrumList count="{len(specs)}">')
    path.write_text(head + "".join(specs) + "</spectrumList></run></mzML>", encoding="utf-8")


def make_demo(folder) -> list[dict]:
    """Writes the files and returns the file list for Experiment(...): name, path, time, type."""
    folder = Path(folder)
    folder.mkdir(parents=True, exist_ok=True)
    rng = np.random.default_rng(11)
    files = []
    for t in TIMES:
        p = folder / f"synth_FullMass-t{t}.mzML"
        write_mzml(p, t, False, rng)
        files.append({"name": p.name, "path": str(p), "time": float(t), "type": "sample"})
    p = folder / "synth_FullMass-blank.mzML"
    write_mzml(p, None, True, rng)
    files.append({"name": p.name, "path": str(p), "time": None, "type": "blank"})
    p = folder / "synth_MS2-t15.mzML"
    write_mzml(p, 15, False, rng, ms2=True)
    files.append({"name": p.name, "path": str(p), "time": 15.0, "type": "sample"})
    return files
