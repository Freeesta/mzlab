"""The criteria of the low-resolution (unit resolution, 3200 QTRAP) feature tables, on plain data: no engine runs here.

MRM (`B_MRM-STD_*`, six levels, and `B_MRM-t*`): the transitions of the standards are detected in every level above the lowest, their area grows with the
concentration (Spearman 1 over the detected levels), the log-log R2 is only reported, and the cells of the treated files are coherent (state and area agree).
Full scan (`B_FullMass-t*`, `B_MS2-*`): every reference ion (the former Python engine's candidates and unexpected ions, frozen for the synthetic series) is a row of the table (+-0.35 Da, +-0.25 min), every
candidate has its proofs, isomers that the profile separates (valley below 50 %) are two rows.

Tolerances are the ones of the project and are never fitted on the products or the standards: a failing criterion is reported with its number. Every
criterion is `cancello.crit` (`{id, valore, soglia, ok}`), `ok` null when it could not be measured."""
from __future__ import annotations

import re
from dataclasses import dataclass, field

import numpy as np

from .cancello import crit

TOL_MZ = 0.35                  # Da, the LR matching window (LR_TOL of mzlab-feat)
TOL_RT = 0.25                  # min
VALLE_MAX = 0.50               # two isomers are two rows when the valley between them is below this fraction of the lower apex
CELL_DETECTED, CELL_TARGETED, CELL_FILLED, CELL_ABSENT = 0, 1, 2, 3
ID_MRM = ("lr_mrm_rilevate_livelli", "lr_mrm_area_cresce", "lr_mrm_stati_coerenti")
ID_FULLSCAN = ("lr_ioni_python_in_tabella", "lr_candidati_con_prove", "lr_isomeri_due_righe")
ID_LR = ID_MRM + ID_FULLSCAN + ("lr_determinismo",)


@dataclass
class Mrm:
    """A native MRM table: one row per transition and RT, one column per file."""
    nomi: list[str]
    q1: np.ndarray
    q3: np.ndarray
    rt: np.ndarray
    stato: np.ndarray                      # (rows, files) uint8, CELL_*
    area: np.ndarray                       # (rows, files) float
    tipo: list[str] = field(default_factory=list)
    livello: list[float | None] = field(default_factory=list)       # concentration (or ordinal) of the standards, None otherwise
    conc_vera: bool = False                # the levels are concentrations (the log-log R2 makes sense), not ordinals


def classifica(nome: str) -> str:
    """'std' (calibration standard), 'bianco', 'campione' (a treated file `..._t30`) or 'altro', from the file name."""
    low = nome.lower()
    if re.search(r"(^|[_\-\s])(blank|blk|bianco)([_\-\s\d]|$)", low):
        return "bianco"
    if re.search(r"(^|[_\-\s])(std|standard|taratura|cal)([_\-\s\d]|$)", low):
        return "std"
    return "campione" if re.search(r"[_\-\s]t\d", low) else "altro"


def livello_da_nome(nome: str) -> tuple[float | None, bool]:
    """(level, is a concentration) of a standard: '12ppb' -> 0.012 mg/L; 'STD_3' -> (3, False) an ordinal; (None, False) when the name says nothing."""
    low = nome.lower().replace(" ", "")
    m = re.search(r"(\d+(?:[._,]\d+)?)(ppm|ppb|ppt|mg/?l|ug/?l|ng/?l)", low)
    if m:
        k = {"ppm": 1.0, "mgl": 1.0, "ppb": 1e-3, "ugl": 1e-3, "ppt": 1e-6, "ngl": 1e-6}[m.group(2).replace("/", "")]
        return float(m.group(1).replace("_", ".").replace(",", ".")) * k, True
    m = re.search(r"(?:std|standard|cal|taratura)[_\-]?(\d+(?:\.\d+)?)", low)
    return (float(m.group(1)), False) if m else (None, False)


def ranghi(v) -> np.ndarray:
    """Ranks with ties averaged."""
    v = np.asarray(v, float)
    o = np.argsort(v, kind="stable")
    r = np.empty(len(v))
    r[o] = np.arange(len(v), dtype=float)
    for x in np.unique(v):
        k = v == x
        if k.sum() > 1:
            r[k] = r[k].mean()
    return r


def spearman(x, y) -> float | None:
    """Rank correlation; None when either series is constant."""
    rx, ry = ranghi(x), ranghi(y)
    if np.ptp(rx) == 0 or np.ptp(ry) == 0:
        return None
    return float(np.corrcoef(rx, ry)[0, 1])


