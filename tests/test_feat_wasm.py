"""The pipeline of mzFinder in WebAssembly (crates/mzlab-feat-wasm) gives the same bytes as the native engine: the `.mzfeatf` of every file, the `.mzfeat` of the
series and the one with the graph of the proofs have the same SHA-256 whether the example `nativo` or the `.wasm` (through Node) makes them. The series is the
synthetic one of tools/feat_mirato.py (profile `piccolo`). Skipped without cargo, the wasm32 target, wasm-bindgen, node or numpy."""
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")
PARENT = ["--parent", "C21H32N2O2", "--rules", str(ROOT / "TP_Mine/data/regole_tp.json"), "--parent-mz", "345.253655", "--parent-rt", "1.0"]


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout


def test_wasm_and_native_give_the_same_mzfeat_bytes(tmp_path):
    pytest.importorskip("numpy")
    ser = tmp_path / "ser"
    run("python3", "tools/feat_mirato.py", "--seme", "2", "--profilo", "piccolo", "--scala", "1.0", "--out", str(ser))
    idx = sorted(str(p) for p in ser.glob("*.mzidx"))
    assert len(idx) == 6
    nat, wasm = tmp_path / "nat", tmp_path / "wasm"
    nat.mkdir()
    wasm.mkdir()
    native = run("cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-feat-wasm", "--example", "nativo", "--", str(nat), *idx, *PARENT)
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm")
    pkg = tmp_path / "pkg"
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(pkg), "target/wasm32-unknown-unknown/release/mzlab_feat_wasm.wasm")
    got = run("node", str(ROOT / "tests" / "feat_wasm_node.js"), str(pkg), str(wasm), *idx, *PARENT)
    assert got == native
    assert "series_evid.mzfeat" in got and "hypotheses" in got
    for f in sorted(p.name for p in nat.iterdir()):
        assert (nat / f).read_bytes() == (wasm / f).read_bytes(), f
