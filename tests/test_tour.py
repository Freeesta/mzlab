"""Pure unit tests for the tour dialog positioning functions (mzlab/web/tour.js)."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

JS = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "tour.js"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def run(code):
    out = subprocess.run(["node", "-e", f"globalThis.NOMI=require({str(JS.with_name('nomi.js'))!r});const t=require({str(JS)!r});{code}"], check=True, capture_output=True, encoding="utf-8").stdout
    return json.loads(out)


def test_narrow_viewport_docks_bottom():
    # Width < 700: docked at bottom
    res = run("console.log(JSON.stringify(t.computePosition({top:100,left:50,right:250,bottom:200,width:200,height:100}, 320, 160, 600, 800)))")
    assert res["side"] == "bottom-dock"
    assert res["left"] == (600 - 320) // 2
    assert res["top"] == 800 - 160 - 12


def test_fits_below_target():
    # Target near top-left, plenty of room below
    res = run("console.log(JSON.stringify(t.computePosition({top:50,left:100,right:300,bottom:150,width:200,height:100}, 320, 160, 1200, 800)))")
    assert res["side"] == "bottom"
    assert res["top"] == 150 + 12
    assert res["left"] == 100


def test_fits_above_target():
    # Target near bottom, no room below, plenty of room above
    res = run("console.log(JSON.stringify(t.computePosition({top:650,left:100,right:300,bottom:750,width:200,height:100}, 320, 160, 1200, 800)))")
    assert res["side"] == "top"
    assert res["top"] == 650 - 12 - 160


def test_fits_right_target():
    # Target spans vertically so bottom/top don't fit, but right fits
    res = run("console.log(JSON.stringify(t.computePosition({top:20,left:10,right:110,bottom:700,width:100,height:680}, 320, 160, 1200, 720)))")
    assert res["side"] == "right"
    assert res["left"] == 110 + 12


def test_fits_left_target():
    # Target spans vertically on the right side, so bottom/top/right don't fit, left fits
    res = run("console.log(JSON.stringify(t.computePosition({top:20,left:1000,right:1180,bottom:700,width:180,height:680}, 320, 160, 1200, 720)))")
    assert res["side"] == "left"
    assert res["left"] == 1000 - 12 - 320
