# TPMINE-PRIVATE
"""mzFinder (public, synthetic): points on the difference map used by the low-resolution engine."""
import numpy as np

from tpmine import mappa as X


def test_find_points_on_a_difference_map():
    nrt, nmz = 200, 120
    rt0, rt1, mz0, dmz = 0.0, 10.0, 200.0, 1.0
    rng = np.random.default_rng(1)
    b = rng.uniform(0, 50, (nrt, nmz))
    a = b.copy()
    i, j = 80, 40                                                                                           # one real new peak: A >> B
    for di, w in zip(range(-2, 3), [0.3, 0.8, 1.0, 0.8, 0.3]):
        a[i + di, j] += 5e4 * w
    a[150, 90] += 5e4
    b[150, 90] += 4.5e4                                                                                     # present in both: ratio A/B too small
    pts = X.find_points(a - b, a, rt0, rt1, mz0, dmz)
    assert len(pts) == 1 and abs(pts[0]["rt"] - 4.025) < 0.06 and abs(pts[0]["mz"] - 240.5) < 0.6
    assert pts[0]["ratio"] > 100 and pts[0]["width"] >= 0.1
    assert X.find_points(a - b, a, rt0, rt1, mz0, dmz, {"min_ratio": 1.05})[0]["rt"] != 0                  # a looser threshold lets more through
    assert len(X.find_points(a - b, a, rt0, rt1, mz0, dmz, {"min_ratio": 1.05})) == 2
    assert X.find_points(np.zeros((2, 2)), np.zeros((2, 2)), 0, 1, 0, 1) == []
