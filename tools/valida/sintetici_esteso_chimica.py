"""V1X, modules M3 and M4: halogenated progenitors with dealogenated products, and groups of close isomers (tools/valida/sintetici_esteso.py).

- M3 `alogenati`: progenitors with Cl, Br, ClBr and Cl2 (the isotope pattern of the program's isotope model: 35/37Cl 3:1, 79/81Br 1:1, Cl2 9:6:1). Each one has a product that keeps
  the halogens (+O) and the dealogenated products -X+H and -X+OH for every halogen type (simplified pattern). The truth carries the halogens kept by every row.
- M4 `isomeri_vicini`: groups of two or three isomers (same m/z) whose RT distance is taken from {0.05, 0.1, 0.15, 0.2, 0.3} min, with a declared FWHM (0.08-0.15 min) and
  fragments that overlap only in part (the second isomer keeps 70% of the first one's fragments, the third 30%)."""
from __future__ import annotations

import numpy as np

from mzlab.chem import elements as E
from valida.sintetici_esteso import testo_delta  # noqa: F401
from valida.sintetici_identita import Ione, formula_inventata, frammenti_di, somma, valida_formula, valle_relativa

SPECIE = [("Cl", {"Cl": 1}), ("Br", {"Br": 1}), ("ClBr", {"Cl": 1, "Br": 1}), ("Cl2", {"Cl": 2})]
DEALOG = {"Cl": [("H", {"Cl": -1, "H": 1}), ("OH", {"Cl": -1, "O": 1, "H": 1})], "Br": [("H", {"Br": -1, "H": 1}), ("OH", {"Br": -1, "O": 1, "H": 1})]}
DT = (0.05, 0.1, 0.15, 0.2, 0.3)
FWHM = 2.3548200450309493                      # sigma -> FWHM


def parametri(rng, forma: str) -> dict:
    u = lambda a, b: round(float(rng.uniform(a, b)), 3)          # noqa: E731
    return {"monotona": lambda: dict(kf=u(0.05, 0.25)), "bateman": lambda: dict(kf=u(0.15, 0.3), kd=u(0.04, 0.1))}[forma]()


def progenitore_alogenato(c, rng, alog: dict):
    """A halogenated progenitor: an invented formula without the halogen of the base model, the given halogens in place of hydrogens. Returns (formula, RT)."""
    while True:
        f = {e: n for e, n in formula_inventata(rng).items() if e != "Cl"}
        f["H"] -= sum(alog.values())
        f.update(alog)
        if valida_formula(f) and not any(f == x for x in c.formule):
            rt = c.slot()
            if c.libero(f, rt):
                return f, rt


def prodotti_alogenato(c, st, rng, p: Ione, f: dict, alog: dict, nome: str) -> None:
    """The product that keeps the halogens (+O) and, for every halogen type, -X+H and -X+OH."""
    passi = [("idrossilazione", "+O", {"O": 1})] + [(f"dealogenazione_{x}_{cl}", f"-{x}+{cl}", d) for x in sorted(alog) for cl, d in DEALOG[x]]
    for j, (classe, testo, delta) in enumerate(passi):
        forma = "bateman" if j % 2 == 0 else "monotona"
        g = somma(f, delta)
        if any(g == x for x in c.formule):                   # the same formula as another molecule: no product
            continue
        for _ in range(5):                                   # an RT whose neighbours are too close in m/z is skipped (the exclusion window is 0.6 min)
            rt = c.slot()
            if c.libero(g, rt):
                break
            c.slot(0.7)
        if not valida_formula(g) or not c.libero(g, rt):
            continue
        mid = c.id_mol()
        o = c.ione(mid, "tp1", mid, g, rt, float(rng.uniform(0.045, 0.07)), c.traccia(forma, float(rng.uniform(0.8, 3.0)) * 1e7, **parametri(rng, forma)), progenitore=p.id,
                   generazione=1, forma=forma, selezionabile=True)
        o.frammenti = frammenti_di(rng, o.mz)
        c.formule.append(g)
        st.neutre[o.id] = g
        st.meta[o.id] = dict(classe=classe, delta=testo, via="regola", alogeni=sorted(sum(([e] * k for e, k in g.items() if e in ("Cl", "Br")), [])), t_max=None, raggiungibile=True,
                             debole=False, specie=nome)
        if classe.startswith("dealogenazione"):
            st.cont[f"dealog_{nome}_{classe.rsplit('_', 1)[1]}"] += 1


