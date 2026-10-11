"""Reference answers of `tpmine.hr.features.detect` for the parity test of the Rust crate (crates/mzlab-feat/tests/trace_parity.rs).

    python3 tools/feat_riferimento.py OUT [--golden]

Writes in OUT/ the synthetic mzML files of `tools/dati_sintetici.py` (Orbitrap-like MS1 series with invented ions: weak, intense, wobbling in
m/z, side lobes, continuous background), their `.mzidx` built by the Python builder and `detect_<name>.json`: the features of
`features.detect` with the first and last scan of every trace. With --golden the JSON files go to tests/golden/feat/ as well.
No compound names: every ion is a number.
"""
import json
import sys
from pathlib import Path

import numpy as np

AQUI = Path(__file__).resolve().parent
sys.path.insert(0, str(AQUI))
sys.path.insert(0, str(AQUI.parent))
sys.path.insert(0, str(AQUI.parent / "TP_Mine" / "py"))
import dati_sintetici as ds  # noqa: E402
from mzlab.reader import mzidx  # noqa: E402
from mzlab.reader.mzml import Run  # noqa: E402
from tpmine.hr import features  # noqa: E402


def piani() -> dict[str, tuple[int, list[tuple], float]]:
    """name -> (seed, ions (m/z, RT, sigma, amplitude), RT end)."""
    rng = np.random.default_rng(11)
    deboli = [(float(m), float(c), 0.05, float(a)) for m, c, a in zip(rng.uniform(150, 800, 40), rng.uniform(1, 9, 40), 10 ** rng.uniform(5.3, 7, 40))]
    intensi = [(300.1234, 4.0, 0.06, 4e8), (300.1234 * (1 + 60e-6), 4.0, 0.06, 5e6),       # a side lobe 60 ppm from an intense ion
               (455.2, 6.0, 0.05, 9e7), (455.2 * (1 + 3e-6), 6.02, 0.05, 2e7),           # two ions 3 ppm apart
               (610.31, 2.5, 0.2, 3e7)]
    fondo = [(250.05, 0.5 + 0.001, 40.0, 3e6), (420.4, 1.0, 40.0, 2.5e5)]                 # continuous background (sigma 40 min = whole run)
    vicini = [(500.0 + 0.0012 * k, 5.0 + 0.3 * (k % 3), 0.04, 5e5 * (1 + k)) for k in range(6)]
    return {"deboli": (21, deboli, 12.0), "intensi": (22, intensi + deboli[:10], 12.0), "fondo": (23, fondo + deboli[:15], 6.0),
            "vicini": (24, vicini, 10.0), "vuoto": (25, [], 3.0)}


def un_file(out: Path, nome: str, seed: int, ioni: list, fine: float, golden: Path | None) -> int:
    mz = out / f"{nome}.mzML"
    ds.hr_series(mz, np.random.default_rng(seed), ioni, rt_end=fine, cyc=0.008 if nome == "fondo" else 0.04)   # "fondo": > 400 scans
    ix = out / f"{nome}.mzidx"
    run = Run(mz, index_path=ix)
    mzidx.build(run, ix)
    t = run.table(1, 1)
    kw = {"long_trace": 60} if nome == "fondo" else {}          # the synthetic traces are shorter than 400 scans: a lower limit marks the background
    f = features.detect(t, **kw)
    srt = np.asarray(f.scan_rt)
    lo = np.searchsorted(srt, f.left)
    hi = np.searchsorted(srt, f.right)
    doc = {"name": nome, "n": len(f), "dt": f.dt, "params": kw,
           "mz": f.mz.tolist(), "rt": f.rt.tolist(), "apex": f.apex.tolist(), "area": f.area.tolist(), "height": f.height.tolist(),
           "npts": f.npts.tolist(), "left": f.left.tolist(), "right": f.right.tolist(), "scan_lo": lo.tolist(), "scan_hi": hi.tolist(),
           "background": f.background.astype(int).tolist(), "artefact": f.artefact.astype(int).tolist()}
    txt = json.dumps(doc, separators=(",", ":"))
    (out / f"detect_{nome}.json").write_text(txt, encoding="utf-8")
    if golden is not None:
        golden.mkdir(parents=True, exist_ok=True)
        (golden / f"detect_{nome}.json").write_text(txt, encoding="utf-8")
    run.close()
    return len(f)


def main(argv) -> int:
    out = Path(argv[0])
    out.mkdir(parents=True, exist_ok=True)
    golden = AQUI.parent / "tests" / "golden" / "feat" if "--golden" in argv else None
    for nome, (seed, ioni, fine) in piani().items():
        print(nome, un_file(out, nome, seed, ioni, fine, golden))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
