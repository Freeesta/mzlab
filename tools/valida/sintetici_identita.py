#!/usr/bin/env python3
"""Synthetic HR series with an identity truth: ion families, in-source fragments, isomers, chimeric MS2, kinetics, decoys (every molecule and number is invented).

    python3 tools/valida/sintetici_identita.py --seme N --profilo piccolo|completo --out DIR

Writes in DIR one Orbitrap-like DDA mzML per sample (SIN_blank, SIN_dark, SIN_t0, SIN_tNNmin; tools/valida/sintetici_mzml.py) and the truth, `verita.csv` and
`verita.json`. The same seed gives the same bytes. The JSON is the format of the real truth files (`parent`, `tps`: read as they are by tools/valida_feat.py)
plus `ioni` (every row of the truth), `file` and `meta`.

Truth columns: id, ruolo, mz, rt_obs[], ion, famiglia, progenitore, generazione, forma, purezza, isf_in_ms2, presente, k_form, k_dec, note.
- ruolo: progenitore, tp1, tp2, isotopo, addotto, isf, isf_finto, isomero, chimerica, esca, coincidenza, fondo. `tps` of the JSON = tp1, tp2, isomero.
- famiglia: id of the molecule the ion belongs to (isotopes, adducts and in-source fragments share the family of their ion).
- progenitore: product -> the molecule it forms from (parent for generation 1, the first product for generation 2); isotopo / addotto / isf / isf_finto /
  chimerica -> the ion they belong to or contaminate; esca / coincidenza -> the ion the hypothesis is built on.
- generazione: 0 parent, 1 product of the parent, 2 product of a product.
- forma: kinetics. progenitore (first-order decay, saturated in the first times: flat apex, constant area, then collapse), monotona, bateman (rises and falls),
  ritardo (2nd generation: delayed), piatto (constant after the first minutes), decrescente (present from t0, decays); come_progenitore for the ions that follow
  their base ion with a declared constant ratio; continuo / lobo_FT for the background.
- purezza: true purity of the isolated precursor (share of the target in the isolation window, constant); on target and partner.
- isf_in_ms2: 1 if the m/z of the in-source fragment is a fragment in the MS2 of its parent, 0 if it is not (decoy); empty otherwise.
- presente: 0 for the decoys (esca) that are not in the data. k_form, k_dec: rate constants (1/min). note: `chiave=valore;...` (ratio, delta_rt, valle, regola...).
- esca: mass hypothesis from an invented rule (delta m of fantasy, none is a rule of the program) applied to the parent or a product; absent from the data.
- coincidenza: a matrix noise peak (random m/z, present in every file) that falls within +-5 ppm of a hypothesis; `meta.coincidenze.attese` is the number
  expected by chance for the whole set of hypotheses (density of the noise peaks x total width of the windows), `osservate` the number found.
Intensities are peak heights of the most abundant component of the ion in the file; derived ions follow the intensity of their base ion with a constant ratio.
"""
from __future__ import annotations

import argparse
import csv
import json
import sys
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from valida import ROOT                                  # noqa: E402,F401  (puts `mzlab` on sys.path)
from valida.isotopologhi import componenti               # noqa: E402
from valida.sintetici_mzml import ISO_SEMI, scrivi_file  # noqa: E402,F401
from mzlab.chem import elements as E                     # noqa: E402
from mzlab.chem import isoscore                          # noqa: E402

NA_DELTA, NH4_DELTA = 21.98194, 17.02655          # [M+Na]+ and [M+NH4]+ minus [M+H]+ (checked by the tests against the element table)
FORME_TP = ["monotona", "bateman", "piatto", "decrescente"]
REGOLE_ESCA = [13.5731, 41.2087, -27.1914, 63.0412, 85.3329, -9.4476, 19.8765, 52.6108, -43.7219, 28.4057, -15.3342, 71.9163]
MZ_RUMORE = (150.0, 800.0)

