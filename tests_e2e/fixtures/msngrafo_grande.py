"""Builds tests_e2e/fixtures/msngrafo_grande.json: a synthetic msngrafo/1 of 80 ions in 5 levels (no real data, no real compound) for the layout tests of the
«Fragmentation graph» (0 overlapping boxes, fewer arc crossings than the barycentre layout). The graph has no `vista`; add it with the engine:
    python3 tests_e2e/fixtures/msngrafo_grande.py > /tmp/g.json && mzlab-cli msn-graph complete /tmp/g.json --out tests_e2e/fixtures/msngrafo_grande.json
The rows `pos` are the barycentre layout of the engine (two passes over the columns, ions by decreasing m/z at the start), the baseline of the crossings."""
import json, random

R = random.Random(20261022)
SIZES = [1, 12, 24, 28, 15]
LOSSES = [(18.01056, "H2O"), (17.02655, "NH3"), (27.99491, "CO"), (43.98983, "CO2"), (28.0313, "C2H4"), (44.02621, "C2H4O"), (56.0626, "C4H8"), (60.02113, "C2H4O2")]
H = 1.007825

def formula(mz, i):
    c = 8 + i % 9
    return f"C{c}H{c * 2 - 3 + i % 4}N{1 + i % 3}O{1 + i % 4}"

levels, nodes, arcs, nid = [], [], [], 0
mz0 = 420.2
for lv, size in enumerate(SIZES, 1):
    cur = []
    for k in range(size):
        mz = mz0 if lv == 1 else round(mz0 - 20 * (lv - 1) - R.uniform(8, 160) * (lv - 0.5) ** 0.5 + k * 0.013, 5)
        n = dict(id=f"n{nid}", mz=round(mz, 5), dev_ppm=round(R.uniform(0, 3), 2), livello=lv, isolato=lv < 4 and k % 3 == 0, albero=[], osservazioni=1 + k % 3,
                 rel_max=100 if lv == 1 else round(R.uniform(3, 100), 1), isotopo_di=None, m1=None, isolamento=dict(larghezza=1, a_monte=None, stretto=False), non_spiegati=None, pos={"col": lv - 1, "riga": 0})
        if lv > 1 or True:
            cs = []
            for j in range(1 + (k % 3 == 1)):
                cs.append(dict(formula=formula(mz, nid + j * 5), mz_teorico=round(mz + 0.0004 * (j + 1), 5), ppm=round(-4.5 + 2.3 * j + (k % 5) * 0.7, 2), dbe=3 + k % 6, elettroni="pari" if k % 2 else "dispari", finestra=10, prove=["perdita"] if j == 0 and k % 2 == 0 else []))
            n["formula"] = dict(candidati=cs, scelta=cs[0]["formula"] if len(cs) == 1 else None, fonte="unica" if len(cs) == 1 else None)
        else:
            n["formula"] = None
        cur.append(n); nid += 1
    levels.append(cur); nodes += cur
for lv in range(1, len(levels)):
    for n in levels[lv]:
        ps = [R.choice(levels[lv - 1])]
        if R.random() < 0.22:
            q = R.choice(levels[R.randrange(0, lv)])
            if q is not ps[0]: ps.append(q)
        for p in ps:
            d = round(p["mz"] - n["mz"], 5)
            known = min(LOSSES, key=lambda x: abs(x[0] - d))
            hit = abs(known[0] - d) < 0.01
            arcs.append(dict(da=p["id"], a=n["id"], perdita=dict(da_massa=d, formula=known[1] if hit and R.random() < 0.8 else None, ppm=None, compatibile=[known[1]] if not hit and R.random() < 0.3 else []),
                             isolamento=bool(n["isolato"]), misure=[dict(albero=f"{p['id']}@cid{25 + 10 * m}", livello=lv + 1, att="cid", ce=25 + 10 * m, n_scansioni=5 + m, rel=round(R.uniform(2, 100), 1)) for m in range(1 + (n["osservazioni"] % 2))]))
arcs.sort(key=lambda a: (int(a["da"][1:]), int(a["a"][1:])))
# barycentre rows: by decreasing m/z, then left-to-right and back
rows = {n["id"]: i for lv in levels for i, n in enumerate(sorted(lv, key=lambda x: -x["mz"]))}
par = {n["id"]: [a["da"] for a in arcs if a["a"] == n["id"]] for n in nodes}
chi = {n["id"]: [a["a"] for a in arcs if a["da"] == n["id"]] for n in nodes}
def sweep(order, nb):
    for lv in order:
        key = lambda n: (sum(rows[x] for x in nb[n["id"]]) / len(nb[n["id"]]) if nb[n["id"]] else rows[n["id"]], -n["mz"])
        for i, n in enumerate(sorted(levels[lv], key=key)): rows[n["id"]] = i
sweep(range(1, len(levels)), par); sweep(range(len(levels) - 2, -1, -1), chi)
for n in nodes: n["pos"]["riga"] = rows[n["id"]]
g = dict(formato="msngrafo/1", motore="1.0.1", sorgente=dict(nome="msn_big.mzML", sha256_idx="0" * 64),
         opzioni=dict(ppm=5, da_min=0.002, min_rel=2, max_picchi=120, isotopi=False, attivazione=None, formula="C20H30N5O6+", smiles=None, ppm_formula=10, mz_basso=150, ppm_basso=20, prove_min=1, scelte=[]),
         polarita=1, radici=["n0"], ione=dict(formula="C20H30N5O6", mz_teorico=420.2, mz_osservato=420.2, ppm=0.0),
         conteggi=dict(nodi=len(nodes), archi=len(arcs), scartati_pesanti=0, scartati_isotopi=0, scartati_deboli=0, scartati_formula=0), avvisi=[], nodi=nodes, archi=arcs, spettri=[])
print(json.dumps(g, ensure_ascii=False))
