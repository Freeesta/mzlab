"""The registry of readers in WebAssembly (`LettoreWasm` of mzlab-wasm, through Node): the same synthetic Agilent `.D` folder as `test_agilent_wasm.py`
is recognised from its members (names, sizes, a read callback) and answers the index, a scan and the `metodo/1` document. Skipped without cargo, wasm-bindgen or node."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

from test_agilent_wasm import SCANS, ROOT, run, write_folder

pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


@pytest.fixture(scope="module")
def pkg(tmp_path_factory):
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm")
    out = tmp_path_factory.mktemp("pkg")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(out), "target/wasm32-unknown-unknown/release/mzlab_wasm.wasm")
    return out


def answer(pkg, d: Path, *extra):
    return json.loads(run("node", str(ROOT / "tests/lettore_wasm_node.js"), str(pkg), str(d), *extra))


def test_the_registry_recognises_the_folder_and_reads_a_scan(pkg, tmp_path):
    a = answer(pkg, write_folder(tmp_path))
    assert a["formato"] == "agilent-d"
    kept = [s for s in SCANS if s[1]]
    assert a["campioni"][0]["scansioni"] == len(kept)
    assert [round(v["rt"], 4) for v in a["indice"]] == [s[0] for s in kept]
    t, pts = kept[2]
    assert a["scan2"]["mz"] == [p[0] for p in pts] and a["scan2"]["it"] == [p[1] for p in pts]
    assert a["metodo"]["schema"]


def test_a_folder_that_no_reader_knows_is_an_error_with_a_key(pkg, tmp_path):
    (tmp_path / "x.D").mkdir()
    (tmp_path / "x.D" / "a.txt").write_text("nothing")
    r = subprocess.run(["node", str(ROOT / "tests/lettore_wasm_node.js"), str(pkg), str(tmp_path / "x.D")], cwd=ROOT, capture_output=True, encoding="utf-8")
    assert r.returncode != 0 and "err.file.notAccepted" in r.stderr