PROFILI = {
    "piccolo": dict(file=[("blank", "blank", 0.0), ("dark", "dark", 0.0), ("t0", "t0", 0.0), ("t05min", "trat", 5.0), ("t10min", "trat", 10.0), ("t20min", "trat", 20.0)],
                    n_tp=4, n_tp2=1, isomeri=[(0.05, 0.3), (0.1, 0.7), (0.2, 0.3), (0.3, 0.7)], purezze=[0.95, 0.7, 0.5, 0.3], per_livello=1, n_fondo=4, passo=0.7,
                    famiglie_extra=0, t_sat=10.0, n_rumore=800, n_regole=8, n_esche=12),
    "completo": dict(file=[("blank", "blank", 0.0), ("dark", "dark", 0.0), ("t0", "t0", 0.0)] + [(f"t{int(t):02d}min", "trat", float(t)) for t in (1, 2, 3, 5, 7, 10, 15, 20, 30, 45, 60, 90)],
                     n_tp=12, n_tp2=4, isomeri=[(d, v) for d in (0.05, 0.1, 0.2, 0.3) for v in (0.3, 0.7)] * 4, purezze=[0.95, 0.7, 0.5, 0.3], per_livello=20, n_fondo=12,
                     passo=0.1, famiglie_extra=30, t_sat=20.0, n_rumore=8000, n_regole=12, n_esche=400),
}
ESTESO = dict(catene={"3": 6, "2": 12}, f_isf_tp=0.5, n_isf_variabile=6, n_travestiti=8, escala_k=1.0, t_limite=72.0, gap_tmax=1.5, n_alogenati=8, gruppi_per_dt=3)
PROFILI["V1X"] = {**PROFILI["completo"], "esteso": ESTESO}
PROFILI["V1X-piccolo"] = {**PROFILI["piccolo"], "esteso": {**ESTESO, "catene": {"3": 2, "2": 3}, "n_isf_variabile": 2, "n_travestiti": 4, "escala_k": 3.0, "t_limite": 16.0, "gap_tmax": 0.5,
                                                            "n_alogenati": 4, "gruppi_per_dt": 1}}
LOSSES = ["C4H8", "H2O", "C2H4", "CH2O", "C3H6", "NH3", "C2H2O", "CO", "C2H6", "CH4O", "C3H4", "C4H6", "H2S", "C2H3N"]
TRASF = [{k: v for k, v in (("C", dc), ("H", dh), ("O", do)) if v} for dc in range(-4, 5) for do in range(0, 4) for dh in range(-6, 7, 2) if (dc, dh, do) != (0, 0, 0)]


@dataclass
class Ione:
    id: str
    ruolo: str
    famiglia: str
    rt: float = 0.0
    sg: float = 0.05
    progenitore: str = ""
    generazione: int | None = None
    forma: str = ""
    ion: str = ""
    picchi: list = field(default_factory=list)       # [(m/z, relative abundance %)] fine components of the ion
    mz: float = 0.0
    amp: np.ndarray | None = None                     # height of the most abundant component, per file
    clip: np.ndarray | None = None                    # saturation: the most abundant component never exceeds this height, per file
    purezza: float | None = None
    isf_in_ms2: int | None = None
    presente: int = 1
    k_form: float | None = None
    k_dec: float | None = None
    note: str = ""
    selezionabile: bool = False
    frammenti: tuple | None = None                    # (m/z[], relative intensity[]) of its MS2
    isomero_di: str = ""


# ---------------------------------------------------------------------------------------------------------------- chemistry
def somma(a: dict, b: dict, s: int = 1) -> dict:
    out = dict(a)
    for e, n in b.items():
        out[e] = out.get(e, 0) + s * n
    return {e: n for e, n in out.items() if n}


def valida_formula(f: dict) -> bool:
    return all(n > 0 for n in f.values()) and f.get("C", 0) >= 3 and f.get("H", 0) >= 4


