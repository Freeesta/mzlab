"""The msngrafo .wasm gives the same bytes as `mzlab-cli msn-graph` (the same core functions) on the public synthetic infusion, through Node.
Skipped without cargo, wasm-bindgen or node."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
MZML = ROOT / "tests" / "golden" / "ref" / "msntree_rotta" / "msn.mzML"
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8")


@pytest.fixture(scope="module")
def pkg(tmp_path_factory):
    d = tmp_path_factory.mktemp("msngrafo")
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-msngrafo-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), "target/wasm32-unknown-unknown/release/mzlab_msngrafo_wasm.wasm")
    run("cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-core", "--example", "mzidx", "--", str(MZML), str(d / "msn.mzidx"))
    return d


@pytest.mark.parametrize("query,args", [
    ("", []),
    ("formula=C14H15N3O5%2B&ppm_formula=10", ["--formula", "C14H15N3O5+"]),
    ("formula=C14H15N3O5%2B&scelte=%5B%7B%22mz%22%3A177.0546%2C%22formula%22%3A%22C9H8NO2%2B%22%7D%5D", ["--formula", "C14H15N3O5+", "--scelta", "177.0546=C9H8NO2+"]),
])
def test_wasm_and_cli_give_the_same_graph(pkg, query, args):
    got = run("node", str(ROOT / "tests" / "msngrafo_wasm_node.js"), str(pkg / "pkg"), str(pkg / "msn.mzidx"), str(MZML), query).stdout
    want = run("cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-cli", "--", "msn-graph", str(MZML), *args).stdout
    assert got + "\n" == want  # the command line ends its output with a newline
    assert '"formato":"msngrafo/1"' in got


def test_a_bad_number_is_a_keyed_error(pkg):
    r = subprocess.run(["node", str(ROOT / "tests" / "msngrafo_wasm_node.js"), str(pkg / "pkg"), str(pkg / "msn.mzidx"), str(MZML), "ppm_formula=abc"], capture_output=True, encoding="utf-8")
    assert r.returncode != 0 and "err.request.badNumber" in r.stderr


@pytest.mark.parametrize("fmt,clean", [("csv", False), ("mgf", False), ("msp", True), ("csv", True)])
def test_wasm_and_cli_give_the_same_export(pkg, fmt, clean):
    q = f"formato={fmt}" + ("&pulito=1" if clean else "")
    got = run("node", str(ROOT / "tests" / "msngrafo_wasm_node.js"), str(pkg / "pkg"), str(pkg / "msn.mzidx"), str(MZML), q, "export").stdout
    want = run("cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-cli", "--", "msn-graph", str(MZML), "--export", fmt, *(["--pulito"] if clean else [])).stdout
    assert got == want and got.strip()


def lib_call(pkg, query, lib):
    node = ["node", str(ROOT / "tests" / "msngrafo_wasm_node.js"), str(pkg / "pkg"), str(pkg / "msn.mzidx"), str(MZML), query, "lib", lib]
    return json.loads(run(*node).stdout)


def test_the_library_gets_an_entry_once(pkg, tmp_path):
    import json as _j
    q = "formula=C14H15N3O5%2B&ppm_formula=10"
    a = lib_call(pkg, q, "-")
    assert a["doppione"] is False and a["libreria"].startswith('{"formato":"msnlib/1"')
    f = tmp_path / "msnlib.json"
    f.write_text(a["libreria"], encoding="utf-8")
    again = lib_call(pkg, q, str(f))
    assert again["doppione"] is True and again["libreria"] == a["libreria"]
    # the file is valid JSON and its entry holds the graph the route gives
    doc = _j.loads(a["libreria"])
    got = run("node", str(ROOT / "tests" / "msngrafo_wasm_node.js"), str(pkg / "pkg"), str(pkg / "msn.mzidx"), str(MZML), q).stdout
    assert doc["voci"][0]["grafo"] == _j.loads(got) and doc["voci"][0]["id"] == "STD-0001"


def test_a_graph_without_an_ion_or_a_foreign_file_is_a_keyed_error(pkg, tmp_path):
    node = ["node", str(ROOT / "tests" / "msngrafo_wasm_node.js"), str(pkg / "pkg"), str(pkg / "msn.mzidx"), str(MZML)]
    r = subprocess.run(node + ["", "lib", "-"], capture_output=True, encoding="utf-8")
    assert r.returncode != 0 and "err.msnlib.noIon" in r.stderr
    bad = tmp_path / "x.json"
    bad.write_text("[1]", encoding="utf-8")
    r = subprocess.run(node + ["formula=C14H15N3O5%2B", "lib", str(bad)], capture_output=True, encoding="utf-8")
    assert r.returncode != 0 and "err.msnlib.bad" in r.stderr
