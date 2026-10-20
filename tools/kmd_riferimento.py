"""Reference of the Kendrick mass defect of `crates/mzlab-feat/src/stat/kmd.rs` (the code of the site is Rust; this one only writes the frozen answers).

    python3 tools/kmd_riferimento.py            # writes tests/golden/kmd.json (needs node)
    python3 tools/kmd_riferimento.py --check    # exits 1 when the file differs

The answers are those of `mzlab/web/kmd-calc.js` for the cases of `tests/test_kmd.py` (a PFCA series with two unrelated ions, a series with a missing
member, a short series, another unit) and one list with ions without RT.
"""
import json
import subprocess
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


def node(feats, opts):
    code = f"const K=require({str(ROOT / 'mzlab' / 'web' / 'kmd-calc.js')!r});console.log(JSON.stringify(K.analyse({json.dumps(feats)}, {json.dumps(opts)})))"
    return json.loads(subprocess.run(["node", "-e", code], check=True, capture_output=True, text=True).stdout)


def build():
    out = {}
    for name, (feats, opts) in CASES.items():
        r = node(feats, opts)
        out[name] = {"input": {"features": feats, **opts}, "rows": [{k: x[k] for k in ("i", "mz", "rt", "km", "nkm", "kmd", "series")} for x in r["rows"]],
                     "series": [{k: s[k] for k in ("n", "members", "kmd", "spanMDa", "steps", "consecutive", "distinct", "rho")} for s in r["series"]]}
    return out


if __name__ == "__main__":
    text = json.dumps(build(), indent=1) + "\n"
    if "--check" in sys.argv:
        sys.exit(0 if TARGET.read_text() == text else 1)
    TARGET.write_text(text)
