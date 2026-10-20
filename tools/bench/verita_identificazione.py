#!/usr/bin/env python3
"""Test 5 of the workflow study (`docs/studi/WORKFLOW_MODULARI.md` §7): library match against an external truth. The spectra of one public library are the
queries, another is the library (decoys of `decoy.rs`); the Rust example `verita_lib` measures, per score (cosine, modified cosine, spectral entropy), the
rank-1 rate and the hits at q <= 1 %. This script only runs it, hashes the inputs and writes a `prov/1`-style record; it calculates nothing.
Declared beforehand: entropy rank-1 >= cosine rank-1, and the answer is the same on every run. The licences of the files are read by Federico, the files stay
outside git, and the numbers go to the PRIVATE repository. Without the files it prints a message and exits 0.

    python3 tools/bench/verita_identificazione.py LIB QUERIES [--out FILE] [--seed N]
"""
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EXAMPLE = ["cargo", "run", "--release", "-q", "-p", "mzlab-lib", "--example", "verita_lib", "--"]


def sha256(path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while chunk := f.read(1 << 20):
            h.update(chunk)
    return h.hexdigest()


def measure(lib, queries, seed=1) -> dict:
    p = subprocess.run([*EXAMPLE, str(lib), str(queries), str(seed)], cwd=ROOT, check=True, capture_output=True, encoding="utf-8")
    return json.loads(p.stdout)


def outcomes(m: dict) -> dict:
    s = m["scores"]
    return {"entropy_rank1_not_below_cosine": s["spectral_entropy"]["rank1"] >= s["cosine"]["rank1"]}


def record(lib, queries, seed=1) -> dict:
    first, second = measure(lib, queries, seed), measure(lib, queries, seed)
    return {
        "schema": "prov/1",
        "activity": {"step": "spectra.match", "params": {"seed": seed, "precursor_ppm": 10, "fragment_da": 0.01, "decoy": "fragments"}},
        "used": {"library": {"sha256": sha256(lib)}, "queries": {"sha256": sha256(queries)}},
        "generated": first,
        "outcomes": {**outcomes(first), "deterministic": first == second},
    }


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("library")
    ap.add_argument("queries")
    ap.add_argument("--out")
    ap.add_argument("--seed", type=int, default=1)
    a = ap.parse_args(argv)
    if not (Path(a.library).is_file() and Path(a.queries).is_file()):
        print("skipped: library or query file not found (public libraries are kept outside the repository)")
        return 0
    text = json.dumps(record(a.library, a.queries, a.seed), indent=1, sort_keys=True)
    if a.out:
        Path(a.out).write_text(text + "\n", encoding="utf-8")
    else:
        print(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
