"""Reference of the Kendrick mass defect of `crates/mzlab-core/src/kendrick.rs` (the code of the site is Rust; this one only writes the frozen answers).

    python3 tools/kmd_riferimento.py            # writes tests/golden/kmd.json
    python3 tools/kmd_riferimento.py --check    # exits 1 when the file differs

The answers are for the cases of `tests/test_kmd.py` (a PFCA series with two unrelated ions, a series with a missing
member, a short series, another unit) and one list with ions without RT.
"""
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGET = ROOT / "tests" / "golden" / "kmd.json"
PFCA = [212.97918, 262.97599, 312.97279, 362.96959, 412.96640, 462.96320, 512.96001]
RT = [3.1, 4.6, 5.9, 7.0, 8.0, 8.9, 9.7]
CASES = {
    "series": ([{"id": i, "mz": m, "rt": r} for i, (m, r) in enumerate(zip(PFCA, RT))] + [{"id": 99, "mz": 301.1234, "rt": 6.0}, {"id": 98, "mz": 455.3141, "rt": 2.0}], {"unit": "CF2", "tolMDa": 3, "minMembers": 3}),
    "gap": ([{"id": i, "mz": m, "rt": 1 + i} for i, m in enumerate([PFCA[0], PFCA[1], PFCA[3], PFCA[4]])], {"minMembers": 3}),
    "short": ([{"id": i, "mz": m} for i, m in enumerate(PFCA[:2])], {"minMembers": 3}),
    "ch2": ([{"id": i, "mz": 100.0 + 14.01565006 * i + 0.0004 * (i % 2), "rt": 2.0 + i} for i in range(6)] + [{"id": 50, "mz": 413.0}], {"unit": "CH2", "tolMDa": 2, "minMembers": 4}),
}

M = {"C": 12.0, "H": 1.00782503223, "F": 18.99840316273, "O": 15.99491461957}


def js_round(x: float) -> int:
    return math.floor(x + 0.5)


def make_unit(uid: str, parts: dict[str, int]):
    mass = sum(M[e] * n for e, n in parts.items())
    return {"id": uid, "mass": mass, "nominal": js_round(mass)}


UNITS = {
    "CF2": make_unit("CF2", {"C": 1, "F": 2}),
    "CF2O": make_unit("CF2O", {"C": 1, "F": 2, "O": 1}),
    "C2F4": make_unit("C2F4", {"C": 2, "F": 4}),
    "CH2": make_unit("CH2", {"C": 1, "H": 2}),
}


def kendrick(mz: float, u: dict):
    km = mz * u["nominal"] / u["mass"]
    nkm = js_round(km)
    return km, nkm, nkm - km


def ranks(v: list[float]) -> list[float]:
    idx = sorted(range(len(v)), key=lambda i: v[i])
    r = [0.0] * len(v)
    i = 0
    while i < len(idx):
        j = i
        while j + 1 < len(idx) and v[idx[j + 1]] == v[idx[i]]:
            j += 1
        avg_rank = (i + j) / 2.0 + 1.0
        for k in range(i, j + 1):
            r[idx[k]] = avg_rank
        i = j + 1
    return r


def spearman(x: list[float], y: list[float]):
    if len(x) < 3 or len(x) != len(y):
        return None
    a = ranks(x)
    b = ranks(y)
    n = len(a)
    ma = sum(a) / n
    mb = sum(b) / n
    sab = sum((a[i] - ma) * (b[i] - mb) for i in range(n))
    saa = sum((a[i] - ma) ** 2 for i in range(n))
    sbb = sum((b[i] - mb) ** 2 for i in range(n))
    if saa > 0 and sbb > 0:
        return sab / math.sqrt(saa * sbb)
    return None


def analyse(features, opts):
    u = UNITS.get(opts.get("unit", "CF2"), UNITS["CF2"])
    tol = max(0.0, float(opts.get("tolMDa", 3.0))) / 1000.0
    min_members = int(opts.get("minMembers", 3))

    rows = []
    for i, f in enumerate(features):
        mz = float(f["mz"])
        if math.isfinite(mz) and mz > 0:
            rt = None if f.get("rt") is None else float(f["rt"])
            km, nkm, kmd = kendrick(mz, u)
            rows.append({"i": i, "id": f.get("id"), "mz": mz, "rt": rt, "km": km, "nkm": nkm, "kmd": kmd, "series": None})

    order = sorted(range(len(rows)), key=lambda k: rows[k]["kmd"])
    series = []
    s = 0
    while s < len(order):
        e = s
        while e + 1 < len(order) and rows[order[e + 1]]["kmd"] - rows[order[s]]["kmd"] <= 2 * tol:
            e += 1
        grp = [order[k] for k in range(s, e + 1)]
        distinct_nkm = set(rows[k]["nkm"] for k in grp)
        if len(distinct_nkm) >= min_members:
            mem = sorted(grp, key=lambda k: (rows[k]["nkm"], rows[k]["kmd"]))
            kmds = [rows[k]["kmd"] for k in grp]
            steps = []
            for k in range(1, len(mem)):
                if rows[mem[k]]["nkm"] != rows[mem[k - 1]]["nkm"]:
                    steps.append(js_round((rows[mem[k]]["nkm"] - rows[mem[k - 1]]["nkm"]) / u["nominal"]))
            with_rt = [k for k in mem if rows[k]["rt"] is not None]
            rho = spearman([rows[k]["nkm"] for k in with_rt], [rows[k]["rt"] for k in with_rt])
            ser = {
                "n": len(series) + 1,
                "members": [rows[k]["i"] for k in mem],
                "kmd": sum(kmds) / len(kmds),
                "spanMDa": (max(kmds) - min(kmds)) * 1000.0,
                "steps": steps,
                "consecutive": (steps.count(1) / len(steps)) if steps else 0.0,
                "distinct": len(distinct_nkm),
                "rho": rho,
            }
            series.append(ser)
            for k in mem:
                rows[k]["series"] = ser["n"]
            s = e + 1
        else:
            s += 1
    return {"unit": u, "tolMDa": opts.get("tolMDa", 3), "minMembers": min_members, "rows": rows, "series": series}


def js_clean(obj):
    if isinstance(obj, float) and obj.is_integer():
        return int(obj)
    if isinstance(obj, dict):
        return {k: js_clean(v) for k, v in obj.items()}
    if isinstance(obj, list):
        return [js_clean(v) for v in obj]
    return obj


def build():
    out = {}
    for name, (feats, opts) in CASES.items():
        r = analyse(feats, opts)
        out[name] = {
            "input": {"features": feats, **opts},
            "rows": js_clean([{k: x[k] for k in ("i", "mz", "rt", "km", "nkm", "kmd", "series")} for x in r["rows"]]),
            "series": js_clean([{k: s[k] for k in ("n", "members", "kmd", "spanMDa", "steps", "consecutive", "distinct", "rho")} for s in r["series"]]),
        }
    return out


if __name__ == "__main__":
    text = json.dumps(build(), indent=1) + "\n"
    if "--check" in sys.argv:
        sys.exit(0 if TARGET.read_text() == text else 1)
    TARGET.write_text(text)
