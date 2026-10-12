"""Extended synthetic series V1X: second and third generation chains, in-source fragments of the products, inherited ions that look like products.

The profile `V1X` (or `V1X-piccolo`) of tools/valida/sintetici_identita.py runs `estendi(c)` after the V1 model is built. Everything here uses its own random streams
(`flusso`: the seed plus a fixed label), never `c.rng`, so the V1 part of the model is the same as in the profile it extends. Halogens and close isomers are in
sintetici_esteso_chimica.py, the truth in sintetici_esteso_verita.py. Every molecule and number is invented; the progenitor is a generic formula.

Modules (parameters in `PROFILI[...]["esteso"]`, table in docs/agenti/mzfinder-validazione.md):
- M1 `catene`: chains progenitor -> TP1 -> TP2 (-> TP3) with the steps taken from the `fotocatalisi` rules table; the total change of a TP is never one rule of the table
  (and, for a third generation, never two): the observed network or a cleavage is needed. One family in the manner of a cleavage followed by oxidations, one chain whose
  third generation is a trace (3-10 times the noise). Delayed maxima: the amount of generation n is the n-th species of a chain of first-order steps.
- M2 `isf_dei_tp`: an in-source fragment with a constant area ratio for a share of the products; `n_isf_variabile` products that co-elute with the in-source fragment of
  another product but follow their own kinetics (the ratio is not constant); cascade of in-source fragments, K adduct and dimer of the progenitor.
- M5 `travestiti`: in-source fragments and adducts of products with an RT delay of exactly 0 or 0.02 min."""
from __future__ import annotations

import zlib
from collections import Counter

import numpy as np

from mzlab.chem import elements as E
from valida.sintetici_identita import ESTESO, LOSSES, Ione, curva, frammenti_di, modello, somma, valida_formula

REGOLE = [("idrossilazione", {"O": 1}), ("idrossilazione", {"O": 2}), ("idrossilazione", {"O": 3}), ("ossidazione_carbonile", {"H": -2, "O": 1}),
          ("deidrogenazione", {"H": -2}), ("idratazione", {"H": 2, "O": 1}), ("dealchilazione", {"C": -1, "H": -2}), ("dealchilazione", {"C": -2, "H": -4}),
          ("dealchilazione", {"C": -3, "H": -6}), ("dealchilazione", {"C": -4, "H": -8}), ("deamminazione_ossidativa", {"N": -1, "H": -3, "O": 1}),
          ("apertura_anello", {"C": -1, "O": 1}), ("apertura_anello", {"C": -2, "H": -2}), ("apertura_anello", {"C": -2, "H": -4, "O": -1}),
          ("apertura_anello", {"C": -4, "H": -6, "O": -1}), ("decarbossilazione", {"C": -1, "O": -2}), ("decarbossilazione", {"C": -1, "O": -1}),
          ("desolfonazione", {"S": -1, "O": -2}), ("desolfonazione", {"S": -1, "O": -3}), ("n_ossidazione", {"N": 1, "O": 1, "H": -1}),
          ("denitrazione", {"N": -1, "O": -2, "H": 1}), ("denitrazione", {"N": -1, "O": -1, "H": 1})]
SCISSIONI = [("scissione", {"C": -5, "H": -8, "O": -1}), ("scissione", {"C": -6, "H": -10, "N": -1}), ("scissione", {"C": -4, "H": -4, "N": -1, "O": -1}),
             ("scissione", {"C": -7, "H": -10, "O": -2}), ("scissione", {"C": -5, "H": -6, "N": -2})]
K_DELTA = E.ADDUCT_SHIFT["[M+K]+"] - E.ADDUCT_SHIFT["[M+H]+"]
ORDINE = "CHNOS"
DEFAULT = ESTESO


def chiave(d: dict) -> tuple:
    return tuple(sorted((e, n) for e, n in d.items() if n))


