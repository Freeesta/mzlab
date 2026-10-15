"""Pure parts of the sight on a difference map (mirino.js): A − B of two spectra (peak by peak) and of two traces (B interpolated on the RT of A)."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

JS = Path(__file__).resolve().parents[1] / "mzlab" / "web" / "mirino.js"
needs_node = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def run(expr):
    code = f"const o=require({str(JS)!r});console.log(JSON.stringify({expr}))"
    return json.loads(subprocess.run(["node", "-e", code], check=True, capture_output=True, encoding="utf-8").stdout)


@needs_node
def test_diff_peaks_signs_and_unmatched():
    r = run("o.diffPeaks([100.0, 200.1, 300.0],[10,50,5],[100.2, 200.0, 400.0],[4,60,7], m=>0.5)")
    assert r["mz"] == [100.0, 200.1, 300.0, 400.0]
    assert r["y"] == [6, -10, 5, -7]                       # more in A (+), more in B (−), only in A, only in B


@needs_node
def test_diff_peaks_one_b_peak_is_used_once():
    r = run("o.diffPeaks([100.0, 100.3],[10,8],[100.1],[9], m=>0.5)")
    assert sorted(r["y"]) == [1, 8]                        # the single B peak is taken away from one A peak only


@needs_node
def test_diff_trace_interpolates_b_and_is_zero_outside():
    r = run("o.diffTrace([0,1,2,3],[10,10,10,10],[1,3],[0,10])")
    assert r["y"] == [10, 10, 5, 0]                        # B: out of range = 0, 0 at RT 1, 5 at RT 2, 10 at RT 3
    assert r["rt"] == [0, 1, 2, 3]