def r2_loglog(conc, area) -> float | None:
    """R2 of log10(area) against log10(concentration); reported, never a criterion (the students make the line)."""
    c, a = np.asarray(conc, float), np.asarray(area, float)
    k = (c > 0) & (a > 0)
    if k.sum() < 3 or np.ptp(c[k]) == 0:
        return None
    x, y = np.log10(c[k]), np.log10(a[k])
    p = np.polyfit(x, y, 1)
    res = float(np.sum((y - np.polyval(p, x)) ** 2))
    tot = float(np.sum((y - y.mean()) ** 2))
    return 1.0 - res / tot if tot > 0 else None


def _livelli_standard(m: Mrm) -> list[int]:
    """Columns of the standards from the lowest to the highest level (file order when the names carry no level)."""
    cols = [k for k, t in enumerate(m.tipo) if t == "std"]
    lv = [m.livello[k] if k < len(m.livello) else None for k in cols]
    if cols and all(x is not None for x in lv):
        return [c for _, c in sorted(zip(lv, cols))]
    return cols


def mrm_standard(m: Mrm) -> dict:
    """The measures of the standards: {n_transizioni, rilevate_livelli, crescono, n_valutate, r2: [...]} (counts over the transitions of the standards)."""
    cols = _livelli_standard(m)
    if len(cols) < 3:
        return {"livelli": len(cols), "n_transizioni": 0}
    rows = [r for r in range(len(m.rt)) if any(m.stato[r, k] == CELL_DETECTED for k in cols)]
    sopra = cols[1:]
    rilevate = [r for r in rows if all(m.stato[r, k] == CELL_DETECTED for k in sopra)]
    crescono, r2 = 0, []
    for r in rows:
        det = [k for k in cols if m.stato[r, k] == CELL_DETECTED]
        s = spearman([cols.index(k) for k in det], [m.area[r, k] for k in det]) if len(det) >= 2 else None
        crescono += s is not None and s > 1.0 - 1e-9
        if m.conc_vera and len(det) >= 3:
            q = r2_loglog([m.livello[k] for k in det], [m.area[r, k] for k in det])
            if q is not None:
                r2.append(q)
    return {"livelli": len(cols), "n_transizioni": len(rows), "rilevate_livelli": len(rilevate), "crescono": crescono, "r2": r2}


def celle_incoerenti(m: Mrm, cols: list[int]) -> tuple[int, int]:
    """(incoherent, total) cells of the given files: a detected or filled cell has a finite area above zero, an absent one none; a targeted cell does not
    exist in an MRM table; a row has at least one detected cell in the whole series."""
    bad = tot = 0
    for r in range(len(m.rt)):
        for k in cols:
            s, a = int(m.stato[r, k]), m.area[r, k]
            ok = (s in (CELL_DETECTED, CELL_FILLED) and np.isfinite(a) and a > 0) or (s == CELL_ABSENT and (not np.isfinite(a) or a == 0))
            bad += not ok
            tot += 1
        bad += not any(m.stato[r, k] == CELL_DETECTED for k in range(m.stato.shape[1]))
    return bad, tot


def criteri_mrm(m: Mrm | None) -> tuple[list[dict], dict]:
    """The three MRM criteria and their details; `m` None: not measured."""
    if m is None:
        return [crit("lr_mrm_rilevate_livelli", None, 1.0, ">="), crit("lr_mrm_area_cresce", None, 0.90, ">="), crit("lr_mrm_stati_coerenti", None, 1.0, ">=")], {}
    s = mrm_standard(m)
    n = s["n_transizioni"]
    campioni = [k for k, t in enumerate(m.tipo) if t == "campione"]
    bad, tot = celle_incoerenti(m, campioni) if campioni else (0, 0)
    det = {"standard": {k: v for k, v in s.items() if k != "r2"}, "campioni": {"file": len(campioni), "celle": tot, "incoerenti": bad}}
    if s.get("r2"):
        det["r2_loglog"] = {"n": len(s["r2"]), "mediano": round(float(np.median(s["r2"])), 4), "minimo": round(float(np.min(s["r2"])), 4)}
    return [crit("lr_mrm_rilevate_livelli", s["rilevate_livelli"] / n if n else None, 1.0, ">="),
            crit("lr_mrm_area_cresce", s["crescono"] / n if n else None, 0.90, ">="),
            crit("lr_mrm_stati_coerenti", 1.0 - bad / tot if tot else None, 1.0, ">=")], det


