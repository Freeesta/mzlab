"""Shared formula / name / molecule-SVG helpers (mzlab/web/molui.js), run with node."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

JS = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "molui.js"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def js(expr):
    code = f"const M=require({str(JS)!r});console.log(JSON.stringify({expr}));"
    return json.loads(subprocess.run(["node", "-e", code], check=True, capture_output=True, text=True, encoding="utf-8").stdout)


def test_same_composition_other_order_is_ok():
    r = js('M.compare("C14H13F4N3O2S","C14H13N3O2F4S")')
    assert r["ok"] is True and r["a"] == r["b"] == "C14H13F4N3O2S" and r["diffs"] == []
    assert js('M.mismatchText("C14H13F4N3O2S","C14H13N3O2F4S")') == ""


def test_different_composition_reports_the_element():
    t = js('M.mismatchText("C14H13F4N3O2S","C14H13N3O2F3S")')
    assert t == "The formula (C14H13F4N3O2S) does not match the SMILES (C14H13F3N3O2S): F: formula 4, SMILES 3."
    assert "S: formula 1, SMILES 0" in js('M.mismatchText("C2H6OS","C2H6O")')


def test_hill_order():
    assert js('M.hillOf("N3O2F4SC14H13")') == "C14H13F4N3O2S"
    assert js('M.hillOf("NaCl")') == "ClNa"                  # no carbon: alphabetical
    assert js('M.hillOf("H2O")') == "H2O"
    assert js('M.hillOf("CH4")') == "CH4"


def test_parentheses_hydrates_charges():
    assert js('M.hillOf("Ca(OH)2")') == "CaH2O2"
    assert js('M.hillOf("CuSO4·5H2O")') == "CuH10O9S"
    assert js('M.hillOf("C2H5O+")') == "C2H5O"
    assert js('M.compare("C(OH)2","CH2O2").ok') is True
    assert js('M.compare("C2H6","not a formula").ok') is None and js('M.mismatchText("C2H6","??")') == ""


def test_capitalise_first_letter_only():
    assert js('M.cap("aspirin")') == "Aspirin"
    for keep in ("N-Methylformamide", "DEET", "17α-estradiol", "2-aminophenol", "α-Pinene", ""):
        assert js(f"M.cap({json.dumps(keep)})") == keep
    assert js('M.cap("tert-butyl alcohol")') == "Tert-butyl alcohol"


def test_svg_themed_and_standalone():
    raw = ('<svg id="mol1" viewBox="0 0 10 10"><text fill="rgb(255,13,13)">O</text><line stroke="rgb(0,0,0)"/><text fill="rgb(0,0,0)">C</text></svg>')
    t = js(f"M.svg({json.dumps(raw)})")
    assert 'stroke="currentColor"' in t and 'class="mol-o"' in t and "rgb(0,0,0)" not in t and 'class="molsvg"' in t
    light = js(f'M.svg({json.dumps(raw)},"light")')
    dark = js(f'M.svg({json.dumps(raw)},"dark")')
    assert 'stroke="#000000"' in light and 'fill="#ffffff"' in light and "currentColor" not in light
    assert 'stroke="#e4e7eb"' in dark and 'fill="#14171b"' in dark


def lum(h):
    c = [int(h[i:i + 2], 16) / 255 for i in (1, 3, 5)]
    c = [x / 12.92 if x <= .03928 else ((x + .055) / 1.055) ** 2.4 for x in c]
    return .2126 * c[0] + .7152 * c[1] + .0722 * c[2]


def ratio(a, b):
    x, y = sorted((lum(a), lum(b)), reverse=True)
    return (x + .05) / (y + .05)


def test_heteroatom_colours_readable_on_both_themes():
    pal = js("{L:M.LIGHT,D:M.DARK}")
    for k, v in pal["L"].items():
        assert ratio(v, "#ffffff") >= 3, k
    for k, v in pal["D"].items():
        assert ratio(v, "#1d2127") >= 3 and ratio(v, "#14171b") >= 3, k
