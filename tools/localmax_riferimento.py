"""Independent reference of the local-maximum mass detector of mzmine 4.10.90 (`LocalMaxMassDetector`) with numpy.

    python3 tools/localmax_riferimento.py     # writes tests/data/untargeted/localmax/casi.json (the Rust test reads it)

Synthetic profile spectra only (Gaussian peaks, fixed seed). Smoothing: Gaussian or Savitzky-Golay (quadratic) with the window cut at the ends of
the run and no renormalisation; the smoothed series serves maxima and valleys only, centroid and height/area use the raw intensities.
"""
from __future__ import annotations

import json
from pathlib import Path

import numpy as np

OUT = Path(__file__).resolve().parent.parent / "tests/data/untargeted/localmax/casi.json"


def window_weights(method, window):
    w = max(window, 3)
    w += 1 - w % 2
    m = (w - 1) // 2
    k = np.arange(-m, m + 1)
    if method == "sg":
        raw = 3 * m * (m + 1) - 1 - 5 * k * k
    else:
        raw = np.exp(-(k * k) / (2 * (w / 6) ** 2))
    return raw / raw.sum(), m


def smooth(y, s, e, method, window):
    wts, m = window_weights(method, window)
    out = np.zeros(len(y))
    for i in range(s, e + 1):
        acc = 0.0
        for j, wt in enumerate(wts):
            p = i + j - m
            if s <= p <= e:
                acc += wt * y[p]
        out[i] = max(acc, 0.0) if method == "sg" else acc
    return out


def max_gap(mz, y):
    n = len(mz)
    right = -1.0
    top = n - 1
    while top > 1:
        if y[top] > 0 and y[top - 1] > 0:
            right = abs(mz[top] - mz[top - 1])
            break
        top -= 1
    left = -1.0
    for i in range(1, top):
        if y[i] > 0 and y[i - 1] > 0:
            left = abs(mz[i] - mz[i - 1])
            break
    g = max(left, right)
    return 0.1 if g == -1.0 else g


def detect(mz, y, noise=0.0, area=False, min_dp=3, method="none", window=5):
    n = len(mz)
    if n < min_dp:
        return [], []
    gap = max_gap(mz, y)
    runs, start, above = [], 0, False
    for i in range(1, n):
        above = above or y[i] > noise
        if mz[i] - mz[i - 1] >= gap:
            if i - start >= min_dp and above:
                runs.append((start, i - 1))
            start, above = i, False
    if n - start >= min_dp and above:
        runs.append((start, n - 1))
    pos = [v for v in y if v > 0]
    min_abs = min(pos) if pos else 0.0
    d = np.array(y, float)
    if method != "none":
        d = np.zeros(n)
        for s, e in runs:
            d[s:e + 1] = smooth(y, s, e, method, window)[s:e + 1]
    rm, ry = [], []
    for s, e in runs:
        cand = [i for i in range(s, e + 1)
                if d[i] >= (d[i - 1] if i > s else 0) and d[i] > (d[i + 1] if i < e else 0)]
        if not cand:
            continue
        edges, active, left = [], cand[0], s
        for nxt in cand[1:]:
            seg = d[active + 1:nxt]
            valley = active + 1 + int(np.argmin(seg)) if len(seg) else active
            if d[valley] < 0.7 * d[active] and d[nxt] > d[valley] / 0.7:
                edges.append((left, valley + 1))
                active, left = nxt, valley
            elif d[nxt] > d[active]:
                active = nxt
        edges.append((left, e + 1))
        for a, b in edges:
            if b - a < min_dp:
                continue
            apex = a + int(np.argmax(y[a:b]))
            top = y[apex]
            if top < max(noise, 2 * min_abs):
                continue
            cut = top * 0.4
            edge = min(apex - a, b - apex)
            sel = [i for i in range(a, b) if y[i] > cut and apex - edge <= i <= apex + edge]
            den = sum(y[i] for i in sel)
            nz = sum(1 for i in range(a, b) if y[i] > 0)
            if den == 0 or nz < min_dp:
                continue
            rm.append(sum(mz[i] * y[i] for i in sel) / den)
            ry.append(float(sum(y[a:b])) if area else float(top))
    return rm, ry


def spectrum(peaks, lo=100.0, hi=103.0, noise=0.0, seed=0, cut=None):
    """Points equally spaced in sqrt(m/z) as on a TOF, so the step grows with m/z and only a real jump splits a run."""
    k = np.arange(0, (np.sqrt(hi) - np.sqrt(lo)) / 5e-4)
    mz = (np.sqrt(lo) + 5e-4 * k) ** 2
    y = np.zeros(len(mz))
    for c, h, s in peaks:
        y += h * np.exp(-0.5 * ((mz - c) / s) ** 2)
    if noise:
        y += np.abs(np.random.default_rng(seed).normal(0, noise, len(mz)))
    y[y < 1e-3] = 0.0
    if cut:
        keep = np.ones(len(mz), bool)
        keep[cut[0]:cut[1]] = False
        mz, y = mz[keep], y[keep]
    return mz, np.round(y, 6)


def cases():
    one = spectrum([(101.0, 1000, 0.03)])
    two = spectrum([(100.5, 1000, 0.03), (102.0, 600, 0.03)])
    close = spectrum([(101.0, 1000, 0.03), (101.08, 900, 0.03)])
    shoulder = spectrum([(101.0, 1000, 0.05), (101.12, 700, 0.05)])
    noisy = spectrum([(101.0, 1000, 0.03)], noise=3.0, seed=410)
    holes = spectrum([(100.4, 800, 0.03), (102.5, 500, 0.03)], cut=(140, 180))
    return [
        ("single_peak", one, {}), ("single_peak_area", one, {"area": True}),
        ("two_separated_peaks", two, {}), ("two_close_peaks", close, {}),
        ("two_resolved_close_peaks", spectrum([(101.0, 1000, 0.03), (101.2, 900, 0.03)]), {}),
        ("shoulder_merges", shoulder, {}), ("noise_floor", noisy, {"noise": 20.0}),
        ("noisy_gaussian_smoothing", noisy, {"noise": 20.0, "method": "gauss"}),
        ("noisy_savitzky_golay", noisy, {"noise": 20.0, "method": "sg", "window": 7}),
        ("zero_gap_splits_runs", holes, {}),
    ]


def main():
    out = []
    for name, (mz, y), kw in cases():
        rm, ry = detect(list(mz), list(y), **kw)
        out.append({"name": name, "mz": mz.tolist(), "y": y.tolist(), "noise": kw.get("noise", 0.0),
                    "area": kw.get("area", False), "method": kw.get("method", "none"),
                    "window": kw.get("window", 5), "peak_mz": rm, "peak_y": ry})
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(out, separators=(",", ":")) + "\n")


if __name__ == "__main__":
    main()
