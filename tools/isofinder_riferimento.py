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


cases = [
    case("c10_z1", 151.0, 1, 10),
    case("c20_z1", 301.0, 1, 20),
    case("c40_z2", 501.25, 2, 40),
    case("no_m1", 200.0, 1, 12, drop=(1,)),
]
out = pathlib.Path(__file__).resolve().parent.parent / "tests/data/untargeted/isofinder/casi.json"
out.write_text(json.dumps(cases, separators=(",", ":")))
print(out, len(cases))
