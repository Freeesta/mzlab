"""`cargo_util.wasm_module`: several builds of one crate with other features (reader families, mzFinder modules) write the same target/ path, so a test that packed
that path could get the module another xdist worker had just built or installed there (`err.file.notAccepted` from a `sciex` test packing `shimadzu`).
Each caller gets the copy of its own build; no test reads the shared path of those crates directly."""
import json
import os
import re
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
FAMILIES = ("aperti", "shimadzu", "sciex", "waters")

CHILD = r"""
import os, sys, time
from pathlib import Path
sys.path.insert(0, sys.argv[1])
import cargo_util
cargo_util.ROOT = Path(sys.argv[2])          # a scratch tree: the real target/ keeps its modules
fams = sys.argv[4:]
for i in range(int(sys.argv[3])):
    fam = fams[(i + os.getpid()) % len(fams)]
    p = cargo_util.wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-fake-wasm", "--no-default-features", "--features", fam)
    time.sleep(0.001)                        # the gap between the build and wasm-bindgen
    got = p.read_bytes()
    assert got == f"module {fam}".encode(), (fam, got)
"""


def prebuilt(d: Path):
    """A MZLAB_PREBUILT folder with one fake module per family (manifest as tools/ci_prebuilt.py writes it)."""
    sys.path.insert(0, str(ROOT / "tools"))
    from ci_prebuilt import canon, stored
    m = {}
    for fam in FAMILIES:
        key, = canon(["build", "--target", "wasm32-unknown-unknown", "-p", "mzlab-fake-wasm", "--no-default-features", "--features", fam])
        f = d / stored(key)
        f.parent.mkdir(parents=True, exist_ok=True)
        f.write_bytes(f"module {fam}".encode())
        m[key] = stored(key)
    (d / "manifest-wasm-b.json").write_text(json.dumps(m), encoding="utf-8")


def test_concurrent_workers_each_get_their_own_family(tmp_path):
    pre, tree = tmp_path / "prebuilt", tmp_path / "tree"
    prebuilt(pre)
    env = {**os.environ, "MZLAB_PREBUILT": str(pre)}
    env.pop("MZLAB_PREBUILT_MISSES", None)
    procs = [subprocess.Popen([sys.executable, "-c", CHILD, str(ROOT / "tests"), str(tree), "60", *FAMILIES], env=env, stderr=subprocess.PIPE, text=True) for _ in range(6)]
    errs = [(p.wait(), p.stderr.read()) for p in procs]
    assert all(code == 0 for code, _ in errs), [e for c, e in errs if c]
    copies = sorted((tree / "target/wasm-variants").glob("*/mzlab_fake_wasm.wasm"))
    assert sorted(c.read_bytes() for c in copies) == sorted(f"module {f}".encode() for f in FAMILIES)


def test_only_one_package_for_wasm32():
    from cargo_util import wasm_module
    with pytest.raises(ValueError):
        wasm_module("-p", "mzlab-cli")
    with pytest.raises(ValueError):
        wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm", "-p", "mzlab-wasm")


def test_no_test_packs_the_shared_path_of_a_crate_with_variants():
    shared = re.compile(r"(release/|WASM / \")mzlab_(lettori|feat|attributio)_wasm\.wasm")
    allowed = {"tests/test_ci_prebuilt.py", "tests/test_wasm_module.py"}   # unit tests of the paths themselves
    found = []
    for d in ("tests", "tests_e2e"):
        for f in sorted((ROOT / d).rglob("*.py")):
            rel = f.relative_to(ROOT).as_posix()
            if rel in allowed:
                continue
            found += [f"{rel}:{i}" for i, line in enumerate(f.read_text(encoding="utf-8").splitlines(), 1) if shared.search(line)]
    assert not found, f"use cargo_util.wasm_module for these builds: {found}"
