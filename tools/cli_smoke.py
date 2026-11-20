#!/usr/bin/env python3
"""Smoke test of a built mzlab-cli binary, used by the release workflow (jobs cli-mac and cli-smoke) and runnable by hand.

    python3 tools/cli_smoke.py BINARY                       # runs, --version, commands --json, flow steps --json
    python3 tools/cli_smoke.py BINARY --reference NATIVE    # plus: byte-for-byte equal stdout and exit code against a native build
    python3 tools/cli_smoke.py BINARY --reference NATIVE --data DIR   # DIR: output of tools/dati_sintetici.py (adds data commands)

The reference is the same source built natively (glibc), so a difference means the release build (musl, another OS) computes
something else. Stdlib only; exit 0 = ok, 1 = a check failed (the reason is on stderr), 2 = bad arguments.
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

# Commands whose stdout must be identical on every build: they need no input, or only the anonymous synthetic file.
FIXED = [
    ["commands", "--json"],
    ["flow", "steps", "--json"],
    ["schema", "--list"],
    ["ion", "pattern", "C20H20N3O4S", "[M+H]+"],
]
SAMPLE = "B_FullMass-t0.mzML"
WITH_DATA = [
    ["file", "info", "{f}", "--json"],
    ["trace", "chrom", "{f}", "--id", "tic", "--format", "csv"],
    ["trace", "xic", "{f}", "--mz", "364.4", "--da", "0.5", "--format", "csv"],
]


def run(binary: str, args: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run([binary, *args], capture_output=True, timeout=300, check=False)


def fail(msg: str) -> None:
    print(f"cli_smoke: FAIL: {msg}", file=sys.stderr)
    raise SystemExit(1)


def must_run(binary: str, args: list[str]) -> bytes:
    p = run(binary, args)
    if p.returncode != 0:
        fail(f"{' '.join(args)} exited {p.returncode}: {p.stderr.decode(errors='replace')[:300]}")
    return p.stdout


def checks(binary: str) -> None:
    ver = must_run(binary, ["--version"]).decode()
    print(ver.strip().splitlines()[0])
    if "mzLab" not in ver:
        fail("--version does not name mzLab")
    if not re.search(r"\(commit [0-9a-f]{7,40}\)", ver):  # `unknown` means the build knew no commit: not a release binary
        fail(f"--version does not carry a commit: {ver.splitlines()[0]!r}")
    cat = must_run(binary, ["commands", "--json"])
    print(cat.decode()[:200] + " ...")  # the start of the catalog, like `commands --json | head -c 200` (it is one long line)
    doc = json.loads(cat)
    if not doc.get("commands"):
        fail("commands --json has no commands")
    steps = json.loads(must_run(binary, ["flow", "steps", "--json"]))
    if not steps:
        fail("flow steps --json is empty")


def compare(binary: str, reference: str, data: Path | None) -> int:
    runs = [list(c) for c in FIXED]
    if data is not None:
        f = data / SAMPLE
        if not f.is_file():
            fail(f"{f} not found (run tools/dati_sintetici.py first)")
        runs += [[a.replace("{f}", str(f)) for a in c] for c in WITH_DATA]
    for args in runs:
        a, b = run(binary, args), run(reference, args)
        label = " ".join(Path(x).name if x.startswith("/") else x for x in args)
        if a.returncode != b.returncode:
            fail(f"{label}: exit {a.returncode} against the native {b.returncode}")
        if a.stdout != b.stdout:
            fail(f"{label}: stdout differs from the native build ({len(a.stdout)} against {len(b.stdout)} bytes)")
        if a.returncode != 0:
            fail(f"{label}: exit {a.returncode}, nothing to compare: {a.stderr.decode(errors='replace')[:300]}")
        print(f"identical to the native build: {label} ({len(a.stdout)} bytes, exit {a.returncode})")
    return len(runs)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("binary")
    ap.add_argument("--reference", help="a natively built mzlab-cli to compare the output with, byte for byte")
    ap.add_argument("--data", type=Path, help="folder written by tools/dati_sintetici.py (adds the commands that read a file)")
    a = ap.parse_args(argv)
    if a.data and not a.reference:
        ap.error("--data needs --reference")
    checks(a.binary)
    if a.reference:
        compare(a.binary, a.reference, a.data)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
