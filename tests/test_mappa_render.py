"""The pure parts of the RT x m/z map drawing (mzlab/web/mappa-render.js, run with node): the «Liscia» splat of a zoomed region and the choice of the m/z labels
of the side spectrum (S/N of each peak over the noise around it, a quota of labels for every slice of the window)."""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import numpy as np
import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import confronta_etichette as ce  # noqa: E402

JS = ROOT / "mzlab" / "web" / "mappa-render.js"
needs_node = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def node(body, data):
    code = f"const R=require({json.dumps(str(JS))});const d=JSON.parse(require('fs').readFileSync(0,'utf8'));{body}"
    return json.loads(subprocess.run(["node", "-e", code], input=json.dumps(data), capture_output=True, text=True, encoding="utf-8", check=True).stdout)


@needs_node
def test_ref_is_the_995th_percentile_of_the_nonzero_values():
    rng = np.random.default_rng(3)
    for n in (50, 40000):                                   # under and over the 10 000 sample
        v = np.where(rng.random(n) < 0.5, 0.0, -rng.lognormal(3, 1, n))
        got = node("console.log(JSON.stringify(R.ref(Float32Array.from(d.v))))", {"v": v.tolist()})
        a = np.sort(np.abs(v[v != 0]))
        assert got == pytest.approx(float(a[min(len(a) - 1, int(len(a) * 0.995))]), rel=0.05 if n > 10000 else 1e-5)
    assert node("console.log(JSON.stringify(R.ref([0,0])))", {}) == 1                            # nothing to scale: 1


@needs_node
def test_splat_keeps_height_and_joins_lines():
    nrt, nmz = 40, 60
    v = np.zeros((nrt, nmz)); v[10:30, 20] = 5.0; v[10:30, 27] = 3.0        # two thin lines in m/z (a centroid per scan, 7 bins apart)
    out = np.array(node("console.log(JSON.stringify(Array.from(R.splat(Float32Array.from(d.v),d.nrt,d.nmz,0,d.sy))))",
                        {"v": v.ravel().tolist(), "nrt": nrt, "nmz": nmz, "sy": 3.0})).reshape(nrt, nmz)
    assert out[:, 20].max() == pytest.approx(5.0) and out[:, 27].max() == pytest.approx(3.0)       # heights are those of the matrix
    assert (out[10:30, 23:25] > 0).all()                                                            # the gap between the two lines is bridged
    assert out[:, 20].tolist() == [5.0 if 10 <= i < 30 else 0.0 for i in range(nrt)]
    assert out[5, 20] == 0                                                                           # nothing spreads along RT when sx = 0
    same = node("console.log(JSON.stringify(R.splat([1,2,3],3,1,0.1,0.1)))", {})
    assert same == [1, 2, 3]                                                                         # kernels under a third of a bin: the matrix itself


@needs_node
def test_splat_difference_keeps_both_signs():
    v = np.zeros((5, 30)); v[2, 10] = 4.0; v[2, 14] = -6.0
    out = np.array(node("console.log(JSON.stringify(Array.from(R.splat(Float32Array.from(d.v),5,30,0,2))))", {"v": v.ravel().tolist()})).reshape(5, 30)
    assert out[2, 10] == pytest.approx(4.0) and out[2, 14] == pytest.approx(-6.0)
    assert out[2, 11] > 0 and out[2, 13] < 0                                                         # each sign spreads on its own side


@needs_node
def test_small_peak_at_high_mz_is_labelled():
    rng = np.random.default_rng(3)
    mz, y, kind = ce.scene(rng, targets=1)
    res = node("console.log(JSON.stringify(R.pick(d.mz,d.y,d.o).map(c=>c.i)))", {"mz": mz, "y": y, "o": {"lo": 120, "hi": 480, "bands": 4, "quota": 1}})
    t = kind.index("t")
    assert t in res[:8] and y[t] < 0.05 * max(y)                  # under 5 % of the highest peak: the old rule never labelled it


@needs_node
def test_active_peak_always_first_and_isotopes_are_not_labelled():
    mz = [150.0, 151.0, 152.0, 300.0, 301.0, 420.0] + list(np.linspace(130, 470, 40) + 0.37)
    y = [1e7, 1.6e6, 5e5, 2e5, 3e4, 4e5] + [3e3] * 40
    res = node("console.log(JSON.stringify(R.pick(d.mz,d.y,d.o).map(c=>[c.i,!!c.act])))", {"mz": mz, "y": y, "o": {"lo": 120, "hi": 480, "act": [300.5, 301.5]}})
    assert res[0] == [4, True]                                    # the peak of the sight (301.0), small as it is, comes first
    labelled = [i for i, _ in res]
    assert 1 not in labelled and 2 not in labelled                # M+1 / M+2 of the strongest peak
    assert 5 in labelled                                          # 420: far from the strong ones, well over the noise


@needs_node
def test_strategy_with_quota_beats_the_others():
    r = ce.run(n=200)
    assert r["bands"]["recall"] >= 0.95 and r["abs"]["recall"] == 0
    assert r["bands"]["recall"] > r["snr"]["recall"] + 0.1
    assert r["bands"]["noise_labels"] < 0.2 and r["bands"]["coverage"] > r["snr"]["coverage"]


@needs_node
def test_noise_is_local():
    mz = list(np.linspace(100, 200, 60)) + list(np.linspace(300, 400, 60)) + [150.3, 350.3]
    y = [100.0] * 60 + [10000.0] * 60 + [2000.0, 2000.0]            # a quiet region and a noisy one: the same peak is significant in one and not in the other
    res = node("console.log(JSON.stringify(R.snrOf(d.mz,d.y,0,500,{band:60}).map(c=>[c.i,c.snr])))", {"mz": mz, "y": y})
    s = dict((i, v) for i, v in res)
    assert s[120] == pytest.approx(20, rel=0.01) and s[121] == pytest.approx(0.2, rel=0.01)


@needs_node
def test_snr_matches_the_sorted_definition_and_is_fast_on_a_full_spectrum():
    # the noise of a peak: median + 1.4826 MAD of the peaks within the band, those within 2.5 Da of it left out (spelled out with sorts, as the definition)
    rng = np.random.default_rng(11)
    n = 5000
    mz = np.sort(rng.uniform(100, 1000, n)); y = np.exp(rng.uniform(0, 10, n)) * rng.choice([-1, 1], n)
    lo, hi = 150.0, 900.0
    got = node("const t=Date.now();const r=R.snrOf(d.mz,d.y,d.lo,d.hi);console.log(JSON.stringify({ms:Date.now()-t,r}))", {"mz": mz.tolist(), "y": y.tolist(), "lo": lo, "hi": hi})
    a = np.abs(y); floor = max(a.max() * 1e-4, 1e-12); H = max(15, (hi - lo) / 4)

    def level(v):
        m = np.median(v); return m + 1.4826 * np.median(np.abs(v - m))
    for c in got["r"][::97]:
        k = c["i"]; w = (mz >= mz[k] - H) & (mz < mz[k] + H + 1e-9) & (np.abs(mz - mz[k]) > 2.5)
        noise = max(level(a[w]), floor)
        assert c["noise"] == pytest.approx(noise, rel=1e-12) and c["snr"] == pytest.approx(a[k] / noise, rel=1e-12)
    assert got["ms"] < 250                                           # a pass over every peak of a scan stays far under the 2 s of sorting each band