def formula_inventata(rng) -> dict:
    C, N, O, S = int(rng.integers(13, 23)), int(rng.integers(2, 5)), int(rng.integers(2, 6)), int(rng.integers(0, 2))
    Cl = 1 if rng.random() < 0.3 else 0
    f = {"C": C, "H": 2 * C + 2 + N - 2 * int(rng.integers(6, 10)) - Cl, "N": N, "O": O}
    if S:
        f["S"] = 1
    if Cl:
        f["Cl"] = 1
    return f


def modello(ion_formula: str):
    """Fine components (m/z, rel %) and the reference peaks (m/z of M, M+1, M+2 as the validation extracts them) of an ion formula."""
    pat = isoscore.fine_pattern(E.parse_formula(ion_formula), z=1)
    mz, rel = componenti(ion_formula)
    return [(p["mz"], p["rel"]) for p in pat if p["rel"] >= 0.05], mz, rel


def valle_relativa(d: float, sg: float, h2: float) -> float:
    """Valley depth between two Gaussians (heights 1 and h2, distance d, same sigma) as a fraction of the lower of the two apexes of the sum; 1 = no valley."""
    x = np.linspace(-4 * sg, d + 4 * sg, 4001)
    y = np.exp(-0.5 * (x / sg) ** 2) + h2 * np.exp(-0.5 * ((x - d) / sg) ** 2)
    m = np.flatnonzero((y[1:-1] > y[:-2]) & (y[1:-1] >= y[2:])) + 1
    if len(m) < 2:
        return 1.0
    a, b = m[0], m[-1]
    return float(y[a:b + 1].min() / min(y[a], y[b]))


def sigma_per_valle(d: float, target: float, h2: float = 0.8) -> float:
    lo, hi = 0.003, 1.0
    for _ in range(60):
        mid = 0.5 * (lo + hi)
        if valle_relativa(d, mid, h2) < target:
            lo = mid
        else:
            hi = mid
    return 0.5 * (lo + hi)


def curva(forma: str, t, p: dict) -> np.ndarray:
    """Relative amount (0..1 for the formed products) at time t. Rates in p: kd (decay), kf (formation), kp / ka / kb (parent -> first product -> second product)."""
    t = np.asarray(t, float)
    if forma in ("progenitore", "decrescente"):
        return np.exp(-p["kd"] * t)
    if forma in ("monotona", "piatto"):
        return 1 - np.exp(-p["kf"] * t)
    if forma == "bateman":
        f = lambda x: p["kf"] / (p["kd"] - p["kf"]) * (np.exp(-p["kf"] * x) - np.exp(-p["kd"] * x))          # noqa: E731
    elif forma == "ritardo":
        kp, ka, kb = p["kp"], p["ka"], p["kb"]
        f = lambda x: kp * ka * (np.exp(-kp * x) / ((ka - kp) * (kb - kp)) + np.exp(-ka * x) / ((kp - ka) * (kb - ka)) + np.exp(-kb * x) / ((kp - kb) * (ka - kb)))      # noqa: E731
    else:
        raise ValueError(forma)
    return f(t) / f(np.linspace(0, 400, 4001)).max()


def frammenti_di(rng, mz: float, extra=(), evita=()) -> tuple:
    """Invented MS2 of a precursor: 5 fragments (fractions of the precursor with a mass defect) plus the `extra` m/z (with relative intensity), none near an `evita` m/z."""
    f, r = [], []
    while len(f) < 5:
        x = round(mz * float(rng.uniform(0.25, 0.93)), 4)
        if all(abs(x - y) > 0.05 for y in evita) and all(abs(x - y) > 0.05 for y, _ in extra):
            f.append(x)
            r.append(float(rng.uniform(0.15, 1.0)))
    for x, ri in extra:
        f.append(x)
        r.append(ri)
    o = np.argsort(f)
    return np.array(f)[o], np.array(r)[o]


