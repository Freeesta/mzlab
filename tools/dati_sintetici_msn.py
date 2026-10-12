"""Synthetic MSn file (a direct infusion with a fragmentation cascade) for the tests of the MSn tree: all numbers invented."""
from pathlib import Path

import numpy as np

from dati_sintetici import _U_MZ, _arr, _cvp, _hr_head


def msn_infusion(path: Path, rng, scans: int = 14) -> None:
    """Direct infusion of one invented ion with a fragmentation cascade (Orbitrap, CID 35): `scans` MS1, MS2 of 305.1000, MS3 of 305.1000 > 261.0800
    and 305.1000 > 244.0600 (paths in the filter strings, nominal masses as Thermo writes them). All numbers invented; the fragments carry a
    mass defect and a few noise peaks, and one fragment is missing from some scans (the consensus keeps the peaks of at least half of them)."""
    stages = [("ms2", [(305.1000, "cid", 35.0)], [(261.0800, 1.0), (244.0600, 0.5), (217.1200, 0.3)]),
              ("ms3", [(305.1000, "cid", 35.0), (261.0800, "cid", 35.0)], [(244.0605, 1.0), (190.0500, 0.4), (145.0400, 0.2)]),
              ("ms3", [(305.1000, "cid", 35.0), (244.0600, "cid", 35.0)], [(190.0498, 1.0), (162.0550, 0.6)])]
    specs, idx = [], 0

    def one(level, filt, peaks, rt, prec=None):
        nonlocal idx
        mz = np.array([m for m, _ in peaks] + list(rng.uniform(60, 320, 6))) * rng.normal(1, 0.2e-6, len(peaks) + 6)
        it = np.array([a * 1e6 * rng.normal(1, 0.05) for _, a in peaks] + list(rng.uniform(1e3, 8e3, 6)))
        o = np.argsort(mz); mz, it = mz[o], it[o]
        pl = (f'<precursorList count="1"><precursor><isolationWindow>{_cvp("MS:1000827", "isolation window target m/z", prec, _U_MZ)}</isolationWindow>'
              f'<selectedIonList count="1"><selectedIon>{_cvp("MS:1000744", "selected ion m/z", prec, _U_MZ)}</selectedIon></selectedIonList>'
              f'<activation>{_cvp("MS:1000133", "collision-induced dissociation")}{_cvp("MS:1000045", "collision energy", 35.0)}</activation></precursor></precursorList>') if prec else ""
        specs.append(f'<spectrum index="{idx}" id="controllerType=0 controllerNumber=1 scan={idx + 1}" defaultArrayLength="{len(mz)}">'
                     + _cvp("MS:1000579" if level == 1 else "MS:1000580", "MS1 spectrum" if level == 1 else "MSn spectrum") + _cvp("MS:1000511", "ms level", level)
                     + _cvp("MS:1000130", "positive scan") + _cvp("MS:1000127", "centroid spectrum") + _cvp("MS:1000285", "total ion current", f"{float(it.sum()):.6g}")
                     + f'<scanList count="1">{_cvp("MS:1000795", "no combination")}<scan>' + _cvp("MS:1000512", "filter string", filt)
                     + _cvp("MS:1000016", "scan start time", f"{rt:.6f}", ' unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"')
                     + f'</scan></scanList>{pl}<binaryDataArrayList count="2">' + _arr("MS:1000514", "m/z array", mz) + _arr("MS:1000515", "intensity array", it)
                     + "</binaryDataArrayList></spectrum>")
        idx += 1

    for k in range(scans):
        rt = 0.1 + 0.02 * k * 4
        one(1, "FTMS + p ESI Full ms [50.0000-400.0000]", [(305.1000, 1.0), (306.1034, 0.14)], rt)
        for j, (name, path_, peaks) in enumerate(stages):
            keep = peaks if k % 5 else peaks[:-1]
            filt = f"FTMS + p ESI d Full {name} " + " ".join(f"{m:.4f}@{a}{ce:.2f}" for m, a, ce in path_) + " [50.0000-400.0000]"
            one(int(name[-1]), filt, keep, rt + 0.005 * (j + 1), prec=path_[-1][0])
    Path(path).write_text(_hr_head("exploris", False, Path(path).stem) + f'<spectrumList count="{len(specs)}">' + "".join(specs)
                          + "</spectrumList></run></mzML>", encoding="utf-8")