def alogenati(c, st, rng) -> None:
    for i in range(int(st.P["n_alogenati"])):
        nome, alog = SPECIE[i % len(SPECIE)]
        f, rt = progenitore_alogenato(c, rng, alog)
        c.formule.append(f)
        pid = c.id_mol()
        kd = round(float(rng.uniform(0.03, 0.08)), 3)
        p = c.ione(pid, "progenitore_alog", pid, f, rt, 0.06, c.traccia("progenitore", 2e7, kd=kd), generazione=0, forma="progenitore", k_dec=kd, selezionabile=True)
        p.frammenti = frammenti_di(rng, p.mz)
        st.meta[p.id] = dict(alogeni=sorted(sum(([e] * k for e, k in alog.items()), [])), specie=nome)
        st.neutre[p.id] = f
        st.cont[f"alog_{nome}"] += 1
        prodotti_alogenato(c, st, rng, p, f, alog, nome)


def n_frammenti(rng, mz: float, n: int, evita=()) -> tuple:
    """`n` invented fragments of a precursor (sorted), none near an `evita` m/z."""
    f, r = [], []
    while len(f) < n:
        a, b = frammenti_di(rng, mz, evita=list(evita) + f)
        f += [x for x in a[:n - len(f)]]
        r += [x for x in b[:n - len(r)]]
    o = np.argsort(f, kind="stable")
    return np.array(f)[o], np.array(r)[o]


def frammenti_gruppo(rng, mz: float, quota: float, base=None, n: int = 10) -> tuple:
    """Fragments of an isomer: `n` in all, a share `quota` of them taken from `base` (the first isomer's), the rest new."""
    if base is None:
        return n_frammenti(rng, mz, n)
    k = int(round(quota * n))
    sel = sorted(rng.permutation(len(base[0]))[:k].tolist())
    nuovi = n_frammenti(rng, mz, n - k, evita=base[0])
    f = np.concatenate([base[0][sel], nuovi[0]])
    r = np.concatenate([base[1][sel], nuovi[1]])
    o = np.argsort(f, kind="stable")
    return f[o], r[o]


def isomeri_vicini(c, st, rng) -> None:
    """`gruppi_per_dt` groups for every ΔRT of DT; groups of three on the odd rounds."""
    padre = next(o for o in c.ioni if o.ruolo == "progenitore")
    for j in range(len(DT) * int(st.P["gruppi_per_dt"])):
        d = DT[j % len(DT)]
        size = 3 if (j // len(DT)) % 2 == 1 else 2
        fw = float(rng.uniform(0.08, min(0.15, d) if d >= 0.1 else 0.15))
        sg = fw / FWHM
        rt = c.slot(d * (size - 1) + c.P["passo"] * 0.5)
        while True:
            f = somma(c.formule[0], {"C": int(rng.integers(-3, 3)), "O": int(rng.integers(0, 4)), "H": 2 * int(rng.integers(-3, 4))})
            if f != c.formule[0] and valida_formula(f) and not any(f == x for x in c.formule) and c.libero(f, rt):
                break
        c.formule.append(f)
        kf = round(float(rng.uniform(0.05, 0.25)), 3)
        scala = float(rng.uniform(1.0, 2.5)) * 1e7
        sep = valle_relativa(d, sg, 0.8) < 0.98
        membri = []
        for m in range(size):
            mid = c.id_mol()
            o = c.ione(mid, "isomero", membri[0].id if membri else mid, f, rt + m * d, sg, c.traccia("monotona", scala * 0.8 ** m, kf=kf), progenitore=padre.id, generazione=1, forma="monotona",
                       k_form=kf, selezionabile=True, isomero_di=membri[0].id if membri else "")
            o.frammenti = frammenti_gruppo(rng, o.mz, (1.0, 0.7, 0.3)[m], membri[0].frammenti if membri else None)
            o.note = f"delta_rt={d};fwhm={fw:.4f};sigma={sg:.4f};separabile={int(sep)}"
            membri.append(o)
        st.cont[f"iso_{d:g}"] += 1