class Derivati:
    """Ions derived from a base ion (isotopes, adducts, in-source fragments); needs `rng`, `ioni` and `ione` of the class that uses it."""
    def derivati(self, base: Ione, neutra: dict, isotopi=True, addotti=(), isf=0, isf_finti=0):
        """Isotopes (rows of the truth only: the pattern is already in the data), adducts, in-source fragments."""
        rng = self.rng
        if isotopi:
            _, mz, rel = modello(base.ion)
            for k in (1, 2):
                self.ioni.append(Ione(id=f"{base.id}_M{k}", ruolo="isotopo", famiglia=base.famiglia, rt=base.rt, sg=base.sg, progenitore=base.id, ion=base.ion, mz=float(mz[k]),
                                      forma="come_progenitore", note=f"rel={rel[k]:.4f}", amp=None))
        for nome, delta, ad in addotti:
            r = round(float(rng.uniform(0.03, 0.12)), 3)
            self.ione(f"{base.id}_{nome}", "addotto", base.famiglia, neutra, base.rt + float(rng.uniform(-0.008, 0.008)), base.sg, base.amp * r, adduct=ad,
                      progenitore=base.id, forma="come_progenitore", note=f"ratio={r};delta_mz={delta}")
        nuovi = []
        for j in range(isf + isf_finti):
            vera = j < isf
            while True:
                p = E.parse_formula(LOSSES[int(rng.integers(0, len(LOSSES)))])
                f = somma(E.parse_formula(base.ion), p, -1)
                if valida_formula(f) and all(abs(modello(E.fmt(f))[1][0] - o.mz) > 0.6 for o in self.ioni if o.famiglia == base.famiglia and o.ruolo.startswith("isf")):
                    break
            r = round(float(rng.uniform(0.06, 0.25)), 3)
            o = Ione(id=f"{base.id}_{'ISF' if vera else 'ISFF'}{j + 1 if vera else j - isf + 1}", ruolo="isf" if vera else "isf_finto", famiglia=base.famiglia,
                     rt=base.rt + float(rng.uniform(-0.008, 0.008)), sg=base.sg, progenitore=base.id, forma="come_progenitore", isf_in_ms2=1 if vera else 0,
                     amp=base.amp * r, note=f"ratio={r}")
            o.ion = E.fmt(f)
            o.picchi, mz, _ = modello(o.ion)
            o.mz = float(mz[0])
            self.ioni.append(o)
            nuovi.append(o)
        veri = [o for o in nuovi if o.ruolo == "isf"]
        finti = [o for o in nuovi if o.ruolo == "isf_finto"]
        base.frammenti = frammenti_di(rng, base.mz, extra=[(o.mz, float(rng.uniform(0.5, 0.9))) for o in veri], evita=[o.mz for o in finti])
        return nuovi


