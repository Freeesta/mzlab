"""Synthetic Orbitrap-like MS1 file whose contaminant ions carry a known mass error: a constant offset plus a drift along the run, for the calibration check
(tests_e2e/e2e_masscal.py). The ions are taken from the lab list (mzlab/web/contaminanti.json), so the file holds what the check looks for; nothing real in it."""
import json
from pathlib import Path

import numpy as np

from dati_sintetici import _U_MZ, _arr, _chrom, _cvp, _g, _hr_head

LISTA = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "contaminanti.json"


def ioni(n: int = 12, pol: int = 1) -> list[tuple[str, float]]:
    """`n` singly charged [M+H]+ / [M+Na]+ (or [M-H]- for pol -1) ions of the list between m/z 150 and 900, spread over the range and at least 2 Da apart: (name, listed m/z)."""
    adducts = ("[M+H]+", "[M+Na]+") if pol > 0 else ("[M-H]-",)
    voci = sorted({round(i["mz"], 4): i["name_en"] + " " + i["adduct"] for l in json.loads(LISTA.read_text(encoding="utf-8"))["lists"] for i in l["items"]
                   if i.get("z") == 1 and i.get("pol") == pol and i["adduct"] in adducts and 150 <= i["mz"] <= 900}.items())
    lontani = [voci[0]]
    for m, nome in voci[1:]:
        if m - lontani[-1][0] >= 2.0:
            lontani.append((m, nome))
    passo = max(1, len(lontani) // n)
    return [(nome, m) for m, nome in lontani[::passo][:n]]


def calibrazione(path: Path, rng, offset_ppm: float = 3.0, drift_ppm: float = 2.0, rt_end: float = 10.5, cyc: float = 0.1, pol: int = 1, n_ioni: int = 12) -> list[tuple[str, float]]:
    """Writes the file; the error of every ion is `offset_ppm` plus `drift_ppm` spread linearly over the run. Each ion stays all along, strongest at its own time (spread over the run),
    so the error against the RT of the apex shows the drift. Returns the ions (name, theoretical m/z as listed)."""
    lista = ioni(n_ioni, pol)
    pre, specs, rts, tic = "controllerType=0 controllerNumber=1 scan=", [], [], []
    filt = f"FTMS {'+' if pol > 0 else '-'} p ESI Full ms [100.0000-900.0000]"
    t0 = 0.5
    centri = np.linspace(t0 + 0.5, rt_end - 0.5, len(lista))
    for i, rt in enumerate(np.arange(t0, rt_end, cyc)):
        mz = list(rng.uniform(100, 900, 100)); it = list(rng.lognormal(np.log(3e3), 0.5, 100))
        err = offset_ppm + drift_ppm * (rt - t0) / (rt_end - t0)
        for (nome, m), c in zip(lista, centri):
            mz.append(m * (1 + (err + rng.normal(0, 0.3)) * 1e-6)); it.append(3e4 + 3e5 * float(_g(rt, c, 2.5)))
        o = np.argsort(mz); mz = np.array(mz)[o]; it = np.array(it)[o]
        n, b = len(mz), int(np.argmax(it))
        specs.append(f'<spectrum index="{i}" id="{pre}{i + 1}" defaultArrayLength="{n}">' + _cvp("MS:1000579", "MS1 spectrum") + _cvp("MS:1000511", "ms level", 1)
                     + _cvp("MS:1000130" if pol > 0 else "MS:1000129", "positive scan" if pol > 0 else "negative scan") + _cvp("MS:1000127", "centroid spectrum")
                     + _cvp("MS:1000504", "base peak m/z", f"{mz[b]:.6f}", _U_MZ) + _cvp("MS:1000505", "base peak intensity", f"{it[b]:.6g}")
                     + _cvp("MS:1000285", "total ion current", f"{float(np.sum(it)):.6g}")
                     + f'<scanList count="1">{_cvp("MS:1000795", "no combination")}<scan>'
                     + _cvp("MS:1000016", "scan start time", f"{rt:.6f}", ' unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"')
                     + _cvp("MS:1000800", "mass resolving power", 60000) + _cvp("MS:1000512", "filter string", filt)
                     + '<scanWindowList count="1"><scanWindow>' + _cvp("MS:1000501", "scan window lower limit", 100.0, _U_MZ)
                     + _cvp("MS:1000500", "scan window upper limit", 900.0, _U_MZ) + '</scanWindow></scanWindowList></scan></scanList>'
                     + '<binaryDataArrayList count="2">' + _arr("MS:1000514", "m/z array", mz) + _arr("MS:1000515", "intensity array", it) + "</binaryDataArrayList></spectrum>")
        rts.append(rt); tic.append(float(it.sum()))
    chroms = [_chrom(0, "TIC", np.array(rts), np.array(tic))]
    Path(path).write_text(_hr_head("exploris", False, Path(path).stem) + f'<spectrumList count="{len(specs)}">' + "".join(specs) + "</spectrumList>"
                          + f'<chromatogramList count="{len(chroms)}">' + "".join(chroms) + "</chromatogramList></run></mzML>", encoding="utf-8")
    return lista
