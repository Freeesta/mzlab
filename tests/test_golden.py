"""The Python engine against the golden data of tests/golden/ (see tools/golden.py)."""
import json
import math
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def test_golden_check():
    r = subprocess.run(
        [sys.executable, str(ROOT / "tools" / "golden.py"), "--controlla", "--skip-missing"],
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    assert r.returncode == 0, r.stdout + r.stderr


def _analytical_g_integral(lo_rt: float, hi_rt: float, c: float = 14.3, s: float = 0.06, tau: float = 0.04) -> float:
    a, b = lo_rt - c, hi_rt - c
    sqrt2 = math.sqrt(2)
    sqrthalfpi = math.sqrt(math.pi / 2)
    left = 0.0
    if a < 0:
        up = min(0.0, b)
        left = s * sqrthalfpi * (math.erf(up / (s * sqrt2)) - math.erf(a / (s * sqrt2)))
    right = 0.0
    if b > 0:
        low = max(0.0, a)
        beta = 0.15 / tau
        factor = math.exp(0.5 * (beta * s) ** 2)
        u_low = (low + beta * s**2) / (s * sqrt2)
        u_high = (b + beta * s**2) / (s * sqrt2)
        right = factor * s * sqrthalfpi * (math.erf(u_high) - math.erf(u_low))
    return left + right


def test_synthetic_analytic_area():
    p = ROOT / "tests" / "golden" / "sintetico_FullScan.json"
    assert p.exists(), f"{p} not found"
    data = json.loads(p.read_text(encoding="utf-8"))
    x364 = next(x for x in data["xic"] if x["mz"] == 364)
    peak = x364["peaks"][0]
    lo_rt = x364["rt"][peak["lo"]]
    hi_rt = x364["rt"][peak["hi"]]
    amplitude = 2.0e7 * math.exp(-0.035 * 15)
    analytical_area = amplitude * _analytical_g_integral(lo_rt, hi_rt, c=14.3, s=0.06, tau=0.04)
    rel_diff = abs(analytical_area - peak["area"]) / analytical_area
    assert rel_diff < 0.005, f"rel diff {rel_diff:.4f} exceeds 0.5% (analytical={analytical_area}, measured={peak['area']})"