class Costruttore(Derivati):
    def __init__(self, seme: int, profilo: str):
        self.rng = np.random.default_rng(seme)
        self.seme = seme
        self.vx = None                                    # state of the V1X extension (valida/sintetici_esteso.py)
        self.P = PROFILI[profilo]
        self.file = self.P["file"]
        self.tempi = np.array([t for _, _, t in self.file])
        self.tipi = [k for _, k, _ in self.file]
        self.ioni: list[Ione] = []
        self.rt_cursor = 1.0
        self.n_mol = 0
        self.formule: list[dict] = []

    def slot(self, passo: float | None = None) -> float:
        r = self.rt_cursor
        self.rt_cursor += passo or self.P["passo"]
        return r

    def id_mol(self) -> str:
        self.n_mol += 1
        return f"SIN-{self.n_mol:03d}"

    def traccia(self, forma, scala=1.0, **p) -> np.ndarray:
        """Amount per file: 0 in the blank, the curve at the time of the file elsewhere."""
        return scala * np.array([0.0 if k == "blank" else float(curva(forma, t, p)) for t, k in zip(self.tempi, self.tipi)])

    def ione(self, id, ruolo, famiglia, neutra: dict | None, rt, sg, amp, adduct="[M+H]+", **kw) -> Ione:
        """An ion of a molecule (neutral formula)."""
        o = Ione(id=id, ruolo=ruolo, famiglia=famiglia, rt=float(rt), sg=float(sg), amp=amp, **kw)
        if neutra is not None:
            o.ion = E.fmt(somma(neutra, {"[M+H]+": {"H": 1}, "[M+Na]+": {"Na": 1}, "[M+NH4]+": {"N": 1, "H": 4}}[adduct]))
            o.picchi, mz, rel = modello(o.ion)
            o.mz = float(mz[0])
        self.ioni.append(o)
        return o

    def libero(self, f: dict, rt: float) -> bool:
        """A new formula whose [M+H]+ is at least 1.3 m/z from every other base ion eluting within 0.6 min (no unplanned co-isolation)."""
        if not valida_formula(f) or any(f == g for g in self.formule):
            return False
        mz = E.ion_mz(E.mass(f), "[M+H]+")
        return all(abs(o.mz - mz) > 1.3 or abs(o.rt - rt) > 0.6 for o in self.ioni if o.amp is not None and o.ruolo in ("progenitore", "tp1", "tp2", "tp3", "isomero", "chimerica", "progenitore_alog"))

    def parametri(self, forma: str, progenitore: Ione) -> dict:
        u = lambda a, b: round(float(self.rng.uniform(a, b)), 3)          # noqa: E731
        return {"monotona": lambda: dict(kf=u(0.05, 0.25)), "piatto": lambda: dict(kf=5.0), "bateman": lambda: dict(kf=u(0.15, 0.3), kd=u(0.04, 0.1)),
                "decrescente": lambda: dict(kd=u(0.03, 0.08)), "ritardo": lambda: dict(kp=progenitore.k_form, ka=progenitore.k_dec, kb=u(0.01, 0.03))}[forma]()

    def prodotto(self, progenitore: Ione, neutra_p: dict, forma="monotona", generazione=1, ruolo="tp1", sg=None, scala=None, rt=None):
        rng = self.rng
        rt = self.slot() if rt is None else rt
        while True:
            f = somma(neutra_p, TRASF[int(rng.integers(0, len(TRASF)))])
            if self.libero(f, rt):
                break
        self.formule.append(f)
        mid = self.id_mol()
        p = self.parametri(forma, progenitore)
        o = self.ione(mid, ruolo, mid, f, rt, float(rng.uniform(0.045, 0.07)) if sg is None else sg,
                      self.traccia(forma, float(rng.uniform(0.8, 3.0)) * 1e7 if scala is None else scala, **p), progenitore=progenitore.id, generazione=generazione, forma=forma,
                      k_form=p.get("kf", p.get("kp")), k_dec=p.get("kd", p.get("kb")), selezionabile=True)
        if forma == "ritardo":
            o.k_form, o.k_dec = p["ka"], p["kb"]
        o.frammenti = frammenti_di(rng, o.mz)
        return o, f


# ---------------------------------------------------------------------------------------------------------------- the model
def padre_e_famiglia(c: Costruttore):
    """The parent: first-order decay, saturated in the first times (flat apex, constant area, then collapse); isotopes, adducts, two true and two decoy in-source fragments."""
    rng = c.rng
    neutra = formula_inventata(rng)
    c.formule.append(neutra)
    pid = c.id_mol()
    kd = round(float(rng.uniform(0.03, 0.05)), 3)
    scala = 5e7
    padre = c.ione(pid, "progenitore", pid, neutra, c.slot(1.0), 0.06, c.traccia("progenitore", scala, kd=kd), generazione=0, forma="progenitore", k_dec=kd, selezionabile=True)
    padre.clip = np.full(len(c.file), scala * float(np.exp(-kd * c.P["t_sat"])))
    padre.note = f"saturo_fino_a_t={c.P['t_sat']:g};altezza_massima={padre.clip[0]:.4g}"
    c.derivati(padre, neutra, addotti=[("Na", NA_DELTA, "[M+Na]+"), ("NH4", NH4_DELTA, "[M+NH4]+")], isf=2, isf_finti=2)
    return padre, neutra


