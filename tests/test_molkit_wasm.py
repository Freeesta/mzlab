"""The molkit .wasm (crates/mzlab-mol-wasm) gives the `check/1` and the `cip/1` of a drawing byte for byte like `mzlab-cli mol-check` and
`mol-cip` and the app fixtures (tests/dati/check_disegno.json and cip_disegno.json, read by crates/mzlab-desktop/tests/labels.rs too), and a bad
molfile as a keyed JSON error; the `subdb/1` of a structure list like `mzlab-cli mol-search --db` and the app (tests/dati/sub_db.json); the Flow
step `structure.moiety` (`flow_moiety`) like `mzlab-cli flow run --set` (tests/dati/moiety.json); the `hilite/1` of the evidence colours like
`mzlab-cli mol-hilite` and the app (tests/dati/hilite_disegno.json). Skipped without cargo, wasm-bindgen or node."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout


@pytest.fixture(scope="module")
def pkg(tmp_path_factory):
    out = tmp_path_factory.mktemp("molkit")
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-mol-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(out), "target/wasm32-unknown-unknown/release/mzlab_mol_wasm.wasm")
    return out


def test_check_through_wasm_equals_the_cli_and_the_app_fixture(pkg, tmp_path):
    fx = json.loads((ROOT / "tests" / "dati" / "check_disegno.json").read_text(encoding="utf-8"))
    req = tmp_path / "req.json"
    req.write_text(json.dumps(fx["request"]), encoding="utf-8")
    site = run("node", "tests/molkit_wasm_node.js", str(pkg), req.read_text(encoding="utf-8"))
    assert site == run("cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-cli", "--", "mol-check", str(req))
    assert site.strip() == fx["check"]
    assert [w["code"] for w in json.loads(site)["warnings"]] == ["valence", "valenceCharge", "isotope", "wedgeNoCentre", "pieces"]


def test_a_bad_molfile_is_a_keyed_error(pkg):
    r = subprocess.run(["node", "tests/molkit_wasm_node.js", str(pkg), json.dumps({"molfile": "nonsense"})], cwd=ROOT, capture_output=True, encoding="utf-8")
    assert r.returncode == 1 and json.loads(r.stderr)["error_key"] == "error.check.molfile"


def test_cip_through_wasm_equals_the_cli_and_the_app_fixture(pkg, tmp_path):
    fx = json.loads((ROOT / "tests" / "dati" / "cip_disegno.json").read_text(encoding="utf-8"))
    req = tmp_path / "req.json"
    req.write_text(json.dumps(fx["request"]), encoding="utf-8")
    site = run("node", "tests/molkit_wasm_node.js", str(pkg), req.read_text(encoding="utf-8"), "cip")
    assert site == run("cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-cli", "--", "mol-cip", str(req))
    assert site.strip() == fx["cip"]
    assert json.loads(site) == {"format": "cip/1", "atoms": [{"atom": 1, "label": "R"}], "bonds": [{"a": 3, "b": 4, "label": "E"}]}
    r = subprocess.run(["node", "tests/molkit_wasm_node.js", str(pkg), json.dumps({"molfile": "nonsense"}), "cip"], cwd=ROOT, capture_output=True, encoding="utf-8")
    assert r.returncode == 1 and json.loads(r.stderr)["error_key"] == "error.cip.molfile"


def test_substructure_over_a_list_through_wasm_equals_the_cli_and_the_fixture(pkg, tmp_path):
    fx = json.loads((ROOT / "tests" / "dati" / "sub_db.json").read_text(encoding="utf-8"))
    for k, c in enumerate(fx["cases"]):
        r = c["request"]
        site = run("node", "tests/molkit_wasm_node.js", str(pkg), json.dumps(r), "sub")
        assert site.strip() == c["answer"], c["name"]
        db = tmp_path / f"list{k}.txt"
        db.write_text(r["db"], encoding="utf-8")
        sets = (["--set", "max=1"] if "max" in r else []) + (["--set", "stereo=1"] if r.get("stereo") else [])
        assert site == run("cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-cli", "--", "mol-search", "--sub", r["query"], "--db", str(db), *sets)
    bad = subprocess.run(["node", "tests/molkit_wasm_node.js", str(pkg), json.dumps({"query": "C", "db": "# none\n"}), "sub"], cwd=ROOT, capture_output=True, encoding="utf-8")
    assert bad.returncode == 1 and json.loads(bad.stderr)["error_key"] == "error.sub.db"


def test_moiety_step_through_wasm_equals_flow_run_with_set_and_the_fixture(pkg, tmp_path):
    fx = json.loads((ROOT / "tests" / "dati" / "moiety.json").read_text(encoding="utf-8"))
    table = tmp_path / "list.csv"
    table.write_bytes(fx["table"].encode("utf-8"))
    plan = tmp_path / "plan.json"
    plan.write_text(json.dumps({"format": "mzflow/2", "inputs": [{"name": "list.csv", "type": "candidates"}],
                                "steps": [{"id": "m", "step": "structure.moiety@1", "in": {"candidates": "file:list.csv"}, "params": {}}]}), encoding="utf-8")
    defaults = {"query": "", "column": "smiles", "stereo": False, "keep": "containing"}
    for k, c in enumerate(fx["cases"]):
        params = json.loads(c["params"])
        site = run("node", "tests/molkit_wasm_node.js", str(pkg), json.dumps({**defaults, **params}), "moiety", str(table))
        assert site == c["out"], c["params"]
        sets = [a for n, v in params.items() for a in ("--set", f"m.{n}={json.dumps(v) if isinstance(v, bool) else v}")]
        out = tmp_path / f"run{k}"
        run("cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(plan), "--in", str(table), "--out", str(out), *sets)
        assert (out / "m.csv").read_text(encoding="utf-8") == site, c["params"]
    for c in fx["errors"]:
        bad = subprocess.run(["node", "tests/molkit_wasm_node.js", str(pkg), c["params"], "moiety", str(table)], cwd=ROOT, capture_output=True, encoding="utf-8")
        assert bad.returncode == 1 and json.loads(bad.stderr)["error_key"] == c["key"], c["params"]


def test_hilite_through_wasm_equals_the_cli_and_the_app_fixture(pkg, tmp_path):
    fx = json.loads((ROOT / "tests" / "dati" / "hilite_disegno.json").read_text(encoding="utf-8"))
    for req_key, want in (("request", "hilite"), ("lr", "hilite_lr")):
        req = tmp_path / (req_key + ".json")
        req.write_text(json.dumps(fx[req_key]), encoding="utf-8")
        site = run("node", "tests/molkit_wasm_node.js", str(pkg), req.read_text(encoding="utf-8"), "hilite")
        assert site == run("cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-cli", "--", "mol-hilite", str(req))
        assert site.strip() == fx[want]
    for bad, key in fx["errors"]:
        r = subprocess.run(["node", "tests/molkit_wasm_node.js", str(pkg), json.dumps(bad), "hilite"], cwd=ROOT, capture_output=True, encoding="utf-8")
        assert r.returncode == 1 and json.loads(r.stderr)["error_key"] == key
