#!/usr/bin/env python3
"""Rewrites the committed answers of the Python reference (tests/golden/ref/) that the parity tests of crates/mzlab-core read when they are there:
`cargo test` then needs no python3 for them. Run it after a deliberate change of a `tools/*_riferimento.py` script or of the Python it calls.

    python3 tools/crea_golden_ref.py            # all
    python3 tools/crea_golden_ref.py stats      # only the named ones

Only the small ones are committed (a few MB together); the others (maps, origin, indexes of the example files: hundreds of MB) are written to a
temporary folder by the test itself, which then needs python3 and numpy (it is skipped without them; MZLAB_GOLDEN_STRICT=1 makes that a failure)."""
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "tests" / "golden" / "ref"
REF = {                       # committed name -> script
    "composizione.json": "composizione_riferimento.py",
    "composizione_rotta.json": "composizione_rotta_riferimento.py",
    "msntree.json": "msntree_riferimento.py",
    "stats.json": "stats_riferimento.py",
    "componenti.json": "componenti_riferimento.py",
    "statmat.json": "statmat_riferimento.py",
    "statistica.json": "statistica_riferimento.py",
    "msntree_rotta": "msntree_rotta_riferimento.py",
}


def main() -> None:
    want = set(sys.argv[1:])
    for name, script in REF.items():
        if want and Path(name).stem not in want:
            continue
        dest = OUT / name
        if dest.is_dir():
            shutil.rmtree(dest)
        subprocess.run([sys.executable, str(ROOT / "tools" / script), str(dest)], check=True, cwd=ROOT)
        print("written", dest.relative_to(ROOT))


if __name__ == "__main__":
    main()
