"""Reference peaks in the manner of mzRAPP, generated from the truth: M, M+1, M+2 of every true ion, each extracted as an XIC from the file where M is
strongest. A reference peak is the most abundant fine isotopologue of its nominal group (13C for M+1, 34S for M+2 when the ion has sulphur), so a
+-5 ppm window around it contains only that component."""
from __future__ import annotations

from dataclasses import dataclass, field

import numpy as np

from mzlab.chem import elements as E
from mzlab.chem import isoscore

from . import PPM

K = (0, 1, 2)


@dataclass
class Picco:
    """One true ion at one observed RT, with its three isotopologue windows."""
    ion_id: str
    rt_obs: float
    mz: list                  # theoretical m/z of M, M+1, M+2
    rel: list                 # theoretical relative abundance of the same components
    area: np.ndarray = field(default=None)       # (3, files): trace area, 0 where absent
    height: np.ndarray = field(default=None)
    apex: np.ndarray = field(default=None)       # (3, files): apex RT
    ref_file: int = -1        # file of the strongest M, -1 when M is nowhere


def componenti(formula: str) -> tuple[list, list]:
    """(m/z, rel) of M, M+1, M+2 of an ion formula (singly charged): the largest fine component of each nominal group."""
    pat = isoscore.fine_pattern(E.parse_formula(formula), z=1)
    m0 = max(pat, key=lambda p: p["rel"])
    mz, rel = [], []
    for k in K:
        grp = [p for p in pat if abs(p["mz"] - m0["mz"] - k * 1.00235) < 0.5 and (k > 0 or p["mz"] <= m0["mz"] + 0.5)]
        top = max(grp, key=lambda p: (p["rel"], -p["mz"])) if grp else None
        mz.append(top["mz"] if top else float("nan"))
        rel.append(top["rel"] if top else 0.0)
    return mz, rel


def picchi_da_verita(ioni: list[dict]) -> list[Picco]:
    """One Picco per (true ion, observed RT); ions without an observed RT or formula cannot be extracted and are left out."""
    out = []
    for ion in ioni:
        if not ion.get("ion"):
            continue
        mz, rel = componenti(ion["ion"])
        for rt in ion.get("rt_obs") or []:
            out.append(Picco(ion_id=str(ion["id"]), rt_obs=float(rt), mz=mz, rel=rel))
    return out


def misura_file(table, picchi: list[Picco], k_file: int, n_files: int, floor: float, ppm: float = PPM, rt_win: float = 0.3, apex_win: float = 0.05) -> None:
    """Extract the three windows of every Picco from one file's MS1 table (called while the table is loaded)."""
    from tpmine.hr import features as FT
    for p in picchi:
        if p.area is None:
            p.area, p.height, p.apex = (np.zeros((3, n_files)), np.zeros((3, n_files)), np.full((3, n_files), np.nan))
        a0, apex0 = FT.trace_area(table, p.mz[0], p.rt_obs - rt_win, p.rt_obs + rt_win, floor, ppm)
        for k in K:
            if k == 0:
                a, apex = a0, apex0
            elif a0 > 0:
                a, apex = FT.trace_area(table, p.mz[k], apex0 - apex_win, apex0 + apex_win, floor, ppm)
            else:
                continue
            if a <= 0:
                continue
            y = table.xic(p.mz[k], p.mz[k] * ppm * 1e-6)
            i = int(np.argmin(np.abs(table.rt - apex)))
            p.area[k, k_file], p.apex[k, k_file] = a, apex
            p.height[k, k_file] = float(y[max(i - 1, 0):i + 2].max())


def scegli_riferimento(picchi: list[Picco]) -> None:
    for p in picchi:
        p.ref_file = int(np.argmax(p.area[0])) if p.area is not None and p.area[0].max() > 0 else -1
