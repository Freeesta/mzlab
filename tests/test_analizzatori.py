"""Loading-screen analysers (mzlab/web/an-*.js): exact Orbitrap profile (Lambert W) and a smoke test of every analyser."""
import json
import math
import shutil
import subprocess
from pathlib import Path

import pytest

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def run(code):
    out = subprocess.run(["node", "-e", f"const o=require({str(WEB / 'an-orbi.js')!r});{code}"], check=True, capture_output=True, encoding="utf-8").stdout
    return json.loads(out)


def test_lambert_w_both_branches():
    ys = [-0.36, -0.3, -0.2, -0.1, -1e-3, -1e-8]
    r = run(f"console.log(JSON.stringify({ys}.map(y=>[o.lambertW0(y),o.lambertWm1(y)])))")
    for y, (w0, wm1) in zip(ys, r):
        assert w0 * math.exp(w0) == pytest.approx(y, rel=1e-9) and -1 <= w0 < 0
        assert wm1 * math.exp(wm1) == pytest.approx(y, rel=1e-9) and wm1 <= -1
    assert run("console.log(JSON.stringify([o.lambertW0(-Math.exp(-1)), o.lambertWm1(-Math.exp(-1))]))") == pytest.approx([-1, -1], abs=1e-6)


@pytest.mark.parametrize("r1,r2,Rm", [(6, 15, 17), (6, 15, 12), (20, 30, 10)])
def test_profile_satisfies_ln_x_minus_x(r1, r2, Rm):
    # every point of both electrodes obeys ln x - x = A(z) with x = r^2 / Rm^2, A(z) = A(0) - 2 z^2 / Rm^2; r(0) is r1 / r2; the profile is symmetric in z
    pts = run(f"const zs=[0,1,3,6,9,12];console.log(JSON.stringify(zs.map(z=>[z,o.radiusAt({r1},{Rm},z),o.radiusAt({r2},{Rm},z),o.radiusAt({r1},{Rm},-z)])))")
    assert pts[0][1] == pytest.approx(r1) and pts[0][2] == pytest.approx(r2)
    for r0, col in ((r1, 1), (r2, 2)):
        x0 = r0 ** 2 / Rm ** 2
        for p in pts:
            x = p[col] ** 2 / Rm ** 2
            assert math.log(x) - x == pytest.approx(math.log(x0) - x0 - 2 * p[0] ** 2 / Rm ** 2, abs=1e-9)
    for p in pts:
        assert p[3] == pytest.approx(p[1])


def test_inner_electrode_is_a_spindle_and_branches_split_at_rm():
    # r1 < Rm: the central electrode thins towards the tips; the outer electrode closes in if r2 < Rm and flares if r2 > Rm
    zs = [0, 4, 8, 12, 16]
    inner = run(f"console.log(JSON.stringify({zs}.map(z=>o.radiusAt(6,17,z))))")
    closing = run(f"console.log(JSON.stringify({zs}.map(z=>o.radiusAt(15,17,z))))")
    flaring = run(f"console.log(JSON.stringify({zs}.map(z=>o.radiusAt(15,12,z))))")
    assert all(a > b for a, b in zip(inner, inner[1:])) and all(a > b for a, b in zip(closing, closing[1:])) and all(a < b for a, b in zip(flaring, flaring[1:]))


def test_zat_inverts_radius_and_equal_potential_rm():
    r = run("const z=o.zAt(15,17,4);const rm=o.rmEqualPotential(6,15);console.log(JSON.stringify([o.radiusAt(15,17,z),o.levelA(6,rm),o.levelA(15,rm),rm]))")
    assert r[0] == pytest.approx(4)
    assert r[1] == pytest.approx(r[2]) and r[3] == pytest.approx(math.sqrt((225 - 36) / math.log(225 / 36)))   # log mean: both electrodes at the same potential (zero voltage)


def test_every_analyser_draws_finite_points():
    code = """
    global.window = global; const noop = new Proxy(function(){}, { get: () => noop, apply: () => noop, set: () => true });
    require('./an-util.js'); for (const f of require('fs').readdirSync('.').filter(f => /^an-.+\\.js$/.test(f) && f !== 'an-util.js')) require('./' + f);
    const names = Object.keys(window.LDAN), out = {};
    for (const k of names) {
      const an = window.LDAN[k]({ cx: noop, W: 420, H: 120, TAU: 2 * Math.PI, line: '#ccc', muted: '#888' }); let seen = 0, bad = 0;
      for (let t = 0; t < 30; t += 0.07) for (let j = 0; j < 3; j++) { const q = an.at(j, t); if (!q) continue; seen++; if (![q.x, q.y].every(Number.isFinite) || q.x < -5 || q.x > 425 || q.y < -5 || q.y > 125) bad++; }
      if (an.draw) an.draw(1); if (an.mid) an.mid(); out[k] = [seen > 20, bad];
    }
    console.log(JSON.stringify(out));
    """
    r = subprocess.run(["node", "-e", code], cwd=WEB, check=True, capture_output=True, encoding="utf-8").stdout
    res = json.loads(r)
    assert len(res) >= 7 and {"quad", "tof", "orbi"} <= set(res)
    assert all(v == [True, 0] for v in res.values()), res
