"""Synthetic inputs of the parity tests of `crates/mzlab-feat` (the answers are frozen in tests/golden/feat/).

    python3 tools/feat_sintetici.py OUT

writes in OUT/ five Orbitrap-like MS1 files of `tools/dati_sintetici.py` (invented ions: weak, intense, wobbling in m/z, side lobes, continuous
background, none) and their `.mzidx`: `<name>.mzidx` for `trace_parity.rs` (answers: `detect.json`).

    python3 tools/feat_sintetici.py OUT --solo identita

writes the small synthetic identity series (seed 7, `tools/valida/sintetici_identita.py`) in OUT/identita/ and the `.mzidx` of every file
(`OUT/ri_<name>.mzidx`); `OUT/identita.mzidx` is the one of SIN_t10min (answers: `iso_ms2_identita.json`, `famiglie_*.json`, `riempimento.json`).
No compound names: every ion is a number.
"""
import shutil
import sys
from pathlib import Path

import numpy as np

AQUI = Path(__file__).resolve().parent
sys.path.insert(0, str(AQUI))
sys.path.insert(0, str(AQUI.parent))
import dati_sintetici as ds  # noqa: E402
from mzlab.reader import mzidx  # noqa: E402
from mzlab.reader.mzml import Run  # noqa: E402


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


def indice(mz: Path, ix: Path) -> None:
    run = Run(mz, index_path=ix)
    mzidx.build(run, ix)
    run.close()


def identita(out: Path) -> int:
    from valida import sintetici_identita as si
    d = out / "identita"
    si.genera(7, "piccolo", d)
    files = sorted(d.glob("SIN_*.mzML"))
    for f in files:
        indice(f, out / f"ri_{f.stem}.mzidx")
    shutil.copyfile(out / "ri_SIN_t10min.mzidx", out / "identita.mzidx")
    return len(files)


def main(argv) -> int:
    out = Path(argv[0])
    out.mkdir(parents=True, exist_ok=True)
    if "--solo" in argv and argv[argv.index("--solo") + 1] == "identita":
        print("identita", identita(out))
        return 0
    for nome, (seed, ioni, fine) in piani().items():
        mz = out / f"{nome}.mzML"
        ds.hr_series(mz, np.random.default_rng(seed), ioni, rt_end=fine, cyc=0.008 if nome == "fondo" else 0.04)   # "fondo": > 400 scans
        indice(mz, out / f"{nome}.mzidx")
        print(nome)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
