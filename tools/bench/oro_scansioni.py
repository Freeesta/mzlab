"""Per-scan points of the ADAP chromatograms of the gold data of mzmine 3.9.0 -> `tests/data/untargeted/mzmine39/adap_scansioni.csv` and `adap_asse.csv`.

`adap.csv` has only the range of each chromatogram (mzmine exports no intensity per scan), so the parity test of the resolver (`crates/mzlab-feat/tests/untargeted_resolve.rs`)
needs the points: for each chromatogram of `adap.csv` and each scan of the file, the most intense peak of the scan (>= the noise level of the batch) within the tolerance of the
ADAP builder around the m/z of the chromatogram (one point per scan, the first and so the most intense, as the builder). Only a test file writer: no stage of mzLab comes from here.
    python3 tools/bench/oro_scansioni.py [--dest DIR]"""
from __future__ import annotations

import argparse
import csv
import sys
import tempfile
from pathlib import Path

import numpy as np

RADICE = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(RADICE), str(RADICE / "tools"), str(RADICE / "tools" / "bench")]
import oro_mzmine  # noqa: E402
from mzlab.reader.mzml import Run  # noqa: E402

DEST = oro_mzmine.DEST
RUMORE = 1e4            # "Noise level" of the mass detection of the batch
TOL_ABS, TOL_PPM = 0.002, 5.0


def punti(run: Run, mz0: float) -> list[tuple[int, float, float]]:
    """(scan, m/z, intensity) of the most intense peak within the tolerance, for each scan that has one."""
    tol = max(TOL_ABS, mz0 * TOL_PPM * 1e-6)
    out = []
    for i in range(len(run.scans)):
        mz, it = run.read(i)
        ok = np.flatnonzero((np.abs(mz - mz0) <= tol) & (it >= RUMORE))
        if len(ok):
            k = ok[int(np.argmax(it[ok]))]
            out.append((i, float(mz[k]), float(it[k])))
    return out


def scrivi(dest: Path) -> None:
    with tempfile.TemporaryDirectory() as tmp:
        oro_mzmine.genera(Path(tmp))
        runs = {n: Run(Path(tmp) / f"{n}.mzML") for n in oro_mzmine.DERIVA}
        righe = list(csv.DictReader((dest / "adap.csv").open()))
        with (dest / "adap_scansioni.csv").open("w", newline="") as f:
            w = csv.writer(f, lineterminator="\n")
            w.writerow(["file", "id", "scan", "mz", "intensity"])
            for r in righe:
                for i, mz, it in punti(runs[r["file"]], float(r["mz"])):
                    w.writerow([r["file"], r["id"], i, f"{mz:.6f}", f"{it:.1f}"])
        rts = [[s.rt for s in runs[n].scans] for n in runs]
        assert all(r == rts[0] for r in rts), "the four files must share the RT axis"
        with (dest / "adap_asse.csv").open("w", newline="") as f:
            w = csv.writer(f, lineterminator="\n")
            w.writerow(["scan", "rt"])
            for i, rt in enumerate(rts[0]):
                w.writerow([i, repr(float(rt))])
        for n in runs:
            runs[n].close()


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--dest", default=str(DEST), metavar="DIR")
    scrivi(Path(ap.parse_args().dest))


if __name__ == "__main__":
    main()
