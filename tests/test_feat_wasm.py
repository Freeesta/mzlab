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


@pytest.fixture(scope="module")
def built(tmp_path_factory):
    """The synthetic series, the native example and the .wasm (nodejs target), built once."""
    pytest.importorskip("numpy")
    root = tmp_path_factory.mktemp("feat")
    ser = root / "ser"
    run("python3", "tools/feat_mirato.py", "--seme", "2", "--profilo", "piccolo", "--scala", "1.0", "--out", str(ser))
    idx = sorted(str(p) for p in ser.glob("*.mzidx"))
    assert len(idx) == 6
    run("cargo", "build", "--release", "--locked", "-q", "-p", "mzlab-feat-wasm", "--example", "nativo")
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm")
    pkg = root / "pkg"
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(pkg), "target/wasm32-unknown-unknown/release/mzlab_feat_wasm.wasm")
    return root, idx, pkg


def native(root, idx, name, *extra):
    out = root / name
    out.mkdir(exist_ok=True)
    got = run("target/release/examples/nativo", str(out), *idx, *PARENT, *extra)
    return got, out


def wasm(root, idx, pkg, name, *extra):
    out = root / name
    out.mkdir(exist_ok=True)
    return run("node", str(ROOT / "tests" / "feat_wasm_node.js"), str(pkg), str(out), *idx, *PARENT, *extra), out


def test_wasm_and_native_give_the_same_mzfeat_bytes(built):
    root, idx, pkg = built
    nat, nat_dir = native(root, idx, "nat")
    got, wasm_dir = wasm(root, idx, pkg, "wasm")
    assert got == nat
    assert "series_evid.mzfeat" in got and "hypotheses" in got
    for f in sorted(p.name for p in nat_dir.iterdir()):
        assert (nat_dir / f).read_bytes() == (wasm_dir / f).read_bytes(), f


@pytest.mark.parametrize("mode", ["ion", "family"])
def test_a_reduced_budget_gives_the_same_sha256_native_and_in_node(built, mode):
    """The contract of memory: the `.mzfeatf`, the `.mzfeat` and the one with `evid` have the same SHA-256 in memory and under a budget (slabs of m/z,
    graph made identity by identity), natively and in the .wasm. Per family the series is not cut: it is in memory (3 MB here) or refused."""
    root, idx, pkg = built
    recipe = ("--recipe", '{"mode":"%s"}' % mode)
    ref, _ = native(root, idx, f"ref-{mode}", *recipe)
    for mb in (192, 3) if mode == "family" else (192, 1, 0.5):
        nat, _ = native(root, idx, f"nat-{mode}-{mb}", *recipe, "--budget-mb", str(mb))
        assert nat == ref, f"native, {mb} MB"
        got, _ = wasm(root, idx, pkg, f"wasm-{mode}-{mb}", *recipe, "--budget-mb", str(mb))
        assert got == ref, f"wasm, {mb} MB"


def test_a_budget_that_fits_nothing_is_refused_with_the_key_and_the_files_saved(built):
    root, idx, pkg = built
    out = root / "refused"
    out.mkdir(exist_ok=True)
    p = subprocess.run(["node", str(ROOT / "tests" / "feat_wasm_node.js"), str(pkg), str(out), *idx, "--recipe", '{"mode":"ion"}', "--budget-mb", "0.05"],
                       cwd=ROOT, capture_output=True, encoding="utf-8")
    assert p.returncode != 0
    assert "error.memoryBudget" in p.stderr and "align" in p.stderr
    # the phase before is already saved
    assert len(list(out.glob("*.mzfeatf"))) == len(idx) and not (out / "series_evid.mzfeat").exists()
