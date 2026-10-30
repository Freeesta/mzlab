#!/usr/bin/env python3
"""Builds the in-silico candidate package of the site: downloads the pinned PubChemLite CSV (SHA-256 checked, `tools/genera_sospetti.py`) and runs the Rust packer on it.

CI only (`pages.yml`, job packages). No computation here: `mzlab-cli insilico pack` makes the pieces and the catalogue. A network failure only warns: the site is then
published without the package and the button says so."""
import argparse
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import genera_sospetti as g  # noqa: E402


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--fonte", type=Path, required=True, help="folder for the downloaded CSV")
    ap.add_argument("--cli", type=Path, required=True, help="the built mzlab-cli")
    ap.add_argument("--out", type=Path, required=True, help="folder of the package (static/insilico)")
    a = ap.parse_args()
    src = next(s for s in g.SOURCES if s["id"] == "pubchemlite")
    try:
        csv = g.prepara_fonti([src], a.fonte)["pubchemlite"][0]
    except (OSError, RuntimeError) as e:
        print(f"::warning::PubChemLite not downloaded ({e}): the site is published without the in-silico candidate package")
        return 0
    r = subprocess.run([str(a.cli), "insilico", "pack", str(csv), str(a.out), "--version", src["versione"], "--doi", src["doi"]], check=False)
    return r.returncode


if __name__ == "__main__":
    sys.exit(main())
