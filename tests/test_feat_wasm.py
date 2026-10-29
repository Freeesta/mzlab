"""The pipeline of mzFinder in WebAssembly (crates/mzlab-feat-wasm) gives the same bytes as the native engine: the `.mzfeatf` of every file, the `.mzfeat` of the
series and the one with the graph of the proofs have the same SHA-256 whether the example `nativo` or the `.wasm` (through Node) makes them. The series is the
synthetic one of tools/feat_mirato.py (profile `piccolo`; the tests of the `Spill` of the page use the profile `debole`). Skipped without cargo, the wasm32 target, wasm-bindgen, node or numpy."""
import re
import shutil
import subprocess
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")
PARENT = ["--parent", "C21H32N2O2", "--rules", str(ROOT / "TP_Mine/data/regole_tp_2.json"), "--parent-mz", "345.253655", "--parent-rt", "1.0"]


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout


@pytest.fixture(scope="module")
def built(tmp_path_factory):
    """The synthetic series, the native example and the .wasm (nodejs target), built once."""
    pytest.importorskip("numpy")
    root = tmp_path_factory.mktemp("feat")
    ser = root / "ser"
    # the series (python) is independent of the two builds (cargo takes the lock of the target folder one after the other): they run at the same time
    with ThreadPoolExecutor(2) as pool:
        made = pool.submit(run, "python3", "tools/feat_mirato.py", "--seme", "2", "--profilo", "piccolo", "--scala", "1.0", "--out", str(ser))
        run("cargo", "build", "--release", "--locked", "-q", "-p", "mzlab-feat-wasm", "--example", "nativo")
        run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm")
        made.result()
    idx = sorted(str(p) for p in ser.glob("*.mzidx"))
    assert len(idx) == 6
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
    graph made identity by identity), natively and in the .wasm. Per family too (the groups of ions made slab by slab, the votes read a file at a time)."""
    root, idx, pkg = built
    recipe = ("--recipe", '{"mode":"%s"}' % mode)
    ref, _ = native(root, idx, f"ref-{mode}", *recipe)
    for mb in (192, 3, 1.5) if mode == "family" else (192, 1, 0.5):
        nat, _ = native(root, idx, f"nat-{mode}-{mb}", *recipe, "--budget-mb", str(mb))
        assert nat == ref, f"native, {mb} MB"
        got, _ = wasm(root, idx, pkg, f"wasm-{mode}-{mb}", *recipe, "--budget-mb", str(mb))
        assert got == ref, f"wasm, {mb} MB"


@pytest.fixture(scope="module")
def weak(built):
    """The series `debole` (tools/feat_mirato.py: `piccolo` plus thousands of weak features in every file) and the bytes of the native engine in memory."""
    root, _, _ = built
    ser = root / "ser-debole"
    run("python3", "tools/feat_mirato.py", "--seme", "2", "--profilo", "debole", "--out", str(ser))
    idx = sorted(str(p) for p in ser.glob("*.mzidx"))
    assert len(idx) == 6
    ref, _ = native(root, idx, "ref-debole", "--recipe", '{"mode":"family"}')
    return idx, ref


def spilling(built, idx, name, mb, spill):
    root, _, pkg = built
    out = root / name
    out.mkdir(exist_ok=True)
    args = ["--budget-mb", str(mb)] + (["--spill", spill] if spill else [])
    return subprocess.run(["node", str(ROOT / "tests" / "feat_wasm_node.js"), str(pkg), str(out), *idx, *PARENT, "--recipe", '{"mode":"family"}', *args],
                          cwd=ROOT, capture_output=True, encoding="utf-8")


@pytest.mark.parametrize("mb", [1.5, 0.5])
def test_the_weak_tables_in_a_spill_of_the_page_give_the_same_sha256(built, weak, mb):
    """The optional `spill` of `align_series_to` (an object of the page with `put` and `get`) changes where the weak tables wait, not the bytes: with the
    series `debole` the weak tables do not fit in half of what is free, so each of the 6 files goes to `put` and every slab reads all the files with `get`."""
    idx, ref = weak
    p = spilling(built, idx, f"spill-{mb}", mb, "1")
    assert p.returncode == 0, p.stderr
    assert p.stdout == ref, f"{mb} MB"
    puts, gets = map(int, re.search(r"spill puts (\d+) gets (\d+)", p.stderr).groups())
    assert puts == len(idx)
    assert gets >= 2 * len(idx) and gets % len(idx) == 0       # more than one slab, each reading every file


def test_a_budget_where_the_weak_tables_do_not_fit_is_refused_without_a_spill(built, weak):
    idx, _ = weak
    p = spilling(built, idx, "no-spill", 1.5, None)
    assert p.returncode != 0
    assert '"error_key":"error.memoryBudget"' in p.stderr and '"fase":"align"' in p.stderr


def test_a_get_that_throws_is_a_refusal_with_the_key_and_not_a_panic(built, weak):
    idx, _ = weak
    p = spilling(built, idx, "get-fails", 1.5, "fail")
    assert p.returncode != 0
    assert '"error_key":"error.memoryBudget"' in p.stderr and '"fase":"align"' in p.stderr
    assert "panick" not in p.stderr and "unreachable" not in p.stderr


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