def testo_delta(d: dict) -> str:
    """'+O', '-C4H8', '-H2+O': losses first, then gains, elements in the order C H N O S then alphabetical."""
    def parte(segno):
        els = sorted((e for e, n in d.items() if n * segno > 0), key=lambda e: (ORDINE.find(e) if e in ORDINE else 9, e))
        return "".join(e + (str(abs(d[e])) if abs(d[e]) != 1 else "") for e in els)
    neg, pos = parte(-1), parte(1)
    return ("-" + neg if neg else "") + ("+" + pos if pos else "")


UN_PASSO = {chiave(d) for _, d in REGOLE}
DUE_PASSI = UN_PASSO | {chiave(somma(a, b)) for _, a in REGOLE for _, b in REGOLE}


def flusso(c, etichetta: str):
    """Random stream of a module: the seed plus a fixed label (never the stream of V1)."""
    return np.random.default_rng([c.seme, zlib.crc32(("v1x/" + etichetta).encode())])


class Stato:
    """What the extension adds to the model, for the truth: side data of the ions by id and the inherited-ion overrides."""
    def __init__(self, P: dict):
        self.P = P
        self.meta: dict[str, dict] = {}
        self.ere: dict[str, dict] = {}
        self.cont: Counter = Counter()
        self.catene: list[list[str]] = []
        self.neutre: dict[str, dict] = {}
        self.tot: dict[str, dict] = {}
        self.albero: dict[str, tuple] = {}


# ---------------------------------------------------------------------------------------------------------------- kinetics of a chain
def conc_catena(t, ks) -> np.ndarray:
    """Amount of the last species of a chain of first-order steps with rates ks (the last one is its own loss), unnormalised."""
    t = np.asarray(t, float)
    n = len(ks) - 1
    s = np.zeros_like(t)
    for i in range(n + 1):
        s += np.exp(-ks[i] * t) / np.prod([ks[j] - ks[i] for j in range(n + 1) if j != i])
    return float(np.prod(ks[:n])) * s


GRIGLIA = np.linspace(0, 1500, 6001)


def picco(ks) -> tuple[float, float]:
    """(time of the maximum in min, height) of the species of a chain."""
    y = conc_catena(GRIGLIA, ks)
    j = int(np.argmax(y))
    return float(GRIGLIA[j]), float(y[j])


def estendi_tassi(rng, ks, e, lim, gap, tm_prec):
    for _ in range(300):
        k = round(float(rng.uniform(0.05, 0.2)) * e, 3)
        if min(abs(k - x) for x in ks) < 0.01:
            continue
        t, _ = picco(ks + [k])
        if tm_prec + gap <= t <= lim:
            return k, t
    return None


def tassi(rng, k0: float, n: int, P: dict):
    """Rates [k0 (decay of the progenitor), k1..kn] whose maxima are increasing, at least `gap_tmax` apart and inside the series."""
    for _ in range(200):
        ks, tm = [k0], [0.0]
        for _ in range(n):
            r = estendi_tassi(rng, ks, P["escala_k"], P["t_limite"], P["gap_tmax"], tm[-1])
            if r is None:
                break
            ks.append(r[0]); tm.append(r[1])
        else:
            return ks, tm[1:]
    raise RuntimeError("no chain rates found")


def ampiezza(c, ks, scala: float, bersaglio: float | None = None) -> np.ndarray:
    """Per-file amount of the last species of a chain: 0 in the blank, the curve (peak = 1) at the time of the file elsewhere. With `bersaglio` the scale is such that
    the highest file reaches that height."""
    _, h = picco(ks)
    y = np.array([0.0 if k == "blank" else float(conc_catena(t, ks)) / h for t, k in zip(c.tempi, c.tipi)])
    return (bersaglio / y.max() if bersaglio else scala) * y


