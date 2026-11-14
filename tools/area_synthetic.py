"""Synthetic LC-HRMS files with known peak areas, for the diagnosis of the area of the feature table (`tools/area_diagnosis.py`).

    python3 tools/area_synthetic.py OUT [--seed N]

writes in OUT/ one Orbitrap-like centroid mzML per scenario (its MS1 points also in `<scenario>.npz`: rt, scan, mz, it) and `truth.json`: for every ion its m/z, components (apex RT in minutes, sigma in
seconds, height) and true area (counts x seconds, the integral of the continuous profile). Scenarios vary the m/z jitter (ppm, scan to scan),
a slow m/z drift over the run, the intensity noise and DDA cycles (MS2 scans between MS1 scans when strong ions elute, so the MS1 intervals
are not regular). Ions are single, shouldered (a second Gaussian at +1.5 sigma) or split (two Gaussians 5 sigma apart). Numbers only.
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
from dati_sintetici import _U_MZ, _arr, _cvp, _hr_head  # noqa: E402

RT_END = 12.0
FILT1 = "FTMS + p ESI Full ms [100.0000-1000.0000]"
# scenario: (jitter ppm sd, drift ppm over the run, relative intensity noise sd, dda)
SCENARIOS = {
    "clean": (0.0, 0.0, 0.0, False),
    "j1": (1.0, 0.0, 0.03, False),
    "j2": (2.0, 0.0, 0.03, False),
    "j3": (3.0, 0.0, 0.03, False),
    "drift": (1.0, 4.0, 0.03, False),
    "dda": (1.0, 0.0, 0.03, True),
}
MS1_TIME, MS2_TIME = 0.25 / 60, 0.06 / 60          # minutes per scan
TOP_N, DDA_MIN = 5, 1e6


def ions(rng, n: int = 48) -> list[dict]:
    """Ions on a grid of m/z and RT that keeps every pair apart; kinds cycle single, single, shoulder, split."""
    out = []
    for k in range(n):
        mz = round(150 + 15.37 * k + rng.uniform(0, 3), 5)
        rt = 0.8 + (k % 16) * 0.65 + rng.uniform(0, 0.1)
        h = float(np.exp(rng.uniform(np.log(2.5e5), np.log(3e7))))
        sg = float(rng.uniform(2.0, 6.0))                 # seconds
        kind = ("single", "single", "shoulder", "split")[k % 4]
        comps = [(rt, sg, h)]
        if kind == "shoulder":
            comps.append((rt + 1.5 * sg / 60, sg, 0.4 * h))
        elif kind == "split":
            comps.append((rt + 5 * sg / 60, sg, 0.6 * h))
        area = [c[2] * c[1] * np.sqrt(2 * np.pi) for c in comps]
        out.append({"mz": mz, "kind": kind, "comps": [list(c) for c in comps], "area": sum(area), "comp_area": area})
    return out


def profile(ion: dict, t: float) -> float:
    return sum(h * np.exp(-0.5 * ((t - r) * 60 / s) ** 2) for r, s, h in ion["comps"])


def scan_times(rng, ions_: list[dict], dda: bool) -> tuple[list[float], list[list[float]]]:
    """MS1 times and, per MS1, the precursors of the MS2 that follow it (the strongest eluting ions, as a top-N DDA would pick)."""
    t, ms1, ms2 = 0.0, [], []
    while t < RT_END:
        ms1.append(t)
        prec = []
        if dda:
            y = [(profile(i, t), i["mz"]) for i in ions_]
            prec = [m for v, m in sorted(y, reverse=True)[:TOP_N] if v >= DDA_MIN]
        ms2.append(prec)
        t += MS1_TIME + MS2_TIME * len(prec)
    return ms1, ms2


def spectrum(i: int, rt: float, mz, it, level: int = 1, prec: float | None = None) -> str:
    p = ""
    if prec is not None:
        p = (f'<precursorList count="1"><precursor><selectedIonList count="1"><selectedIon>'
             f'{_cvp("MS:1000744", "selected ion m/z", f"{prec:.5f}", _U_MZ)}</selectedIon></selectedIonList>'
             f'<activation>{_cvp("MS:1000133", "collision-induced dissociation")}{_cvp("MS:1000045", "collision energy", 30)}</activation></precursor></precursorList>')
    filt = FILT1 if level == 1 else f"FTMS + p ESI d Full ms2 {prec:.4f}@hcd30.00 [50.0000-{prec + 10:.4f}]"
    return (f'<spectrum index="{i}" id="controllerType=0 controllerNumber=1 scan={i + 1}" defaultArrayLength="{len(mz)}">'
            + _cvp("MS:1000580" if level > 1 else "MS:1000579", "MSn spectrum" if level > 1 else "MS1 spectrum") + _cvp("MS:1000511", "ms level", level)
            + _cvp("MS:1000130", "positive scan") + _cvp("MS:1000127", "centroid spectrum")
            + f'<scanList count="1">{_cvp("MS:1000795", "no combination")}<scan>'
            + _cvp("MS:1000016", "scan start time", f"{rt:.6f}", ' unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"')
            + _cvp("MS:1000512", "filter string", filt) + '</scan></scanList>' + p
            + '<binaryDataArrayList count="2">' + _arr("MS:1000514", "m/z array", mz) + _arr("MS:1000515", "intensity array", it) + "</binaryDataArrayList></spectrum>")


def write(path: Path, rng, ions_: list[dict], scen: tuple) -> None:
    jit, drift, noise, dda = scen
    ms1, ms2 = scan_times(rng, ions_, dda)
    specs, pts = [], {"rt": np.array(ms1), "scan": [], "mz": [], "it": []}
    for n, (t, precs) in enumerate(zip(ms1, ms2)):
        mz = list(rng.uniform(100, 1000, 150))
        it = list(rng.lognormal(np.log(3e3), 0.6, 150))
        shift = drift * (t / RT_END - 0.5) * 1e-6
        for ion in ions_:
            y = profile(ion, t) * (1 + rng.normal(0, noise) if noise else 1)
            if y >= 1e3:
                mz.append(ion["mz"] * (1 + shift + rng.normal(0, jit) * 1e-6))
                it.append(y)
        o = np.argsort(mz)
        pts["scan"] += [n] * len(o); pts["mz"] += list(np.array(mz)[o]); pts["it"] += list(np.array(it)[o])
        specs.append(spectrum(len(specs), t, np.array(mz)[o], np.array(it)[o]))
        for k, p in enumerate(precs):
            specs.append(spectrum(len(specs), t + MS1_TIME + k * MS2_TIME, np.array([60.0, p * 0.5]), np.array([1e4, 2e4]), 2, p))
    path.write_text(_hr_head("exploris", False, path.stem) + f'<spectrumList count="{len(specs)}">' + "".join(specs)
                    + "</spectrumList></run></mzML>", encoding="utf-8")
    np.savez_compressed(path.with_suffix(".npz"), **{k: np.asarray(v) for k, v in pts.items()})


def main(argv) -> int:
    out = Path(argv[0])
    seed = int(argv[argv.index("--seed") + 1]) if "--seed" in argv else 11
    out.mkdir(parents=True, exist_ok=True)
    rng = np.random.default_rng(seed)
    ions_ = ions(rng)
    truth = {"seed": seed, "scenarios": {k: dict(zip(("jitter_ppm", "drift_ppm", "noise", "dda"), v)) for k, v in SCENARIOS.items()}, "ions": ions_}
    for k, (name, scen) in enumerate(SCENARIOS.items()):
        write(out / f"{name}.mzML", np.random.default_rng([seed, k]), ions_, scen)
    (out / "truth.json").write_text(json.dumps(truth, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
