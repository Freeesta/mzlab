"""Reference answers of `api/composition` through `api.dispatch` for crates/mzlab-core/tests/composition_route_parity.rs.

    python3 tools/composizione_rotta_riferimento.py OUT.json

A list of {query, status, result}: the query strings the page sends (generic formulas, no compound names) and some that Python must refuse.
"""
import json
import sys
import time
from pathlib import Path
from urllib.parse import quote

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from mzlab.api import dispatch  # noqa: E402
from mzlab.chem.elements import ADDUCT_SHIFT, mass, parse_formula  # noqa: E402

FORMS = ["C6H12O6", "C9H10Cl2N2O", "C12H10ClNO2S", "C20H25N3O"]
EL = "C:0-20,H:0-40,N:0-5,O:0-8,S:0-2,Cl:0-3"


def queries() -> list[str]:
    out = []
    for f in FORMS:
        mz = mass(parse_formula(f)) + ADDUCT_SHIFT["[M+H]+"]
        base = f"mz={mz}&elements={quote(EL)}&tol=5&unit=ppm&max=10"
        out += [base, base + "&rdb=0,20&n=even", base + "&rules=HC,LEWIS,SENIOR&ion=" + quote("[M+Na]+"), base + "&m1=0.2&m2=0.03&isotol=0.25",
                f"mz={mz}", f"mz={mz}&tol=2&unit=mda&max=500", f"mz={mz}&parent={f}&elements={quote(EL)}", f"mz={mz}&z=1&ion=" + quote("[M+H]+")]
        obs = ";".join(f"{mz + d}:{i}" for d, i in ((0, 100), (1.00335, 7.5), (2.0067, 3.2), (5.0, 2.0)))
        frags = ";".join(f"{m}:{i}" for m, i in ((mz - 18.0106, 50), (mz - 44.0, 100), (100.0, 20), (60.0, 5)))
        out += [base + f"&obs={quote(obs)}", base + f"&obs={quote(obs)}&norm=mono&w=1,1,3&r200=240000&sppm=1&sint=0.2&imin=0.1&inoise=3",
                base + f"&frags={quote(frags)}&fppm=20", base.replace("max=10", "max=3") + f"&obs={quote(obs)}&frags={quote(frags)}"]
    out += ["", "mz=", "mz=abc", "mz=-5", "mz=150&tol=", "mz=150&rdb=1", "mz=150&rdb=a,b", "mz=150&elements=Xx:1-2", "mz=150&ion=bad", "mz=150&unit=x", "mz=150&z=1.5",
            "mz=150&obs=1:2:3", "mz=150&obs=a:b", "mz=150&w=a,b,c&elements=" + quote(EL), "mz=150&parent=Xx", "mz=150&norm=weird&elements=" + quote(EL)]
    return out


def main(argv) -> int:
    res = []
    for q in queries():
        params = dict(p.split("=", 1) for p in q.split("&") if "=" in p)
        params = {k: __import__("urllib.parse").parse.unquote(v) for k, v in params.items()}
        t = time.perf_counter()
        code, _, body = dispatch(None, "GET", "/api/composition", params)[:3]
        res.append({"query": q, "status": code, "py_s": time.perf_counter() - t, "result": json.loads(body)})
    Path(argv[0]).write_text(json.dumps(res), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