# ---------------------------------------------------------------------------------------------------------------- full scan
def righe_vicine(mz: float, rt: float, righe_mz, righe_rt, tol_mz: float = TOL_MZ, tol_rt: float = TOL_RT) -> np.ndarray:
    """Indexes of the table rows within the window of an ion."""
    return np.flatnonzero((np.abs(np.asarray(righe_mz) - mz) <= tol_mz) & (np.abs(np.asarray(righe_rt) - rt) <= tol_rt))


def copertura(rif: list[dict], righe_mz, righe_rt) -> tuple[int, int, list[dict]]:
    """(found, total, missing ions) of the ions {mz, rt} of the reference in the rows of the table."""
    miss = [x for x in rif if len(righe_vicine(x["mz"], x["rt"], righe_mz, righe_rt)) == 0]
    return len(rif) - len(miss), len(rif), miss


def prove_candidato(i: dict) -> bool:
    """A candidate has its proofs when it has at least one contribution in favour and every contribution says what it is (`tipo`, `segno`, `fonte`)."""
    c = i.get("contributi") or []
    return bool(c) and all(x.get("tipo") and x.get("segno") and x.get("fonte") for x in c) and any(x["segno"] == "a favore" for x in c)


def candidati_lr(identita: list[dict]) -> list[dict]:
    """The LR candidates: the identities that a formula hypothesis explains (the others are unexpected ions)."""
    return [i for i in identita if i.get("formula")]


def isomeri_separati(coppie: list[tuple], righe_mz, righe_rt) -> tuple[int, int]:
    """(pairs with two distinct rows, pairs): `coppie` are ((mz, rt), (mz, rt)) of isomers that the profile separates; two distinct rows match the two points."""
    ok = 0
    for a, b in coppie:
        ra, rb = righe_vicine(*a, righe_mz, righe_rt), righe_vicine(*b, righe_mz, righe_rt)
        ok += any(x != y for x in ra for y in rb)
    return ok, len(coppie)


def valle(ca: float, sa: float, aa: float, cb: float, sb: float, ab: float) -> float:
    """Valley between two Gaussian peaks (centres, sigmas, amplitudes) as a fraction of the lower apex: min of the sum between the centres / lower apex."""
    t = np.linspace(min(ca, cb), max(ca, cb), 400)
    y = aa * np.exp(-0.5 * ((t - ca) / sa) ** 2) + ab * np.exp(-0.5 * ((t - cb) / sb) ** 2)
    return float(y.min() / min(aa, ab))


def coppie_isomeri(rif: list[dict], tol_rt: float = TOL_RT) -> list[tuple]:
    """Pairs of reference ions with the same nominal mass whose RT differ by more than the matching window: the engine resolved two peaks."""
    out = []
    for i, a in enumerate(rif):
        for b in rif[i + 1:]:
            if round(a["mz"]) == round(b["mz"]) and abs(a["rt"] - b["rt"]) > tol_rt:
                out.append(((a["mz"], a["rt"]), (b["mz"], b["rt"])))
    return out


def criteri_fullscan(rif: list[dict] | None, righe_mz, righe_rt, identita: list[dict], coppie: list[tuple]) -> tuple[list[dict], dict]:
    """The three full scan criteria and their details; `rif` None (no reference run): not measured."""
    if rif is None:
        return [crit("lr_ioni_python_in_tabella", None, 0.95, ">="), crit("lr_candidati_con_prove", None, 1.0, ">="), crit("lr_isomeri_due_righe", None, 1.0, ">=")], {}
    trovati, n, miss = copertura(rif, righe_mz, righe_rt)
    cand = candidati_lr(identita)
    con_prove = sum(prove_candidato(i) for i in cand)
    iso, np_ = isomeri_separati(coppie, righe_mz, righe_rt)
    det = {"ioni_riferimento": n, "ioni_trovati": trovati, "ioni_mancanti": [{"mz": round(x["mz"], 2), "rt": round(x["rt"], 2)} for x in miss],
           "candidati": len(cand), "candidati_con_prove": con_prove, "coppie_isomeri": np_, "coppie_separate": iso, "righe": len(righe_mz)}
    return [crit("lr_ioni_python_in_tabella", trovati / n if n else None, 0.95, ">="),
            crit("lr_candidati_con_prove", con_prove / len(cand) if cand else None, 1.0, ">="),
            crit("lr_isomeri_due_righe", iso / np_ if np_ else None, 1.0, ">=")], det
