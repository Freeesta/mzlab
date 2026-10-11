"""Reference answers of the small statistics of mzlab/ionfamily.py for crates/mzlab-core/tests/stats_parity.rs.

    python3 tools/stats_riferimento.py OUT.json

Seeded synthetic vectors (ties included); NaN is written as null. No compound names.
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from mzlab.ionfamily import chi2_sf, kendall_tau, perm_pvalue, rankdata, theil_sen, wpearson, wspearman  # noqa: E402


def f(v):
    v = float(v)
    return None if v != v else v


def main() -> None:
    rng = np.random.default_rng(11)
    cases = []
    for k in range(120):
        n = int(rng.choice([3, 4, 5, 6, 7, 8, 9, 12, 20, 40, 100, 130]))
        x = np.round(rng.normal(size=n) * 5, int(rng.choice([0, 1, 3])))
        y = np.round(0.6 * x + rng.normal(size=n), int(rng.choice([0, 1, 3]))) if k % 5 else np.zeros(n)
        w = rng.uniform(0.1, 3, n)
        sl, ic = theil_sen(x, y)
        row = {"x": x.tolist(), "y": y.tolist(), "w": w.tolist(), "rank": rankdata(x).tolist(), "pearson": f(wpearson(x, y)), "pearson_w": f(wpearson(x, y, w)),
               "spearman": f(wspearman(x, y)), "spearman_w": f(wspearman(x, y, w)), "kendall": f(kendall_tau(x, y)),
               "ts": [f(sl), f(ic)], "ts30": [f(v) for v in theil_sen(x, y, 30)]}
        if n >= 4 and (n <= 20 or k % 10 == 0):
            row["perm"] = f(perm_pvalue(wspearman, x, y))
            row["perm_k"] = f(perm_pvalue(kendall_tau, x, y, 500, 3))
        cases.append(row)
    chi = [[float(x), int(d), float(chi2_sf(x, d))] for x in (0.0, 0.01, 0.5, 1, 3.3, 7, 15, 40, 120, 400) for d in (1, 2, 3, 5, 9, 30, 100)]
    Path(sys.argv[1]).write_text(json.dumps({"cases": cases, "chi": chi}), encoding="utf-8")
    print(len(cases))


if __name__ == "__main__":
    main()
