"""Reference answers of /api/qaqc for crates/mzlab-core/tests/qaqc_parity.rs.

    python3 tools/qaqc_riferimento.py OUT

Writes the synthetic files of tools/differenza_riferimento.py in OUT/ and OUT/qaqc.json = [{query, status, body}] answered by
`mzlab.api.dispatch`; prints "<mzML path>" lines in the order of the session. No data of the laboratory, no compound names.
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from differenza_riferimento import session  # noqa: E402
from mzlab.api import dispatch  # noqa: E402


def queries() -> list[dict]:
    out = []
    ions = (("364.4", "14.3"), ("194.2", "14.3"), ("305.3", "14.7"), ("229.1", "7.07"), ("99999", "5"), ("364.4", "30"))
    series = (("0,1,2", "sample,blank,qc"), ("2,1,0", "blank,sample,sample"), ("0,1,2", "qc,qc,qc"), ("0,1,2", "dark,blank,sample"), ("1", "sample"),
              ("0,1,2,3", "sample,sample,qc,sample"), ("0,0,1", "blank,qc,sample"))
    for mz, rt in ions:
        for ks, roles in series:
            base = {"k": ks, "roles": roles, "mz": mz, "rt": rt}
            for extra in ({}, {"win": "1", "ratio": "1"}, {"win": "0.2", "tol": "0.3"}, {"unit": "ppm", "tol": "1000", "ratio": "1", "thr": "10"},
                          {"unit": "ppm", "tol": "20"}, {"level": "2", "tol": "1"}, {"win": "0.01"}):
                out.append({**base, **extra})
    out += [{"k": "0,1", "roles": "sample", "mz": "364.4", "rt": "14.3"}, {"k": "0,1", "mz": "364.4", "rt": "14.3"}, {"k": "0", "roles": "sample", "rt": "14.3"},
            {"k": "0", "roles": "sample", "mz": "x", "rt": "14.3"}, {"k": "0", "roles": "sample", "mz": "364.4", "rt": "14.3", "win": "x"},
            {"k": "0", "roles": "sample", "mz": "364.4", "rt": "14.3", "level": "x"}, {"k": "x", "roles": "sample", "mz": "364.4", "rt": "14.3"}]
    return out


def main(argv) -> int:
    out = Path(argv[0])
    paths, app = session(out)
    rows = []
    for q in queries():
        status, _ctype, body, _h = dispatch(app, "GET", "/api/qaqc", q)
        rows.append({"query": q, "status": status, "body": json.loads(body)})
    (out / "qaqc.json").write_text(json.dumps(rows), encoding="utf-8")
    for f in paths:
        print(f)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
