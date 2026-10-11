# TPMINE-PRIVATE
"""WP4 (public, synthetic): feature detection on a hand-made MS1 peak table, stitching of a wobbling ion, Fourier side lobes, long background,
alignment across files and the cap on the width of a group."""
import numpy as np
import pytest

from mzlab.reader.mzml import PeakTable
from tpmine.hr import features as FT

NS, DT_MIN = 1000, 0.01          # 1000 scans, 0.6 s apart: 10 minutes


def table(ions, rng, noise=3000):
    """ions: (mz, apex_rt_min, sigma_min, height, wobble_ppm, rt_min_visible, rt_max_visible). Returns a PeakTable sorted by m/z."""
    rt = np.arange(NS) * DT_MIN
    mzs, its, pos = [], [], []
    for mz, r0, sg, h, wob, lo, hi in ions:
        y = h * np.exp(-0.5 * ((rt - r0) / sg) ** 2)
        ok = (y > 2e4) & (rt >= lo) & (rt <= hi)
        k = np.flatnonzero(ok)
        mzs.append(mz * (1 + rng.normal(0, wob, len(k)) * 1e-6))
        its.append(y[k])
        pos.append(k)
    n = noise                                                               # single-scan noise peaks
    mzs.append(rng.uniform(100, 600, n)); its.append(rng.uniform(2e4, 9e4, n)); pos.append(rng.integers(0, NS, n))
    mz, it, p = np.concatenate(mzs), np.concatenate(its), np.concatenate(pos).astype(np.int32)
    o = np.argsort(mz)
    return PeakTable(rt=rt, scan_ids=np.arange(NS), mz=mz[o].astype(np.float32), inten=it[o].astype(np.float32), pos=p[o], tic=np.zeros(NS))


@pytest.fixture(scope="module")
def det():
    rng = np.random.default_rng(7)
    ions = [(300.1000, 5.0, 0.08, 5e6, 2.0, 0, 99),        # wobbles by +-2 ppm: crosses the borders of the 4 ppm channels
            (300.1030, 5.0, 0.08, 3e6, 0.3, 0, 99),        # 10 ppm away: another ion
            (450.2000, 3.0, 0.05, 6e5, 0.3, 0, 99),
            (500.0000, 7.0, 0.06, 5e8, 0.3, 0, 99),        # very intense ion ...
            (500.0200, 7.0, 0.06, 8e6, 0.3, 0, 99),        # ... its Fourier side lobe at +40 ppm, same apex
            (500.1500, 7.0, 0.06, 8e6, 0.3, 0, 99),        # +300 ppm: a real ion
            (250.0000, 5.0, 100.0, 5e6, 0.3, 0.5, 9.5),    # continuous intense background (900 scans)
            (210.0000, 2.0, 0.05, 1e5, 0.3, 0, 99)]        # below min_height: not a feature
    t = table(ions, rng)
    return FT.detect(t), t


def near(f, mz, tol_ppm=6):
    return np.flatnonzero(np.abs(f.mz - mz) <= mz * tol_ppm * 1e-6)


def test_wobbling_ion_is_one_feature(det):
    f, t = det
    i = near(f, 300.1000)
    assert len(i) == 1                                           # without stitching it would be cut at the channel borders
    i = i[0]
    assert abs(f.rt[i] - 5.0) < 0.03 and f.height[i] == pytest.approx(5e6, rel=0.02)
    assert f.left[i] < 4.8 and f.right[i] > 5.2 and f.npts[i] > 30
    area_true = 5e6 * 0.08 * np.sqrt(2 * np.pi) * 60                     # counts x s of the Gaussian
    assert f.area[i] == pytest.approx(area_true, rel=0.15)
    assert len(near(f, 300.1030, 3)) == 1                                # the neighbour 10 ppm away is separate


def test_floor_background_and_artefacts(det):
    f, _ = det
    assert len(near(f, 210.0)) == 0 and len(near(f, 450.2)) == 1
    bg = near(f, 250.0)
    assert len(bg) == 1 and f.background[bg[0]]
    assert not f.background[near(f, 300.1)[0]]
    side = near(f, 500.0200, 2)
    assert len(side) == 1 and f.artefact[side[0]]                    # +40 ppm, same apex, 60 times weaker
    assert not f.artefact[near(f, 500.15, 2)[0]] and not f.artefact[near(f, 500.0, 2)[0]]
    assert f.artefact.sum() == 1
    assert len(f.select(~f.artefact)) == len(f) - 1