def prodotti_cinetici(c: Costruttore, padre: Ione, neutra: dict) -> list:
    """First-generation products (one kinetics each, cycling), second-generation ones (delayed, from the products that rise and fall) and extra families."""
    P, out = c.P, []
    for i in range(P["n_tp"] + P["famiglie_extra"]):
        o, f = c.prodotto(padre, neutra, forma=FORME_TP[i % len(FORME_TP)])
        out.append((o, f))
        c.derivati(o, f, addotti=[("Na", NA_DELTA, "[M+Na]+")] if i % 2 == 0 else [("NH4", NH4_DELTA, "[M+NH4]+")] if i % 3 == 0 else [], isf=1 if i % 4 == 0 else 0, isf_finti=1 if i % 4 == 0 else 0)
    bat = [(o, f) for o, f in out if o.forma == "bateman"]
    for i in range(P["n_tp2"]):
        a, fa = bat[i % len(bat)]
        o, f = c.prodotto(a, fa, forma="ritardo", generazione=2, ruolo="tp2")
        c.derivati(o, f, addotti=[("Na", NA_DELTA, "[M+Na]+")] if i % 2 else [])
        out.append((o, f))
    return out


def isomeri(c: Costruttore, padre: Ione, neutra: dict) -> None:
    """Pairs with the same formula, similar MS2, distance and valley between the peaks declared (sigma solved so that the valley is the declared share of the lower apex)."""
    rng = c.rng
    for d, v in c.P["isomeri"]:
        sg = sigma_per_valle(d, v)
        a, f = c.prodotto(padre, neutra, sg=sg, ruolo="isomero", scala=2e7)
        b = c.ione(c.id_mol(), "isomero", a.id, f, a.rt + d, sg, a.amp * 0.8, progenitore=padre.id, generazione=1, forma="monotona", k_form=a.k_form, selezionabile=True, isomero_di=a.id)
        b.frammenti = (a.frammenti[0], a.frammenti[1] * rng.uniform(0.9, 1.1, len(a.frammenti[1])))
        a.note = f"delta_rt={d};valle={v};sigma={sg:.4f};altezze=1:0.8;isomero_di={b.id}"
        b.note = f"delta_rt={d};valle={v};sigma={sg:.4f};altezze=1:0.8;isomero_di={a.id}"
        a.isomero_di = b.id
        c.slot(d + c.P["passo"] * 0.5)


def chimeriche(c: Costruttore, padre: Ione, neutra: dict) -> None:
    """Target and partner co-isolated; the partner follows the target with the ratio that gives the declared true purity."""
    rng = c.rng
    for p in c.P["purezze"]:
        for _ in range(c.P["per_livello"]):
            t, f = c.prodotto(padre, neutra, sg=0.06)
            t.purezza = p
            off = float(rng.uniform(0.25, 0.5)) * (1 if rng.random() < 0.5 else -1)
            par = Ione(id=f"{t.id}_COI", ruolo="chimerica", famiglia=t.id, rt=t.rt + float(rng.uniform(-0.005, 0.005)), sg=t.sg, progenitore=t.id, forma="come_progenitore",
                       mz=round(t.mz + off, 6), amp=t.amp * (1 - p) / p, purezza=p, note=f"delta_mz={off:.3f};ratio={(1 - p) / p:.4f}")
            par.picchi = [(par.mz, 100.0), (par.mz + 1.00335, 14.0)]
            par.frammenti = frammenti_di(rng, par.mz)
            c.ioni.append(par)
            t.note = f"coisolato_con={par.id}"
            c.derivati(t, f, isotopi=False)


