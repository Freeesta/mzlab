#!/usr/bin/env python3
"""Synthetic HR series with an identity truth: ion families, in-source fragments, isomers, chimeric MS2, blank / dark / t0 (every molecule and number is invented).

    python3 tools/valida/sintetici_identita.py --seme 7 --profilo piccolo|completo --out DIR

Writes in DIR one Orbitrope-like DDA mzML per sample (SIN_blank, SIN_dark, SIN_t0, SIN_tNNmin...) and the truth, `verita.csv` and `verita.json`. The same seed gives
the same bytes. The JSON is the format of the real truth files (`parent`, `tps`: read as they are by tools/valida_feat.py) plus `ioni` (every row of the truth) and `file`.

Truth columns: id, ruolo, mz, rt_obs[], ion, famiglia, progenitore, generazione, forma, purezza, isf_in_ms2, presente, k_form, k_dec, note.
- ruolo: progenitore, tp1, tp2, isotopo, addotto, isf, isf_finto, isomero, chimerica, esca, coincidenza, fondo. `tps` of the JSON = tp1, tp2, isomero (true products).
- famiglia: id of the molecule the ion belongs to (the isotopes, adducts and in-source fragments of an ion share the family of the ion).
- progenitore: for a product the parent molecule; for isotopo / addotto / isf / isf_finto / chimerica the id of the ion they belong to or contaminate.
- generazione: 1 for products of the parent, 2 for products of a product; forma: kinetics (progenitore, monotona, come_progenitore, ...).
- purezza: true purity of the isolated precursor in the MS2 (share of the ion current of the target in the isolation window); on target and partner.
- isf_in_ms2: 1 if the m/z of the in-source fragment is a fragment in the MS2 of its parent, 0 if it is not (decoy); empty otherwise.
- presente: 0 for the decoys that are not in the data. k_form, k_dec: rate constants (1/min). note: `chiave=valore;...` (ratio, delta_rt, valle, ...).
Intensities are peak heights of the most abundant component of the ion in the file; derived ions (isotopes, adducts, in-source fragments) follow the
intensity of their base ion with a constant, declared ratio.
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
from valida import ROOT                                  # noqa: E402,F401  (puts `mzlab` and `tpmine` on sys.path)
from valida.isotopologhi import componenti               # noqa: E402
import dati_sintetici as ds                              # noqa: E402
from mzlab.chem import elements as E                     # noqa: E402
from mzlab.chem import isoscore                          # noqa: E402

CICLO = 0.01                  # min between two survey scans
RT0 = 0.5
ISO_SEMI = 0.6                # half of the isolation window (1.2 m/z)
SOGLIA_MS2 = 5e4              # minimum height of the apex to trigger a fragmentation
TOP_N = 3
SD_PPM = 0.8e-6
RUMORE = 100                  # noise peaks per survey scan
NA_DELTA, NH4_DELTA = 21.98194, 17.02655          # [M+Na]+ and [M+NH4]+ minus [M+H]+ (checked by the tests against the element table)
ECCEZIONI = ("blank", "dark", "t0")

PROFILI = {
    "piccolo": dict(file=[("blank", "blank", 0.0), ("dark", "dark", 0.0), ("t0", "t0", 0.0), ("t05min", "trat", 5.0), ("t10min", "trat", 10.0), ("t20min", "trat", 20.0)],
                    n_tp=3, isomeri=[(0.05, 0.3), (0.1, 0.7), (0.2, 0.3), (0.3, 0.7)], purezze=[0.95, 0.7, 0.5, 0.3], per_livello=1, n_fondo=4, passo=0.7, famiglie_extra=0),
    "completo": dict(file=[("blank", "blank", 0.0), ("dark", "dark", 0.0), ("t0", "t0", 0.0)]
                     + [(f"t{int(t):02d}min", "trat", float(t)) for t in (1, 2, 3, 5, 7, 10, 15, 20, 30, 45, 60, 90)],
                     n_tp=10, isomeri=[(d, v) for d in (0.05, 0.1, 0.2, 0.3) for v in (0.3, 0.7)] * 4, purezze=[0.95, 0.7, 0.5, 0.3], per_livello=20, n_fondo=12, passo=0.1, famiglie_extra=30),
}
LOSSES = ["C4H8", "H2O", "C2H4", "CH2O", "C3H6", "NH3", "C2H2O", "CO", "C2H6", "CH4O", "C3H4", "C4H6", "H2S", "C2H3N"]
TRASF = [{"O": 1}, {"H": -2}, {"O": 2}, {"C": 1, "H": 2}, {"C": -1, "H": -2}, {"H": -2, "O": 1}, {"O": 1, "H": 2}, {"C": 2, "H": 4}, {"H": -4, "O": 1},
         {"C": -1, "O": 1, "H": -2}, {"O": 3}, {"C": 1, "O": 1}, {"H": -2, "O": 2}, {"C": 1, "H": 2, "O": 1}, {"N": -1, "O": 1, "H": -1}, {"H": 2}]


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


def fattore(forma: str, t: float, tipo: str, k_form: float | None, k_dec: float | None) -> float:
    """Relative amount of an ion in a file of the series (0 in the blank). Parent: first-order decay; product: monotonic formation (reaches 1)."""
    if tipo == "blank":
        return 0.0
    if forma == "progenitore":
        return float(np.exp(-k_dec * t))
    if forma == "monotona":
        return float(1 - np.exp(-k_form * t))
    raise ValueError(forma)


# ---------------------------------------------------------------------------------------------------------------- the model
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
    def derivati(self, base: Ione, neutra: dict, isotopi=True, addotti=(), isf=0, isf_finti=0, perdite=None):
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

    def traccia(self, forma, k_form=None, k_dec=None, scala=1.0) -> np.ndarray:
        return scala * np.array([fattore(forma, t, k, k_form, k_dec) for t, k in zip(self.tempi, self.tipi)])

    def ione(self, id, ruolo, famiglia, neutra: dict | None, rt, sg, amp, adduct="[M+H]+", **kw) -> Ione:
        """An ion of a molecule (neutral formula) or, with neutra None, an ion without formula (`ion` given in kw as m/z via `mz`)."""
        o = Ione(id=id, ruolo=ruolo, famiglia=famiglia, rt=float(rt), sg=float(sg), amp=amp, **kw)
        if neutra is not None:
            add = {"[M+H]+": {"H": 1}, "[M+Na]+": {"Na": 1}, "[M+NH4]+": {"N": 1, "H": 4}}[adduct]
            o.ion = E.fmt(somma(neutra, add))
            o.picchi, mz, rel = modello(o.ion)
            o.mz = float(mz[0])
        self.ioni.append(o)
        return o

    def prodotto(self, progenitore: Ione, neutra_p: dict, nome=None, rt=None, sg=None, forma="monotona", generazione=1, ruolo="tp1", scala=None, trasf=None):
        rng = self.rng
        while True:
            t = TRASF[int(rng.integers(0, len(TRASF)))] if trasf is None else trasf
            f = somma(neutra_p, t)
            if valida_formula(f) and all(f != g for g in self.formule):
                break
            if trasf is not None:
                raise ValueError("duplicate formula")
        self.formule.append(f)
        mid = nome or self.id_mol()
        kf = round(float(rng.uniform(0.05, 0.25)), 3)
        o = self.ione(mid, ruolo, mid, f, self.slot() if rt is None else rt, float(rng.uniform(0.045, 0.07)) if sg is None else sg,
                      self.traccia(forma, k_form=kf, scala=float(rng.uniform(0.8, 3.0)) * 1e7 if scala is None else scala), progenitore=progenitore.id, generazione=generazione,
                      forma=forma, k_form=kf, selezionabile=True)
        o.frammenti = frammenti_di(rng, o.mz)
        return o, f


def costruisci(seme: int, profilo: str) -> Costruttore:
    c = Costruttore(seme, profilo)
    rng, P = c.rng, c.P
    # --- parent with its family: isotopes, adducts, two true and two decoy in-source fragments
    neutra = formula_inventata(rng)
    c.formule.append(neutra)
    pid = c.id_mol()
    kd = round(float(rng.uniform(0.04, 0.07)), 3)
    padre = c.ione(pid, "progenitore", pid, neutra, c.slot(1.0), 0.06, c.traccia("progenitore", k_dec=kd, scala=5e7), generazione=0, forma="progenitore", k_dec=kd, selezionabile=True)
    c.derivati(padre, neutra, addotti=[("Na", NA_DELTA, "[M+Na]+"), ("NH4", NH4_DELTA, "[M+NH4]+")], isf=2, isf_finti=2)
    # --- first-generation products with their families
    prodotti = []
    for i in range(P["n_tp"]):
        o, f = c.prodotto(padre, neutra)
        prodotti.append((o, f))
        c.derivati(o, f, addotti=[("Na", NA_DELTA, "[M+Na]+")] if i % 2 == 0 else [], isf=1 if i == 0 else 0)
    for i in range(P["famiglie_extra"]):                       # more families (full profile): products with isotopes and adducts
        o, f = c.prodotto(padre, neutra)
        c.derivati(o, f, addotti=[("NH4", NH4_DELTA, "[M+NH4]+")] if i % 3 == 0 else [], isf=1 if i % 4 == 0 else 0)
    # --- isomer pairs: same formula, similar MS2, distance and valley declared
    for d, v in P["isomeri"]:
        sg = sigma_per_valle(d, v)
        a, f = c.prodotto(padre, neutra, sg=sg, ruolo="isomero", scala=2e7)
        rt_b = a.rt + d
        b = c.ione(c.id_mol(), "isomero", a.id, f, rt_b, sg, a.amp * 0.8, progenitore=padre.id, generazione=1, forma="monotona", k_form=a.k_form, selezionabile=True,
                   isomero_di=a.id)
        b.famiglia = a.famiglia
        b.frammenti = (a.frammenti[0], a.frammenti[1] * rng.uniform(0.9, 1.1, len(a.frammenti[1])))
        a.note = f"delta_rt={d};valle={v};sigma={sg:.4f};altezze=1:0.8;isomero_di={b.id}"
        b.note = f"delta_rt={d};valle={v};sigma={sg:.4f};altezze=1:0.8;isomero_di={a.id}"
        a.isomero_di = b.id
        c.slot(0.5 + d)
    # --- chimeric MS2: target and partner co-isolated, true purity declared
    for p in P["purezze"]:
        for _ in range(P["per_livello"]):
            t, f = c.prodotto(padre, neutra, sg=0.06)
            t.purezza = p
            # partner: no formula, m/z inside the isolation window, same shape, amplitude fixed by the purity
            off = float(rng.uniform(0.25, 0.5)) * (1 if rng.random() < 0.5 else -1)
            par = Ione(id=f"{t.id}_COI", ruolo="chimerica", famiglia=t.id, rt=t.rt + float(rng.uniform(-0.005, 0.005)), sg=t.sg, progenitore=t.id, forma="come_progenitore",
                       mz=round(t.mz + off, 6), amp=t.amp * (1 - p) / p, purezza=p, note=f"delta_mz={off:.3f};ratio={(1 - p) / p:.4f}")
            par.picchi = [(par.mz, 100.0), (par.mz + 1.00335, 14.0)]
            par.frammenti = frammenti_di(rng, par.mz)
            c.ioni.append(par)
            t.note = f"coisolato_con={par.id}"
            c.derivati(t, f, isotopi=False)
    # --- continuous background (all the run, in every file) and its FT lobes
    c.fondo = []
    for i in range(P["n_fondo"]):
        m = float(rng.uniform(150, 600))
        c.fondo.append((f"SIN-BG-{i + 1:02d}", m, float(rng.uniform(1e6, 4e6)), float(rng.uniform(0, 6.28))))
    c.rt_fine = c.rt_cursor + 1.0
    return c


# ---------------------------------------------------------------------------------------------------------------- mzML
def _arr(acc, name, data, f4=False) -> str:
    cv = ('<cvParam cvRef="MS" accession="MS:1000521" name="32-bit float" value=""/>' if f4 else '<cvParam cvRef="MS" accession="MS:1000523" name="64-bit float" value=""/>')
    return (f'<binaryDataArray encodedLength="0">{cv}<cvParam cvRef="MS" accession="MS:1000574" name="zlib compression" value=""/>'
            f'<cvParam cvRef="MS" accession="{acc}" name="{name}" value=""/><binary>{ds._b64(data, f4) if len(data) else ""}</binary></binaryDataArray>')


def _scan(idx, rt, level, mz, it, scan_extra="", prec="", filt="", res=0) -> str:
    n = len(mz)
    b = int(np.argmax(it)) if n else 0
    cv = ds._cvp
    return (f'<spectrum index="{idx}" id="controllerType=0 controllerNumber=1 scan={idx + 1}" defaultArrayLength="{n}">'
            + cv("MS:1000579" if level == 1 else "MS:1000580", "MS1 spectrum" if level == 1 else "MSn spectrum") + cv("MS:1000511", "ms level", level)
            + cv("MS:1000130", "positive scan") + cv("MS:1000127", "centroid spectrum")
            + cv("MS:1000504", "base peak m/z", f"{mz[b]:.6f}" if n else 0, ds._U_MZ) + cv("MS:1000505", "base peak intensity", f"{it[b]:.6g}" if n else 0)
            + cv("MS:1000285", "total ion current", f"{float(np.sum(it)):.6g}")
            + f'<scanList count="1">{cv("MS:1000795", "no combination")}<scan{scan_extra}>'
            + cv("MS:1000016", "scan start time", f"{rt:.6f}", ' unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"')
            + cv("MS:1000800", "mass resolving power", res) + cv("MS:1000512", "filter string", filt)
            + '<scanWindowList count="1"><scanWindow>' + cv("MS:1000501", "scan window lower limit", 50.0, ds._U_MZ) + cv("MS:1000500", "scan window upper limit", 900.0, ds._U_MZ)
            + '</scanWindow></scanWindowList></scan></scanList>' + prec + '<binaryDataArrayList count="2">'
            + _arr("MS:1000514", "m/z array", mz) + _arr("MS:1000515", "intensity array", it, f4=True) + "</binaryDataArrayList></spectrum>")


def _precursore(idx_ms1, m, e, vint) -> str:
    cv = ds._cvp
    return ('<precursorList count="1">' + f'<precursor spectrumRef="controllerType=0 controllerNumber=1 scan={idx_ms1 + 1}"><isolationWindow>'
            + cv("MS:1000827", "isolation window target m/z", f"{m:.6f}", ds._U_MZ) + cv("MS:1000828", "isolation window lower offset", ISO_SEMI, ds._U_MZ)
            + cv("MS:1000829", "isolation window upper offset", ISO_SEMI, ds._U_MZ) + '</isolationWindow><selectedIonList count="1"><selectedIon>'
            + cv("MS:1000744", "selected ion m/z", f"{e:.6f}", ds._U_MZ) + cv("MS:1000041", "charge state", 1) + cv("MS:1000042", "peak intensity", f"{vint:.6g}")
            + '</selectedIon></selectedIonList><activation>' + cv("MS:1000422", "beam-type collision-induced dissociation")
            + cv("MS:1000045", "collision energy", 30.0, ' unitCvRef="UO" unitAccession="UO:0000266" unitName="electronvolt"') + '</activation></precursor></precursorList>')


def scrivi_file(path: Path, k: int, c: Costruttore, seme: int) -> dict:
    """One file of the series: survey scans every CICLO min and, at the apex of every selectable ion, an MS2 (top-N per cycle; the ions in the isolation window are all fragmented)."""
    rng = np.random.default_rng([seme, 1000 + k])
    ioni = [o for o in c.ioni if o.amp is not None and o.amp[k] >= 2e3 and o.picchi]
    cm = np.array([m for o in ioni for m, _ in o.picchi])
    cr = np.array([o.rt for o in ioni for _ in o.picchi])
    cs = np.array([o.sg for o in ioni for _ in o.picchi])
    ch = np.array([o.amp[k] * r / 100.0 for o in ioni for _, r in o.picchi])
    mono, pos = [], 0
    for o in ioni:
        mono.append(pos)
        pos += len(o.picchi)
    mono = np.array(mono, int)
    imz = np.array([o.mz for o in ioni])
    ion_di = np.array([i for i, o in enumerate(ioni) for _ in o.picchi], int)
    bg = np.array([[m, a, ph] for _, m, a, ph in c.fondo]) if c.fondo else np.zeros((0, 3))
    lobi = []
    for m, a, _ in bg:                                   # FT lobes: two small peaks around every strong background ion
        for d, r in ((-2.5e-5, 0.02), (2.5e-5, 0.02)):
            lobi.append((m * (1 + d), r))
    rts = np.arange(RT0, c.rt_fine, CICLO)
    # MS2 schedule: apex cycle of every selectable ion, at most TOP_N per cycle
    carico, pian = {}, {}
    for j, o in enumerate(ioni):
        if o.selezionabile and o.amp[k] >= SOGLIA_MS2:
            n = int(round((o.rt - RT0) / CICLO))
            while carico.get(n, 0) >= TOP_N:
                n += 1
            carico[n] = carico.get(n, 0) + 1
            pian.setdefault(n, []).append(j)
    specs, tic, rt_tic, idx, n_ms2 = [], [], [], 0, 0
    filt1, filt2 = "FTMS + p ESI Full ms [100.0000-900.0000]", "FTMS + p ESI d Full ms2 {p:.4f}@hcd30.00 [50.0000-{q:.4f}]"
    for n, rt in enumerate(rts):
        v = ch * np.exp(-0.5 * ((rt - cr) / cs) ** 2) * rng.normal(1, 0.03, len(ch))
        mz_s = cm * (1 + rng.normal(0, SD_PPM, len(cm)))
        keep = v >= 2e3
        mz = [rng.uniform(100, 900, RUMORE), mz_s[keep]]
        it = [rng.lognormal(np.log(8e3), 0.7, RUMORE), v[keep]]
        if len(bg):
            hb = bg[:, 1] * (1 + 0.1 * np.sin(rt / 2.0 + bg[:, 2])) * rng.normal(1, 0.03, len(bg))
            mz += [bg[:, 0] * (1 + rng.normal(0, SD_PPM, len(bg))), np.array([m for m, _ in lobi]) * (1 + rng.normal(0, SD_PPM, len(lobi)))]
            it += [hb, np.repeat(hb, 2) * np.array([r for _, r in lobi]) * rng.normal(1, 0.05, len(lobi))]
        mz, it = np.concatenate(mz), np.concatenate(it)
        o_ = np.argsort(mz, kind="stable")
        mz, it = mz[o_], it[o_]
        i1 = idx
        specs.append(_scan(idx, rt, 1, mz, it, filt=filt1, res=60000))
        tic.append(float(it.sum())); rt_tic.append(rt); idx += 1
        for s, j in enumerate(pian.get(n, [])):
            o = ioni[j]
            e = float(mz_s[mono[j]])
            win = np.flatnonzero((np.abs(imz - o.mz) <= ISO_SEMI) & (v[mono] >= 2e3)).tolist()
            fm, fi = [], []
            for i in win:
                fr = ioni[i].frammenti
                if fr is None:
                    continue
                fm.append(fr[0] * (1 + rng.normal(0, SD_PPM, len(fr[0]))))
                fi.append(fr[1] * v[mono[i]] * 0.2 * rng.normal(1, 0.05, len(fr[0])))
            fm.append(rng.uniform(60, o.mz, 8)); fi.append(rng.lognormal(np.log(1.5e3), 0.5, 8))
            fm, fi = np.concatenate(fm), np.concatenate(fi)
            q = np.argsort(fm, kind="stable")
            specs.append(_scan(idx, rt + 0.0015 * (s + 1), 2, fm[q], fi[q], prec=_precursore(i1, o.mz, e, float(v[mono[j]])), filt=filt2.format(p=o.mz, q=o.mz + 10), res=15000))
            idx += 1; n_ms2 += 1
    chroms = [ds._chrom(0, "TIC", np.array(rt_tic), np.array(tic))]
    stem = path.stem
    path.write_text(ds._hr_head("exploris", False, stem) + f'<spectrumList count="{len(specs)}">' + "".join(specs) + "</spectrumList>"
                    + f'<chromatogramList count="{len(chroms)}">' + "".join(chroms) + "</chromatogramList></run></mzML>", encoding="utf-8")
    return {"file": path.name, "n_ms1": len(rts), "n_ms2": n_ms2}


# ---------------------------------------------------------------------------------------------------------------- truth
COLONNE = ["id", "ruolo", "mz", "rt_obs", "ion", "famiglia", "progenitore", "generazione", "forma", "purezza", "isf_in_ms2", "presente", "k_form", "k_dec", "note"]


def riga(o: Ione) -> dict:
    return {"id": o.id, "ruolo": o.ruolo, "mz": round(o.mz, 6), "rt_obs": [round(o.rt, 4)] if o.presente else [], "ion": o.ion, "famiglia": o.famiglia, "progenitore": o.progenitore,
            "generazione": o.generazione, "forma": o.forma, "purezza": o.purezza, "isf_in_ms2": o.isf_in_ms2, "presente": o.presente, "k_form": o.k_form, "k_dec": o.k_dec, "note": o.note}


def righe_verita(c: Costruttore) -> list[dict]:
    rows = [riga(o) for o in c.ioni]
    for nome, m, a, _ in c.fondo:
        rows.append({"id": nome, "ruolo": "fondo", "mz": round(m, 6), "rt_obs": [], "ion": "", "famiglia": nome, "progenitore": "", "generazione": None, "forma": "continuo",
                     "purezza": None, "isf_in_ms2": None, "presente": 1, "k_form": None, "k_dec": None, "note": f"altezza={a:.3g};tracce=tutte le scansioni"})
        for j, d in enumerate((-2.5e-5, 2.5e-5), 1):
            rows.append({"id": f"{nome}_L{j}", "ruolo": "fondo", "mz": round(m * (1 + d), 6), "rt_obs": [], "ion": "", "famiglia": nome, "progenitore": nome, "generazione": None,
                         "forma": "lobo_FT", "purezza": None, "isf_in_ms2": None, "presente": 1, "k_form": None, "k_dec": None, "note": "ratio=0.02"})
    return rows


def scrivi_verita(out: Path, c: Costruttore, seme: int, profilo: str) -> None:
    rows = righe_verita(c)
    with open(out / "verita.csv", "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f, lineterminator="\n")
        w.writerow(COLONNE)
        for r in rows:
            w.writerow([";".join(f"{x:.4f}" for x in r[k]) if k == "rt_obs" else "" if r[k] is None else f"{r[k]:.6f}" if k == "mz" else r[k] for k in COLONNE])
    padre = next(o for o in c.ioni if o.ruolo == "progenitore")
    tps = [r for r in rows if r["ruolo"] in ("tp1", "tp2", "isomero")]
    d = {"_note": "Verità dei dati sintetici di tools/valida/sintetici_identita.py: tutte le molecole e i numeri sono inventati. `parent` e `tps` hanno il formato di timolol_truth.json; `ioni` ha tutte le righe.",
         "meta": {"seme": seme, "profilo": profilo, "colonne": COLONNE},
         "parent": {"name": padre.id, "ion": padre.ion, "mz": round(padre.mz, 6), "rt_obs": round(padre.rt, 4), "notes": padre.note, "k_dec": padre.k_dec},
         "tps": tps, "ioni": rows, "file": [{"nome": f"SIN_{s}", "tipo": k, "time": t} for s, k, t in c.file]}
    (out / "verita.json").write_text(json.dumps(d, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")


def genera(seme: int, profilo: str, out) -> Path:
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    c = costruisci(seme, profilo)
    for k, (stem, _, _) in enumerate(c.file):
        scrivi_file(out / f"SIN_{stem}.mzML", k, c, seme)
    scrivi_verita(out, c, seme, profilo)
    return out


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--seme", type=int, default=7)
    ap.add_argument("--profilo", choices=sorted(PROFILI), default="piccolo")
    ap.add_argument("--out", required=True)
    a = ap.parse_args(argv)
    print(genera(a.seme, a.profilo, a.out))
    return 0


if __name__ == "__main__":
    sys.exit(main())
