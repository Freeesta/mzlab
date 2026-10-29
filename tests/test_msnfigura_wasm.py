"""The msn.figure of the native side (`mzlab-cli msn-graph figure`, Rust SVG) and of the browser (the page drawer on the graph the .wasm gives) show the same
ion and loss texts on the public synthetic infusion. The browser has no .wasm for the figure (the page draws it in JS), so the parity is on the labels, not on the bytes
of the SVG; the graph itself is byte-equal in both sides (tests/test_msngrafo_wasm.py). Skipped without cargo, wasm-bindgen or node."""
import collections
import html
import json
import re
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
MZML = ROOT / "tests" / "golden" / "ref" / "msntree_rotta" / "msn.mzML"
FORMULA = "C14H15N3O5+"
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8")


@pytest.fixture(scope="module")
def graph(tmp_path_factory):
    d = tmp_path_factory.mktemp("msnfigura")
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-msngrafo-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), "target/wasm32-unknown-unknown/release/mzlab_msngrafo_wasm.wasm")
    run("cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-core", "--example", "mzidx", "--", str(MZML), str(d / "msn.mzidx"))
    # the graph the browser gets: the .wasm answer, with its `vista`
    g = run("node", str(ROOT / "tests" / "msngrafo_wasm_node.js"), str(d / "pkg"), str(d / "msn.mzidx"), str(MZML), "formula=C14H15N3O5%2B&ppm_formula=10").stdout
    (d / "g.json").write_text(g, encoding="utf-8")
    return d


@pytest.mark.parametrize("vista,mode", [("albero", "tree"), ("radiale", "radial")])
def test_the_native_figure_and_the_page_drawer_show_the_same_texts(graph, vista, mode):
    run("cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-cli", "--", "msn-graph", "figure", str(graph / "g.json"), "--vista", vista, "--out", str(graph / vista))
    svg = (graph / (vista + ".svg")).read_text(encoding="utf-8")
    rust = collections.Counter(html.unescape(re.sub(r"<[^>]+>", "", t)) for t in re.findall(r"<text[^>]*>(.*?)</text>", svg))  # the subscripts are tspans
    page = collections.Counter(json.loads(run("node", str(ROOT / "tests" / "msn_figura_wasm_node.js"), str(graph / "g.json"), mode).stdout))
    assert sum(page.values()) >= 4
    missing = {t: n for t, n in page.items() if rust[t] < n}
    assert not missing, f"texts of the page drawer absent from the Rust SVG: {missing}"
