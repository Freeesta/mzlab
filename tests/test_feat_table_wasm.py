"""The table bridge of mzFinder in WebAssembly (crates/mzlab-feat-table-wasm) gives the same bytes as the native example `tabella`: the view, the report, the CSV
full (with the words of mzmine too), the FBMN quantification table, the MGF and the SIRIUS `.ms` have the same SHA-256 through Node as natively. The series is the
synthetic one of tools/feat_mirato.py (profile `piccolo`), aligned by the native example `nativo`. Skipped without cargo, the wasm32 target, wasm-bindgen, node or numpy."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")
SPEC = {"normalizza": {"metodo": "somma"}, "filtra": [{"regola": "min_rilevati", "valore": 2}]}


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout


def samples_json(idx):
    rows = [{"file": Path(p).stem + ".mzML", "sha256": "00" * 32, "tipo": "sample", "tempo_min": None, "replica": 1, "gruppo": "", "concentrazione": None,
             "unita": None, "ordine_iniezione": i + 1, "istd": False, "note": ""} for i, p in enumerate(idx)]
    return json.dumps({"schema": "campioni/1", "campioni": rows})


@pytest.fixture(scope="module")
def built(tmp_path_factory):
    pytest.importorskip("numpy")
    root = tmp_path_factory.mktemp("table")
    ser = root / "ser"
    run("python3", "tools/feat_mirato.py", "--seme", "2", "--profilo", "piccolo", "--scala", "1.0", "--out", str(ser))
    idx = sorted(str(p) for p in ser.glob("*.mzidx"))
    run("cargo", "build", "--release", "--locked", "-q", "-p", "mzlab-feat-wasm", "--example", "nativo")
    run("cargo", "build", "--release", "--locked", "-q", "-p", "mzlab-feat-table-wasm", "--example", "tabella")
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-table-wasm")
    pkg = root / "pkg"
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(pkg), "target/wasm32-unknown-unknown/release/mzlab_feat_table_wasm.wasm")
    series = root / "series"
    series.mkdir()
    run("target/release/examples/nativo", str(series), *idx)
    (root / "campioni.json").write_text(samples_json(idx))
    (root / "spec.json").write_text(json.dumps(SPEC))
    return root, idx, pkg, series / "series.mzfeat"


@pytest.mark.parametrize("spec", ["-", "spec.json"])
def test_wasm_and_native_write_the_same_files(built, spec):
    root, idx, pkg, table = built
    args = [str(table), str(root / "campioni.json"), "-" if spec == "-" else str(root / spec), "1", *idx]
    outs = {}
    for name, cmd in (("nat", ["target/release/examples/tabella"]), ("wasm", ["node", str(ROOT / "tests/feat_table_node.js"), str(pkg)])):
        out = root / f"{name}-{spec}"
        out.mkdir()
        got = run(*cmd, str(out), *args)
        outs[name] = (got, out)
    assert outs["nat"][0] == outs["wasm"][0]
    nat, wasm = outs["nat"][1], outs["wasm"][1]
    for f in sorted(p.name for p in nat.iterdir()):
        assert (nat / f).read_bytes() == (wasm / f).read_bytes(), f
    full = (nat / "full.csv").read_text().splitlines()
    assert full[0].startswith("id,mz,rt,") and len(full) > 10
    assert (nat / "quant.csv").read_text().splitlines()[0].startswith("row ID,row m/z,row retention time")
    view = json.loads((nat / "view.json").read_text()) if spec != "-" else None
    if view:
        assert view["conteggi"]["tenute"] == len(view["righe"]) == len(full) - 1
    assert json.loads((nat / "qc.json").read_text())["schema"] == "qc/1"
    assert (nat / "features.csv").read_text().splitlines()[0].startswith("id;mz;rt;")
    assert len(json.loads((nat / "features.rows.json").read_text())["hojas"][0]["filas"]) == len(full)
    assert json.loads((nat / "report.json").read_text())["schema"] == "report/1"