# ---------------------------------------------------------------------------------------------------------------- M1: chains
def crea_tp(c, st, rng, pred: Ione, npred: dict, scelta, ks, g: int, ruolo: str, **kw) -> tuple[Ione, dict]:
    """A product of `pred` with the change `scelta` = (class, delta, route); `ks` are the chain rates up to this product. Returns the ion and its neutral formula."""
    classe, delta, via = scelta
    f = somma(npred, delta)
    mid = c.id_mol()
    t, _ = picco(ks)
    sg = float(rng.uniform(0.045, 0.07))
    scala = float(rng.uniform(0.8, 3.0)) * 1e7
    o = c.ione(mid, ruolo, mid, f, kw.pop("rt"), sg, ampiezza(c, ks, scala, kw.pop("bersaglio", None)), progenitore=pred.id, generazione=g, forma="bateman" if g == 1 else "ritardo",
               k_form=ks[-2], k_dec=ks[-1], selezionabile=True)
    o.frammenti = frammenti_di(rng, o.mz)
    c.formule.append(f)
    st.neutre[o.id] = f
    st.meta[o.id] = dict(classe=classe, delta=testo_delta(delta), via=via, t_max=round(t, 2), **kw)
    return o, f


def scegli_delta(c, rng, npred: dict, totale: dict, g: int, rt: float, forzato=None):
    """A rule (or the given cleavage) whose product is a valid, new, isolated formula and whose total change from the progenitor is not one rule of the table
    (second generation and beyond) nor two (third generation): the observed network is needed to reach it."""
    for _ in range(2000):
        classe, delta = forzato if forzato else REGOLE[int(rng.integers(0, len(REGOLE)))]
        f = somma(npred, delta)
        tot = chiave(somma(totale, delta))
        if not valida_formula(f) or any(f == x for x in c.formule) or not c.libero(f, rt):
            continue
        if (g >= 2 and tot in UN_PASSO) or (g >= 3 and tot in DUE_PASSI):
            continue
        return (classe, delta, "scissione" if classe == "scissione" else "regola")
    raise RuntimeError("no rule found")


def passo(c, st, rng, pred, npred, tot, ks, g, forzato=None, debole=False, **kw):
    """One step of a chain: the product of `pred`; returns (ion, neutral, total change from the progenitor)."""
    rt = c.slot()
    sc = scegli_delta(c, rng, npred, tot, g, rt, forzato)
    tot = somma(tot, sc[1])
    o, f = crea_tp(c, st, rng, pred, npred, sc, ks, g, f"tp{g}", rt=rt, bersaglio=float(rng.uniform(3, 10)) * 6e5 if debole else None, debole=debole,
                   raggiungibile=chiave(tot) in DUE_PASSI, delta_totale=testo_delta(tot), **kw)
    st.tot[o.id] = tot
    return o, f, tot


def nodo(c, st, rng, padre: Ione, npadre: dict, g: int, n: int, forzato=None, debole=False):
    """A product of generation `g` of the tree of chains below `padre` (a new rate extends the rates of `padre`). Returns (ion, neutral) or None if no rate fits."""
    ks, tm = st.albero.get(padre.id, ([padre.k_dec], 0.0))
    r = estendi_tassi(rng, ks, st.P["escala_k"], st.P["t_limite"], st.P["gap_tmax"], tm)
    if r is None:
        return None
    o, f, _ = passo(c, st, rng, padre, npadre, st.tot.get(padre.id, {}), ks + [r[0]], g, forzato, debole, catena=n)
    st.albero[o.id] = (ks + [r[0]], r[1])
    return o, f


