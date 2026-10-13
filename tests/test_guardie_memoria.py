"""Guards of memory (docs/agenti/memoria.md): the peak of the linear memory of the .wasm after each phase, on the synthetic series `piccolo`, must stay under the
recorded value + 10 % (tools/guardie_memoria.json). The growth of WASM memory is deterministic for a given input: it is a guard of the architecture, not a measure of
speed. To record new values on purpose: MZLAB_GUARDIE_CREA=1 pytest tests/test_guardie_memoria.py (and say why in the PR). Skipped without cargo, the wasm32 target,
wasm-bindgen, node or numpy. The static rules (exported WASM functions that return a whole Vec<u8>, IndexedDB) are in tools/controlla.py and tested here."""
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
GUARD = ROOT / "tools" / "guardie_memoria.json"
SLACK = 1.10
BLOCKS_MB = 24  # budget that makes align and proofs of the series `completo` take the way in slabs
sys.path.insert(0, str(ROOT / "tools"))


def run(*a, **kw):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8", **kw).stdout


needs = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


def build_wasm(tmp_path):
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm", "-p", "mzlab-wasm")
    for crate, name in (("feat", "mzlab_feat_wasm"), ("eng", "mzlab_wasm")):
        run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(tmp_path / crate), f"target/wasm32-unknown-unknown/release/{name}.wasm")


def series(tmp_path, profile):
    ser = tmp_path / f"ser_{profile}"
    run("python3", "tools/feat_mirato.py", "--seme", "2", "--profilo", profile, "--scala", "1.0", "--out", str(ser))
    return sorted(str(p) for p in ser.glob("*.mzidx"))


def measure(tmp_path, idx, budget, tag):
    """Peaks of the linear memory after each phase (twice: the growth is deterministic) and the SHA-256 of the two outputs."""
    out = tmp_path / f"out_{tag}"
    out.mkdir()
    env = dict(os.environ, RULES=str(ROOT / "TP_Mine/data/regole_tp.json"), BUDGET_MB=str(budget))
    cmd = ("node", "tests/guardie_memoria_node.js", str(tmp_path / "feat"), str(tmp_path / "eng"), str(out), *idx)
    got = json.loads(run(*cmd, env=env))
    assert got == json.loads(run(*cmd, env=env)), "the growth of the memory is not deterministic"
    sha = {n: hashlib.sha256((out / n).read_bytes()).hexdigest() for n in ("series.mzfeat", "series_evid.mzfeat")}
    return got, sha


@needs
def test_peak_memory_of_each_phase_stays_under_the_recorded_value(tmp_path):
    pytest.importorskip("numpy")
    build_wasm(tmp_path)
    small = series(tmp_path, "piccolo")
    big = series(tmp_path, "completo")
    measured = {
        "byte": measure(tmp_path, small, 192, "piccolo")[0],
        "completo": measure(tmp_path, big, 192, "completo")[0],
    }
    blocks, sha_blocks = measure(tmp_path, big, BLOCKS_MB, "blocchi")
    measured["completo_blocchi"] = blocks
    # the way in slabs (small budget) gives the same bytes as the way in memory
    assert sha_blocks == measure(tmp_path, big, 192, "completo2")[1], "the slabs do not give the bytes of the way in memory"
    if os.environ.get("MZLAB_GUARDIE_CREA"):
        GUARD.write_text(json.dumps({"serie": "piccolo e completo (feat_mirato.py --seme 2 --scala 1.0); completo_blocchi = completo con budget di %d MB" % BLOCKS_MB,
                                     "soglia": "valore registrato + 10 %", **measured}, indent=1) + "\n", encoding="utf-8")
    want = json.loads(GUARD.read_text(encoding="utf-8"))
    for name, got in measured.items():
        assert set(got) == set(want[name]), (name, sorted(got), sorted(want[name]))
        over = {k: (got[k], want[name][k]) for k in got if got[k] > want[name][k] * SLACK}
        assert not over, f"{name}: peak of the linear memory above the recorded value + 10 %: {over}"


def test_a_new_exported_function_that_returns_a_whole_vec_is_refused(monkeypatch, tmp_path):
    import controlla
    assert controlla.regole_statiche() == []
    monkeypatch.setattr(controlla, "BYTE_API", tmp_path / "vuoto.txt")
    bad = controlla.regole_statiche()
    assert any("scanbin" in b for b in bad) and any("build_evidence" in b for b in bad), bad
