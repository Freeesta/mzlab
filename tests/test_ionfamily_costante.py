import math

import numpy as np

from mzlab import ionfamily as f


def test_wspearman_constant_trace_is_nan_even_with_weights():
    """Ranks of a constant trace are all equal: the weighted mean is the constant up to an ulp, which must not give a correlation of ~1e-16."""
    w = np.array([0.5067, 0.5499, 0.4706, 0.5874, 0.7650, 0.7180, 0.2594, 0.2256, 0.3320])
    y = np.array([66072.0, 32733.0, 449395.0, 116059.0, 599391.0, 382724.0, 66059.0, 99395.0, 99399.0])
    assert math.isnan(f.wspearman(np.zeros(9), y, w))
    assert math.isnan(f.wspearman(np.zeros(9), y))