def test_noise_is_not_a_feature(det):
    f, _ = det
    assert len(f) == 7                                               # 6 real ions + the continuous background; nothing from the 3000 noise peaks
    assert np.all(f.npts >= 5) and np.all(f.height >= 2e5)


def test_a_table_without_traces_gives_no_features():
    f = FT.detect(table([], np.random.default_rng(1)))              # a blank with nothing but single-scan noise
    assert len(f) == 0 and f.mz.shape == (0,)


def test_features_are_sorted_and_table_is_dropped(tmp_path):
    from mzlab.reader.mzml import Run
    import msn_synth as S
    p = S.write_msn(tmp_path / "x.mzML", S.caffeine_nodes())          # MS1 of 12 scans: too short to hold a feature, but the call must work and clear the cache
    r = Run(p)
    fe = FT.detect_file(r)
    assert len(fe) == 0 or np.all(np.diff(fe.mz) >= 0)
    assert r._tables == {}


def _feats(mz_rt_h, nscan=100):
    mz, rt, h = (np.array(v, float) for v in zip(*mz_rt_h))
    n = len(mz)
    return FT.Features(mz=mz, rt=rt, apex=(rt * 100).astype(int), area=h * 10, height=h, npts=np.full(n, 20), left=rt - 0.05, right=rt + 0.05,
                       background=np.zeros(n, bool), artefact=np.zeros(n, bool), dt=0.6)


def test_alignment_groups_and_tolerances():
    f0 = _feats([(300.1000, 5.00, 1e6), (450.2, 3.0, 2e6)])
    f1 = _feats([(300.1003, 5.03, 3e6), (300.1100, 5.0, 1e6)])              # drift of +1 ppm and +0.03 min; a different ion 33 ppm away
    f2 = _feats([(300.0998, 4.97, 2e6), (450.2, 3.0, 1e6), (450.2, 3.5, 1e6)])    # the same m/z at another RT is another group
    al = FT.align([f0, f1, f2])
    assert len(al) == 4
    i = np.flatnonzero(np.abs(al.mz - 300.1) < 0.0005)[0]
    assert (al.area[i] > 0).all() and al.area[i].tolist() == [1e7, 3e7, 2e7]
    assert al.apex_rt[i].tolist() == pytest.approx([5.0, 5.03, 4.97], abs=1e-5)
    assert al.left[i][0] == pytest.approx(4.95, abs=1e-5) and al.right[i][0] == pytest.approx(5.05, abs=1e-5)
    j = np.flatnonzero(np.abs(al.mz - 450.2) < 0.001)
    assert len(j) == 2 and sorted(al.rt[j].round(1).tolist()) == [3.0, 3.5]


def test_alignment_artefacts_are_left_out_and_largest_feature_wins():
    a = _feats([(300.1, 5.0, 1e6), (300.1, 5.02, 4e6)])                   # two features of one group in one file: the larger gives apex and area
    b = _feats([(400.0, 5.0, 1e6)])
    b.artefact[0] = True
    al = FT.align([a, b])
    assert len(al) == 1 and al.area[0, 0] == 4e7 and al.apex_rt[0, 0] == pytest.approx(5.02, abs=1e-5) and al.area[0, 1] == 0
    z = FT.align([_feats([(300.0, 1.0, 1e6)])], drop_artefacts=True)
    assert len(z) == 1


def test_group_width_is_capped():
    # a chain of ions 3 ppm apart (gaps below the 5 ppm limit) spanning 18 ppm: single linkage would make one group
    mz = 400.0 * (1 + np.arange(7) * 3e-6)
    f = _feats([(m, 5.0, 1e6) for m in mz])
    al = FT.align([f], max_ppm_span=10.0)
    assert len(al) >= 2 and np.ptp(np.log(al.mz)) * 1e6 < 18.5
    f2 = _feats([(300.0, 4.0 + 0.08 * k, 1e6) for k in range(8)])                # an RT chain of 0.08 min gaps, 0.56 min wide
    assert len(FT.align([f2], max_rt_span=0.25)) >= 3
    assert len(FT.align([f2], max_rt_span=5.0)) == 1
    g = np.array([0, 0, 0, 0, 1, 1])
    x = np.array([0.0, 1.0, 1.1, 5.0, 7.0, 7.2])
    out = FT._limit_span(x, g, 2.0)
    assert out.tolist() == [0, 0, 0, 1, 2, 2]