def ipotesi(c: Costruttore) -> None:
    """Matrix noise (random m/z and RT, in every file) and the mass hypotheses of the invented rules: those with no signal are decoys (esca), those with a noise
    peak within +-5 ppm are coincidences (the peak is not a product). The expected number by chance goes to c.meta_coinc."""
    rng, P = c.rng, c.P
    n = P["n_rumore"]
    c.rumore = (rng.uniform(*MZ_RUMORE, n), rng.uniform(1.0, c.rt_fine - 0.3, n), rng.uniform(0.04, 0.08, n), rng.lognormal(np.log(6e5), 0.5, n))
    veri = np.sort(np.array([m for o in c.ioni if o.amp is not None for m, _ in o.picchi] + [m for _, m, _, _ in c.fondo]))
    basi = [o for o in c.ioni if o.ruolo in ("progenitore", "tp1", "tp2", "tp3", "isomero")]
    hyp = [(b, r, REGOLE_ESCA[r], b.mz + REGOLE_ESCA[r]) for b in basi for r in range(P["n_regole"])]
    hyp = [h for h in hyp if MZ_RUMORE[0] < h[3] < MZ_RUMORE[1] and (lambda i: min(abs(veri[min(i, len(veri) - 1)] - h[3]), abs(veri[max(i - 1, 0)] - h[3])) > h[3] * 2e-5)(np.searchsorted(veri, h[3]))]
    attese = sum(n * 2 * 5e-6 * h[3] / (MZ_RUMORE[1] - MZ_RUMORE[0]) for h in hyp)
    esche, coinc = [], []
    for b, r, d, m in hyp:
        vic = np.flatnonzero(np.abs(c.rumore[0] - m) / m <= 5e-6)
        if len(vic):
            j = int(vic[np.argmin(np.abs(c.rumore[0][vic] - m))])
            coinc.append(Ione(id=f"SIN-C-{len(coinc) + 1:03d}", ruolo="coincidenza", famiglia=b.id, rt=float(c.rumore[1][j]), sg=float(c.rumore[2][j]), progenitore=b.id,
                              mz=float(c.rumore[0][j]), forma="rumore", note=f"regola=R{r + 1:02d};delta={d};ipotesi_mz={m:.6f};ppm={(c.rumore[0][j] - m) / m * 1e6:.2f}"))
        else:
            esche.append(Ione(id="", ruolo="esca", famiglia=b.id, progenitore=b.id, mz=m, presente=0, forma="ipotesi", note=f"regola=R{r + 1:02d};delta={d};ipotesi_su={b.id}"))
    sel = sorted(rng.permutation(len(esche))[:P["n_esche"]].tolist())
    for i, j in enumerate(sel, 1):
        esche[j].id = f"SIN-E-{i:03d}"
    c.ioni += coinc + [esche[j] for j in sel]
    c.meta_coinc = {"finestra_ppm": 5, "n_ipotesi": len(hyp), "n_rumore": n, "intervallo_mz": list(MZ_RUMORE), "attese": round(attese, 3), "osservate": len(coinc),
                    "n_esche": len(sel), "regole_delta_mz": REGOLE_ESCA[:P["n_regole"]]}


def costruisci(seme: int, profilo: str) -> Costruttore:
    c = Costruttore(seme, profilo)
    padre, neutra = padre_e_famiglia(c)
    prodotti_cinetici(c, padre, neutra)
    isomeri(c, padre, neutra)
    chimeriche(c, padre, neutra)
    if c.P.get("esteso"):
        from valida import sintetici_esteso                    # V1X: chains, in-source fragments of the products, halogens, close isomers
        sintetici_esteso.estendi(c)
    c.fondo = [(f"SIN-BG-{i + 1:02d}", float(c.rng.uniform(150, 600)), float(c.rng.uniform(1e6, 4e6)), float(c.rng.uniform(0, 6.28))) for i in range(c.P["n_fondo"])]
    c.rt_fine = c.rt_cursor + 1.0
    ipotesi(c)
    return c


# ---------------------------------------------------------------------------------------------------------------- truth
COLONNE = ["id", "ruolo", "mz", "rt_obs", "ion", "famiglia", "progenitore", "generazione", "forma", "purezza", "isf_in_ms2", "presente", "k_form", "k_dec", "note"]


