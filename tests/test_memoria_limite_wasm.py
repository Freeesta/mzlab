"""The engine under the memory a browser grants (docs/agenti/memory-limits.md): files opened one after the other in ONE engine on a synthetic DDA-like index
(`mem_idx` example, no real data), with `node --wasm-max-mem-pages` imitating a browser that grants an instance less than the build ceiling.
The rule: the engine refuses cleanly BEFORE the memory runs out (`error.memoryBudget`), never a trap (`unreachable`), whatever the grant."""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest
from cargo_util import build

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
needs = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")
MB = 16  # wasm pages per MB


def run(*a, **kw):
    r = subprocess.run(a, cwd=ROOT, capture_output=True, encoding="utf-8", **kw)
    assert r.returncode == 0, f"{' '.join(a)} exited {r.returncode}:\n{r.stderr[-4000:]}"
    return r.stdout


@pytest.fixture(scope="module")
def lab(tmp_path_factory):
    d = tmp_path_factory.mktemp("memlimit")
    build("--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm", runner=run)
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "eng"), "target/wasm32-unknown-unknown/release/mzlab_wasm.wasm")
    run("cargo", "run", "-q", "--release", "-p", "mzlab-core", "--example", "mem_idx", "--", str(d / "f.mzidx"), "60000")
    return d


def opens(lab, grant_mb, copies=8):
    """The outcome of each open: 'o' opened, 'r' refused with `error.memoryBudget`, 'T' a trap."""
    out = run("node", f"--wasm-max-mem-pages={grant_mb * MB}", "tests/memoria_limite_node.js", str(lab / "eng"), "48", str(copies), str(lab / "f.mzidx"))
    d = json.loads(out.strip().splitlines()[-1])
    assert all(f["error"] == "error.memoryBudget" for f in d["files"] if not f["ok"]), d
    return "".join("T" if f.get("trap") else "o" if f["ok"] else "r" for f in d["files"])


@needs
@pytest.mark.parametrize("grant_mb", [64, 80, 96, 112, 128])
def test_no_grant_ends_in_a_trap_and_the_files_that_fit_open(lab, grant_mb):
    got = opens(lab, grant_mb)
    assert "T" not in got, f"{grant_mb} MB granted: {got}"
    assert got.startswith("o"), f"{grant_mb} MB granted: nothing opens: {got}"
    assert got == "o" * got.count("o") + "r" * got.count("r"), f"{grant_mb} MB granted: a refusal then an open: {got}"


@needs
def test_a_bigger_grant_opens_more_files_never_fewer(lab):
    n = [opens(lab, g).count("o") for g in (64, 96, 128, 256)]
    assert n == sorted(n) and n[-1] > n[0], n


@needs
def test_scan_tables_stay_outside_the_cache_budget(lab):
    """Open decision (b), left as it is: a tiny cache budget does not refuse files, because their scan tables are not counted in it (the refusal comes from the real room)."""
    out = run("node", f"--wasm-max-mem-pages={512 * MB}", "tests/memoria_limite_node.js", str(lab / "eng"), "0.001", "4", str(lab / "f.mzidx"))
    d = json.loads(out.strip().splitlines()[-1])
    assert [f["ok"] for f in d["files"]] == [True] * 4, d