def catene(c, st, rng, padre, npadre) -> None:
    """Chains share their first products (a tree: the number of different changes from the progenitor is small): `n3` paths of three products and `n2` of two. Path 0 is the
    family (a cleavage, then +O, -H2 and +2O of it, the +O carries the third generation), path 1 ends in a trace."""
    n3, n2 = (int(st.P["catene"][k]) for k in ("3", "2"))
    pool = []
    for i in range(max(1, -(-(n3 + n2) // 3))):
        r = nodo(c, st, rng, padre, npadre, 1, i, SCISSIONI[int(rng.integers(0, len(SCISSIONI)))] if i == 0 else None)
        if r:
            pool.append(r)
    for i in range(n3 + n2):
        lung = 3 if i < n3 else 2
        a, fa = pool[i % len(pool)]
        r2 = nodo(c, st, rng, a, fa, 2, i, ("idrossilazione", {"O": 1}) if i == 0 else None)
        r3 = nodo(c, st, rng, *r2, 3, i, debole=i == 1) if r2 and lung == 3 else None
        st.catene.append([o.id for o in (a, r2 and r2[0], r3 and r3[0]) if o])
        if r2 and (lung == 2 or r3):
            st.cont["catene3" if lung == 3 else "catene2"] += 1
        if i == 0:
            for dl in (("deidrogenazione", {"H": -2}), ("idrossilazione", {"O": 2})):
                nodo(c, st, rng, a, fa, 2, i, dl)


# ---------------------------------------------------------------------------------------------------------------- inherited ions
def aggiungi_frammenti(rng, o: Ione, extra) -> None:
    """Adds fragments (m/z, relative intensity) to the MS2 of an ion."""
    if o.frammenti is None:
        o.frammenti = frammenti_di(rng, o.mz)
    f = np.concatenate([o.frammenti[0], [x for x, _ in extra]])
    r = np.concatenate([o.frammenti[1], [y for _, y in extra]])
    q = np.argsort(f, kind="stable")
    o.frammenti = (f[q], r[q])


def libero_mz(c, mz: float, rt: float, famiglia: str) -> bool:
    """No ion of another family within 1.3 m/z of `mz` eluting within 0.6 min (no unplanned co-isolation)."""
    return all(abs(o.mz - mz) > 1.3 or abs(o.rt - rt) > 0.6 for o in c.ioni if o.amp is not None and o.famiglia != famiglia and o.mz)


def crea_isf(c, st, rng, base: Ione, rapporto: float, rt: float, nome: str, perdite=LOSSES) -> Ione | None:
    """An in-source fragment of `base` (loss from the list), constant ratio to its height, in the MS2 of its parent. None if no isolated candidate is found."""
    fam = [o for o in c.ioni if o.famiglia == base.famiglia and o.ruolo.startswith("isf")]
    for _ in range(300):
        f = somma(E.parse_formula(base.ion), E.parse_formula(perdite[int(rng.integers(0, len(perdite)))]), -1)
        if not valida_formula(f):
            continue
        ion = E.fmt(f)
        picchi, mz, _ = modello(ion)
        if all(abs(mz[0] - o.mz) > 0.6 for o in fam) and libero_mz(c, float(mz[0]), rt, base.famiglia):
            o = Ione(id=nome, ruolo="isf", famiglia=base.famiglia, rt=rt, sg=base.sg, progenitore=base.id, forma="come_progenitore", isf_in_ms2=1, amp=base.amp * rapporto,
                     note=f"ratio={rapporto}", ion=ion, picchi=picchi, mz=float(mz[0]))
            c.ioni.append(o)
            aggiungi_frammenti(rng, base, [(o.mz, float(rng.uniform(0.5, 0.9)))])
            return o
    return None


def crea_addotto(c, rng, base: Ione, nome: str, rt: float, delta_mz: float, agg: dict, rapporto: float, ruolo="addotto") -> Ione:
    f = somma(E.parse_formula(base.ion), agg)
    ion = E.fmt(f)
    picchi, mz, _ = modello(ion)
    o = Ione(id=f"{base.id}_{nome}", ruolo=ruolo, famiglia=base.famiglia, rt=rt, sg=base.sg, progenitore=base.id, forma="come_progenitore", amp=base.amp * rapporto,
             note=f"ratio={rapporto};delta_mz={delta_mz:.5f}", ion=ion, picchi=picchi, mz=float(mz[0]))
    c.ioni.append(o)
    return o


def eleggibili(c) -> list[Ione]:
    """Products that can carry an in-source fragment: not isomers, not chimeric targets, none yet."""
    con = {o.progenitore for o in c.ioni if o.ruolo in ("isf", "isf_finto")}
    return [o for o in c.ioni if o.ruolo in ("tp1", "tp2", "tp3") and o.amp is not None and o.selezionabile and o.purezza is None and o.id not in con]


def padre_esteso(c, st, rng, padre: Ione) -> None:
    """Cascade of three in-source fragments, K adduct and dimer of the progenitor, as inherited ions."""
    base = padre
    for k, (perd, r) in enumerate([("C4H8", (0.1, 0.25)), ("H2O", (0.05, 0.15)), ("C2H4", (0.03, 0.1))], 1):
        f = somma(E.parse_formula(base.ion), E.parse_formula(perd), -1)
        if not valida_formula(f):
            continue
        ion = E.fmt(f)
        picchi, mz, _ = modello(ion)
        rr = round(float(rng.uniform(*r)), 3)
        o = Ione(id=f"{padre.id}_CISF{k}", ruolo="isf", famiglia=padre.famiglia, rt=padre.rt + float(rng.uniform(-0.004, 0.004)), sg=padre.sg, progenitore=padre.id,
                 forma="come_progenitore", isf_in_ms2=1, amp=padre.amp * rr, note=f"ratio={rr};cascata={k}", ion=ion, picchi=picchi, mz=float(mz[0]))
        c.ioni.append(o)
        aggiungi_frammenti(rng, padre, [(o.mz, float(rng.uniform(0.5, 0.9)))])
        st.ere[o.id] = dict(cascata=k, da=base.id)
        st.cont["cascata"] += 1
        base = o
    crea_addotto(c, rng, padre, "K", padre.rt + float(rng.uniform(-0.008, 0.008)), K_DELTA, {"H": -1, "K": 1}, round(float(rng.uniform(0.02, 0.06)), 3))
    ion2 = E.fmt(somma({e: 2 * n for e, n in E.parse_formula(padre.ion).items()}, {"H": -1}))
    picchi, mz, _ = modello(ion2)
    c.ioni.append(Ione(id=f"{padre.id}_DIM", ruolo="dimero", famiglia=padre.famiglia, rt=padre.rt, sg=padre.sg, progenitore=padre.id, forma="come_progenitore",
                       amp=padre.amp * 0.04, note="ratio=0.04;tipo=[2M+H]+", ion=ion2, picchi=picchi, mz=float(mz[0])))
    st.cont["dimeri"] += 1


# ---------------------------------------------------------------------------------------------------------------- M2: in-source fragments of the products
def tp_coeluito(c, st, rng, padre: Ione, b: Ione, n: int) -> bool:
    """A product with the formula of the in-source fragment of `b`, same RT, own kinetics: the area ratio to `b` is not constant."""
    rt = b.rt + float(rng.uniform(-0.01, 0.01))
    r = round(float(rng.uniform(0.06, 0.25)), 3)
    for _ in range(300):
        f = somma(E.parse_formula(b.ion), E.parse_formula(LOSSES[int(rng.integers(0, len(LOSSES)))]), -1)
        neutra = somma(f, {"H": -1})
        if not valida_formula(f) or not valida_formula(neutra) or any(neutra == x for x in c.formule) or not libero_mz(c, modello(E.fmt(f))[1][0], rt, b.famiglia):
            continue
        forma = "monotona" if b.forma in ("bateman", "ritardo") else "bateman"
        p = dict(kf=round(float(rng.uniform(0.05, 0.25)) * st.P["escala_k"], 3)) if forma == "monotona" else dict(kf=round(float(rng.uniform(0.15, 0.3)) * st.P["escala_k"], 3),
                                                                                                                      kd=round(float(rng.uniform(0.04, 0.1)) * st.P["escala_k"], 3))
        mid = c.id_mol()
        amp = c.traccia(forma, float(b.amp.max()) * r * float(rng.uniform(0.6, 2.5)), **p) + b.amp * r          # the signal of the ion is the sum of the two
        t = c.ione(mid, "tp1", mid, neutra, rt, float(rng.uniform(0.045, 0.07)), amp, progenitore=padre.id, generazione=1, forma=forma, k_form=p["kf"], k_dec=p.get("kd"), selezionabile=True)
        t.frammenti = frammenti_di(rng, t.mz)
        c.formule.append(neutra)
        v = Ione(id=f"{b.id}_ISFV{n}", ruolo="isf", famiglia=b.famiglia, rt=rt, sg=t.sg, progenitore=b.id, forma="isf_variabile", isf_in_ms2=1, note=f"ratio={r};coeluisce_con={t.id}",
                 ion=t.ion, mz=t.mz)
        c.ioni.append(v)
        aggiungi_frammenti(rng, b, [(t.mz, float(rng.uniform(0.5, 0.9)))])
        st.meta[t.id] = dict(classe=None, delta=None, via="scissione", t_max=None, isf_di=b.id, raggiungibile=None)
        st.ere[v.id] = dict(rapporto_costante=False, coeluisce_con_tp=t.id)
        st.cont["isf_variabili"] += 1
        return True
    return False


def isf_dei_tp(c, st, rng, padre: Ione) -> None:
    cand = eleggibili(c)
    scelti = [cand[i] for i in sorted(rng.permutation(len(cand))[:int(round(st.P["f_isf_tp"] * len(cand)))].tolist())] if cand else []
    liberi = [o for o in cand if o.forma in ("bateman", "monotona", "ritardo")]
    for n, i in enumerate(sorted(rng.permutation(len(liberi))[:st.P["n_isf_variabile"]].tolist()), 1):
        if tp_coeluito(c, st, rng, padre, liberi[i], n):
            scelti = [o for o in scelti if o.id != liberi[i].id]
    for o in scelti:
        r = round(float(rng.uniform(0.03, 0.3)), 3)
        if crea_isf(c, st, rng, o, r, o.rt + float(rng.uniform(-0.01, 0.01)), f"{o.id}_ISF1"):
            st.cont["isf_costanti"] += 1


# ---------------------------------------------------------------------------------------------------------------- M5: inherited ions that look like products
def travestiti(c, st, rng) -> None:
    cand = eleggibili(c)
    for i, j in enumerate(sorted(rng.permutation(len(cand))[:st.P["n_travestiti"]].tolist())):
        o = cand[j]
        d = 0.0 if i % 2 == 0 else 0.02
        r = round(float(rng.uniform(0.04, 0.2)), 3)
        isf = crea_isf(c, st, rng, o, r, o.rt + d, f"{o.id}_ISFT")
        na = crea_addotto(c, rng, o, "NaT", o.rt + d, 21.98194, {"H": -1, "Na": 1}, round(float(rng.uniform(0.03, 0.12)), 3))
        for k in (1, 2):
            _, mz, rel = modello(o.ion)
            c.ioni.append(Ione(id=f"{o.id}_M{k}", ruolo="isotopo", famiglia=o.famiglia, rt=o.rt, sg=o.sg, progenitore=o.id, ion=o.ion, mz=float(mz[k]), forma="come_progenitore", note=f"rel={rel[k]:.4f}"))
            st.ere[f"{o.id}_M{k}"] = dict(travestito=True, delta_rt=0.0)
        for x in (isf, na):
            if x is not None:
                st.ere[x.id] = dict(travestito=True, delta_rt=d)
        st.cont["travestiti"] += 1


def estendi(c) -> None:
    """Runs the modules in a fixed order; the state goes to `c.vx`."""
    st = c.vx = Stato({**DEFAULT, **c.P["esteso"]})
    padre = next(o for o in c.ioni if o.ruolo == "progenitore")
    npadre = c.formule[0]
    st.neutre = {padre.id: npadre}
    from valida import sintetici_esteso_chimica as X                      # halogens and close isomers (the order of the calls fixes the model)
    catene(c, st, flusso(c, "m1"), padre, npadre)
    X.alogenati(c, st, flusso(c, "m3"))
    X.isomeri_vicini(c, st, flusso(c, "m4"))
    r2 = flusso(c, "m2")
    padre_esteso(c, st, r2, padre)
    isf_dei_tp(c, st, r2, padre)
    travestiti(c, st, flusso(c, "m5"))
