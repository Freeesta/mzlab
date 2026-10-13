"""LAYOUT.geometria (layout.js): rows side by side, shares, minimum widths and the stacked view of a narrow window. Skipped without node."""
import json, shutil, subprocess
from pathlib import Path
import pytest

ROOT = Path(__file__).resolve().parent.parent
pytestmark = pytest.mark.skipif(not shutil.which("node"), reason="node not available")


@pytest.fixture(scope="module")
def g():
    r = subprocess.run(["node", str(ROOT / "tests/layout_geometria.js"), str(ROOT / "mzlab/web/layout.js")], capture_output=True, encoding="utf-8", check=True)
    return json.loads(r.stdout)


def widths(c): return [(r["x"], r["w"]) for r in c["rett"]]


def test_two_side_by_side(g):
    for W in ("930", "1280", "1920"):
        (x0, w0), (x1, w1) = widths(g[W])
        assert x0 == 0 and x1 == w0 + 6 and x1 + w1 == int(W) and not g[W]["impilata"][0]
        assert {r["h"] for r in g[W]["rett"]} == {300} and g[W]["H"] == 300       # one height per row


def test_narrow_window_stacks(g):
    c = g["800"]
    assert c["impilata"] == [True] and [(r["x"], r["w"], r["y"]) for r in c["rett"]] == [(0, 800, 0), (0, 800, 306)]


def test_shares_keep_the_minimum(g):
    (_, w0), (_, w1) = widths(g["quote"])
    assert w1 == 440 and w0 + w1 + 6 == 1280


def test_three_fit_only_when_wide(g):
    assert not g["tre"]["impilata"][0] and sum(w for _, w in widths(g["tre"])) + 12 == 1920
    assert g["tre930"]["impilata"] == [True]


def test_single_rows_and_gaps(g):
    assert [(r["y"], r["w"]) for r in g["pila"]["rett"]] == [(0, 1000), (106, 1000), (186, 1000)]