def riga(o: Ione) -> dict:
    return {"id": o.id, "ruolo": o.ruolo, "mz": round(o.mz, 6), "rt_obs": [round(o.rt, 4)] if o.presente else [], "ion": o.ion, "famiglia": o.famiglia, "progenitore": o.progenitore,
            "generazione": o.generazione, "forma": o.forma, "purezza": o.purezza, "isf_in_ms2": o.isf_in_ms2, "presente": o.presente, "k_form": o.k_form, "k_dec": o.k_dec, "note": o.note}


def righe_verita(c: Costruttore) -> list[dict]:
    rows = [riga(o) for o in c.ioni]
    vuota = {"rt_obs": [], "ion": "", "generazione": None, "purezza": None, "isf_in_ms2": None, "presente": 1, "k_form": None, "k_dec": None}
    for nome, m, a, _ in c.fondo:
        rows.append({"id": nome, "ruolo": "fondo", "mz": round(m, 6), "famiglia": nome, "progenitore": "", "forma": "continuo", "note": f"altezza={a:.3g};tracce=tutte le scansioni", **vuota})
        for j, d in enumerate((-2.5e-5, 2.5e-5), 1):
            rows.append({"id": f"{nome}_L{j}", "ruolo": "fondo", "mz": round(m * (1 + d), 6), "famiglia": nome, "progenitore": nome, "forma": "lobo_FT", "note": "ratio=0.02", **vuota})
    return rows


def scrivi_verita(out: Path, c: Costruttore, seme: int, profilo: str) -> None:
    rows = righe_verita(c)
    with open(out / "verita.csv", "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f, lineterminator="\n")
        w.writerow(COLONNE)
        for r in rows:
            w.writerow([";".join(f"{x:.4f}" for x in r[k]) if k == "rt_obs" else "" if r[k] is None else f"{r[k]:.6f}" if k == "mz" else r[k] for k in COLONNE])
    padre = next(o for o in c.ioni if o.ruolo == "progenitore")
    d = {"_note": "Verità dei dati sintetici di tools/valida/sintetici_identita.py: tutte le molecole e i numeri sono inventati. `parent` e `tps` hanno il formato di timolol_truth.json; `ioni` ha tutte le righe.",
         "meta": {"seme": seme, "profilo": profilo, "colonne": COLONNE, "coincidenze": c.meta_coinc, "t_saturazione": c.P["t_sat"]},
         "parent": {"name": padre.id, "ion": padre.ion, "mz": round(padre.mz, 6), "rt_obs": round(padre.rt, 4), "notes": padre.note, "k_dec": padre.k_dec},
         "tps": [r for r in rows if r["ruolo"] in ("tp1", "tp2", "tp3", "isomero")], "ioni": rows, "file": [{"nome": f"SIN_{s}", "tipo": k, "time": t} for s, k, t in c.file]}
    (out / "verita.json").write_bytes((json.dumps(d, indent=1, ensure_ascii=False) + "\n").encode("utf-8"))      # bytes: no CRLF on Windows


def genera(seme: int, profilo: str, out) -> Path:
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    c = costruisci(seme, profilo)
    for k, (stem, _, _) in enumerate(c.file):
        scrivi_file(out / f"SIN_{stem}.mzML", k, c, seme)
    scrivi_verita(out, c, seme, profilo)
    if c.P.get("esteso"):
        from valida import sintetici_esteso_verita
        sintetici_esteso_verita.scrivi(out, c, seme, profilo)
    return out


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--seme", type=int, default=7)
    ap.add_argument("--profilo", choices=sorted(PROFILI), default="piccolo")
    ap.add_argument("--esteso", action="store_true", help="V1X: second and third generations, in-source fragments of the products, halogens, close isomers (+ verita_tp.json)")
    ap.add_argument("--out", required=True)
    a = ap.parse_args(argv)
    print(genera(a.seme, {"completo": "V1X", "piccolo": "V1X-piccolo"}.get(a.profilo, a.profilo) if a.esteso else a.profilo, a.out))
    return 0


if __name__ == "__main__":
    sys.exit(main())
