"""Independent reference of the isotope-finder tests: synthetic spectra with an exact binomial 13C envelope (numpy), written to
tests/data/untargeted/isofinder/casi.json. For each case it also gives the cosine between the observed M, M+1, M+2 ... heights and the binomial
envelope that generated them (1 by construction) and the cosine against a Poisson envelope of the same mean, which the Rust finder is compared to.
Run: python3 tools/isofinder_riferimento.py"""
import json
import math
import pathlib

import numpy as np

C13 = 1.003354831
P13 = 0.0107


def binom_env(n_c, kmax=6):
    k = np.arange(kmax + 1)
    v = np.array([math.comb(n_c, int(i)) * P13**i * (1 - P13) ** (n_c - i) for i in k])
    return v / v[0]


def poisson_env(n_c, kmax=6):
    lam = n_c * P13 / (1 - P13)
    v = np.array([lam**i / math.factorial(i) for i in range(kmax + 1)])
    return v / v[0]


def cosine(a, b):
    return float(np.dot(a, b) / (np.linalg.norm(a) * np.linalg.norm(b)))


def case(name, mz0, z, n_c, base=1e6, drop=(), noise=()):
    env = binom_env(n_c)
    step = C13 / z
    peaks = [(mz0 + i * step, float(base * env[i])) for i in range(len(env)) if env[i] * base > 1e3 and i not in drop]
    peaks += [tuple(p) for p in noise]
    peaks.sort()
    obs = np.array([p[1] for p in peaks[: len(env)]]) if not noise else np.array([base * env[i] for i in range(len(env)) if i not in drop and env[i] * base > 1e3])
    k = len(obs)
    return {
        "name": name, "mz0": mz0, "z": z, "n_c": n_c, "mz": [p[0] for p in peaks], "y": [p[1] for p in peaks],
        "cos_binomial": cosine(obs, env[:k]) if not drop else None, "cos_poisson": cosine(obs, poisson_env(n_c)[:k]) if not drop else None,
    }


ISO = {
    "H": [(1.00782503207, 0.999885), (2.0141017778, 0.000115)],
    "C": [(12.0, 0.9893), (13.0033548378, 0.0107)],
    "N": [(14.0030740048, 0.99636), (15.0001088982, 0.00364)],
    "O": [(15.99491461956, 0.99757), (16.99913170, 0.00038), (17.9991610, 0.00205)],
    "S": [(31.97207100, 0.9499), (32.97145876, 0.0075), (33.96786690, 0.0425)],
    "Si": [(27.9769265325, 0.92223), (28.976494700, 0.04685), (29.97377017, 0.03092)],
    "Cl": [(34.96885268, 0.7576), (36.96590259, 0.2424)],
    "Br": [(78.9183371, 0.5069), (80.9162906, 0.4931)],
}
PROTON = 1.007276466


def fine_pattern(formula, floor=1e-4):
    """[M+H]+ pattern of a formula by exact convolution of the isotopes of every atom, merged per nominal mass (weighted mean m/z)."""
    dist = {0: (0.0, 1.0)}  # nominal offset -> (sum of mass x abundance, abundance)
    mono = 0.0
    for el, n in formula.items():
        iso = ISO[el]
        base = iso[0]
        mono += n * base[0]
        for _ in range(n):
            new = {}
            for off, (mw, ab) in dist.items():
                for m, a in iso:
                    k = off + round(m - base[0])
                    cur = new.get(k, (0.0, 0.0))
                    new[k] = (cur[0] + (mw + ab * (m - base[0])) * a, cur[1] + ab * a)
            dist = {k: v for k, v in new.items() if v[1] > 1e-9}
    out = []
    top = max(v[1] for v in dist.values())
    for k in sorted(dist):
        mw, ab = dist[k]
        if ab / top >= floor:
            out.append((mono + PROTON + (mw / ab if ab else 0.0), ab / top))
    return out


def element_case(name, formula, expect):
    pat = fine_pattern(formula)
    return {"name": name, "formula": formula, "mz": [p[0] for p in pat], "y": [p[1] * 1e6 for p in pat], "expect": expect}


cases = [
    case("c10_z1", 151.0, 1, 10),
    case("c20_z1", 301.0, 1, 20),
    case("c40_z2", 501.25, 2, 40),
    case("no_m1", 200.0, 1, 12, drop=(1,)),
]
elements = [
    element_case("cl2", {"C": 15, "H": 12, "Cl": 2, "O": 1}, ["Cl"]),
    element_case("br1", {"C": 12, "H": 9, "Br": 1, "O": 1}, ["Br"]),
    element_case("s2", {"C": 10, "H": 8, "S": 2, "O": 2}, ["S"]),
    element_case("chno", {"C": 20, "H": 25, "N": 1, "O": 3}, []),
    element_case("si1", {"C": 14, "H": 24, "O": 1, "Si": 1}, ["Si"]),
]
out = pathlib.Path(__file__).resolve().parent.parent / "tests/data/untargeted/isofinder/elementi.json"
out.write_text(json.dumps(elements, separators=(",", ":")))
print(out, len(elements))
out = pathlib.Path(__file__).resolve().parent.parent / "tests/data/untargeted/isofinder/casi.json"
out.write_text(json.dumps(cases, separators=(",", ":")))
print(out, len(cases))
