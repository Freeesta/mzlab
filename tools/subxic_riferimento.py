"""Reference answers of `api/subxic` for the parity test of the Rust port (crates/mzlab-core/tests/subxic_parity.rs).

    python3 tools/subxic_riferimento.py OUT

Writes two synthetic high-resolution DDA files (Orbitrap-like and ion-trap-like) with their `.mzidx` in OUT/ and OUT/subxic.json: a list of
{k, query, status, result, py_s} with the answer of `api.dispatch`. MZLAB_SUBXIC_FILES=a.mzML,b.mzML replaces the synthetic files by real ones.
Prints nothing. No compound names (the formula is the invented one of the generator).
"""
import json
import os
import sys
import time
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import dati_sintetici as ds  # noqa: E402
from mzlab.api import dispatch  # noqa: E402
from mzlab.app import App  # noqa: E402
from mzlab.chem.elements import fmt, parse_formula  # noqa: E402
from mzlab.explore import Session  # noqa: E402
from mzlab.reader import mzidx  # noqa: E402
from mzlab.reader.mzml import Run  # noqa: E402


def ion_formula() -> str:
    f = parse_formula(ds.HR_PARENT)
    f["H"] = f.get("H", 0) + 1                      # [M+H]+: the formula of the ion has one more H
    return fmt(f)


def queries() -> list[tuple[int, str]]:
    f = ion_formula()
    out = []
    for k in (0, 2):                                   # a DDA file is two items of the session (survey, MS2 part): the survey of file 0 is item 0, that of file 1 is item 2
        out += [(k, f"f={f}"), (k, f"f={f}&ppm=2&min=150"), (k, f"f={f}&ppm=10&min=50&z=1"), (k, f"f={f}&ppm=5&min=300"), (k, "f=C6H12O6&ppm=5&min=60"),
                (k, f"f={f}&ppm=0"), (k, f"f={f}&ppm=101"), (k, f"f={f}&min=100000"), (k, "f=Xx"), (k, "f=C60H120N30O40&min=10"), (k, f"f={f}&z=x")]
    return out


def main(argv) -> int:
    out = Path(argv[0])
    out.mkdir(parents=True, exist_ok=True)
    real = [x for x in os.environ.get("MZLAB_SUBXIC_FILES", "").split(",") if x]
    names = []
    for i, kind in enumerate(("exploris", "fusion")):
        f = out / f"s{i}.mzML"
        if real:
            f.unlink(missing_ok=True)
            f.symlink_to(Path(real[i]).resolve())
        else:
            ds.hr_dda(f, np.random.default_rng(20 + i), kind)
        run = Run(f, index_path=out / (f.stem + ".mzidx"))
        mzidx.build(run, out / (f.stem + ".mzidx"))
        run.close()
        names.append(f.name)
    mzidx.REGISTRY.update({n: str(out / (Path(n).stem + ".mzidx")) for n in names})       # each file with its own index (two synthetic files can share size and probe)
    app = App(out)
    app.session = Session([{"file": n} for n in names], out)
    res = []
    for k, query in queries():
        q = {"k": str(k), **{a: b for a, b in (p.split("=", 1) for p in query.split("&"))}}
        t = time.perf_counter()
        code, _, body = dispatch(app, "GET", "/api/subxic", q)[:3]
        res.append({"k": k, "query": query, "status": code, "py_s": time.perf_counter() - t, "result": json.loads(body)})
    (out / "subxic.json").write_text(json.dumps(res), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
