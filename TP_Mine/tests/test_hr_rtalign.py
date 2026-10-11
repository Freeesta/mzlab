# TPMINE-PRIVATE
"""WP4c (public, synthetic): the RT alignment of a series: anchors, shift curves, recovery of a known drift, the map that goes to the panel."""
import numpy as np
import pytest

from tpmine.hr import features as FT
from tpmine.hr import rtalign as RA

DRIFT = [0.0, 0.07, -0.05, 0.03, 0.12, -0.02]            # minutes, constant; the median of the files is 0.015


def _feats(rt_mz_h, drift, nscan=100):
    """One Features per file: the ions (m/z, rt, height) with the drift of the file added to the RT."""
    out = []
    for d in drift:
        mz, rt, h = (np.array(v, float) for v in zip(*rt_mz_h))
        n = len(mz)
        out.append(FT.Features(mz=mz, rt=rt + d, apex=(rt * 100).astype(int), area=h * 10, height=h, npts=np.full(n, 20), left=rt + d - 0.05, right=rt + d + 0.05,
                               background=np.zeros(n, bool), artefact=np.zeros(n, bool), dt=0.6))
    return out


def _ions(n=40, seed=3):
    rng = np.random.default_rng(seed)
    return [(150.0 + 7.3 * i + rng.uniform(0, 0.5), 0.5 + 8.5 * i / n + rng.uniform(0, 0.1), 5e6) for i in range(n)]


def test_known_drift_is_recovered_within_0_02_min():
    al = FT.align(_feats(_ions(), DRIFT))
    m = RA.build(al)
    assert m.ok and m.n_anchors == 40
    truth = np.array(DRIFT) - np.median(DRIFT)                       # the shift is against the median of the files
    for rt in (0.8, 3.0, 5.5, 8.5):
        got = np.array([m.at(k, rt) for k in range(len(DRIFT))])
        assert np.abs(got - truth).max() <= 0.02, (rt, got, truth)
    assert m.rms.max() <= 0.02
    assert [round(m.expected(5.0, k) - 5.0, 3) for k in range(6)] == pytest.approx(truth, abs=0.02)


def test_drift_that_changes_along_the_run_is_followed():
    ions = _ions(60)
    out = []
    for k, (a, b) in enumerate([(0.0, 0.0), (0.04, 0.0), (0.0, 0.008), (-0.03, 0.004), (0.02, -0.006)]):         # shift = a + b * RT
        mz, rt, h = (np.array(v, float) for v in zip(*ions))
        d = a + b * (rt - 5.0) * 2
        out.append((mz, rt + d, h))
    feats = [FT.Features(mz=mz, rt=rt, apex=(rt * 100).astype(int), area=h * 10, height=h, npts=np.full(len(mz), 20), left=rt - 0.05, right=rt + 0.05,
                         background=np.zeros(len(mz), bool), artefact=np.zeros(len(mz), bool), dt=0.6) for mz, rt, h in out]
    m = RA.build(FT.align(feats))
    truth = lambda k, rt: np.array([[0.0, 0.0], [0.04, 0.0], [0.0, 0.008], [-0.03, 0.004], [0.02, -0.006]])[k] @ np.array([1.0, (rt - 5.0) * 2])    # noqa: E731
    med = lambda rt: np.median([truth(k, rt) for k in range(5)])                                                                    # noqa: E731
    for rt in (2.0, 3.0, 5.0, 7.0, 8.0):
        for k in range(5):
            assert abs(m.at(k, rt) - (truth(k, rt) - med(rt))) <= 0.02


def test_crowded_weak_or_inconsistent_ions_are_not_anchors():
    ions = _ions(12)
    ions += [(ions[0][0] * (1 + 1e-6), ions[0][1] + 0.45, 5e6)]      # a neighbour of the first ion (same m/z, 0.45 min later): both are crowded
    ions += [(900.0, 4.0, 5e5)]                                      # weak: under ten times the detection height
    al = FT.align(_feats(ions, DRIFT))
    iso = RA.isolated(al)
    assert not iso[np.flatnonzero(np.abs(al.mz - ions[0][0]) < 0.01)].any() and iso.sum() == len(al) - 2
    assert RA.build(al).n_anchors == 11                              # 14 groups: the crowded pair and the weak ion are left out
    f = _feats(_ions(12), DRIFT)
    f[2].rt[3] += 1.0                                                # one ion that lands 1 minute away in a file is another peak: not an anchor
    f[2].apex[3] += 100
    assert RA.build(FT.align(f)).n_anchors <= 11


def test_too_few_anchors_give_an_empty_map_that_shifts_nothing():
    m = RA.build(FT.align(_feats(_ions(2), DRIFT)))
    assert not m.ok and m.at(1, 3.0) == 0.0 and m.as_dict(["a"] * 6)["rt"] == []
    assert not RA.build(FT.align(_feats(_ions(20), [0.0]))).ok       # one file: nothing to align


def test_anchors_missing_in_a_file_still_anchor_the_others():
    f = _feats(_ions(30), DRIFT)
    for k in range(3, 6):                                            # a file or two lack some ions; the strict set (all files) is under MIN_ANCHORS
        keep = np.arange(len(f[k].mz)) % 4 != 0
        f[k] = f[k].select(keep)
    m = RA.build(FT.align(f))
    assert m.ok and m.n_anchors >= 20
    assert abs(m.at(1, 4.0) - (DRIFT[1] - np.median(DRIFT))) <= 0.02


def test_as_dict_is_plain_json_and_reference_is_the_inverse():
    import json
    m = RA.build(FT.align(_feats(_ions(), DRIFT)))
    d = m.as_dict([f"f{k}" for k in range(6)])
    json.dumps(d)
    assert len(d["shift"]) == 6 and all(len(s) == len(d["rt"]) for s in d["shift"]) and d["anchors"] == 40
    rt = np.array([2.0, 5.0, 8.0])
    for k in range(6):
        assert np.allclose(m.reference(m.expected(rt, k), k), rt, atol=1e-3)


def test_gap_filling_looks_where_the_map_expects_the_peak():
    from test_hr_features import table
    rng = np.random.default_rng(11)
    # file 1 is 0.08 min late: the weak product (height 1.5e5, under the detection height) sits at 5.08, a strong isomer-like trace of the same m/z at 5.35
    t = table([(300.1, 5.08, 0.05, 1.5e5, 0.3, 0, 99), (300.1, 5.35, 0.03, 3e6, 0.3, 0, 99)], rng)

    def al():
        a = FT.Alignment(mz=np.array([300.1]), rt=np.array([5.0]), area=np.array([[3e7, 0.0]]), height=np.array([[5e6, 0.0]]), apex_rt=np.array([[5.0, np.nan]], np.float32),
                         left=np.array([[4.6, np.nan]], np.float32), right=np.array([[5.4, np.nan]], np.float32), n_files=2)
        return a
    rm = RA.RtMap(np.array([1.0, 9.0]), np.array([[0.0, 0.0], [0.08, 0.08]]), 8, np.zeros(2))
    plain, aligned = al(), al()
    FT.fill_gaps(plain, [0], lambda k: t, [1])                                 # one window around the median apex: the strong trace wins
    f = FT.fill_gaps(aligned, [0], lambda k: t, [1], rt_tol=0.04, rtmap=rm)
    assert f[0, 1] and not f[0, 0] and aligned.area[0, 0] == 3e7                # found cells are never touched
    assert plain.area[0, 1] > 10 * aligned.area[0, 1] > 0                       # the aligned fill is the weak product, not the neighbour
    assert 1e5 < aligned.area[0, 1] < 3e6
