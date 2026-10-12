"""Python reference of crates/mzlab-core/src/centroidi (profile scan -> centroids), same operations in the same order, for
crates/mzlab-core/tests/centroidi_parity.rs. Only a reference for the tests: the program uses the Rust version.

    python3 tools/centroidi_riferimento.py OUT.json

Synthetic scans only (seeded generator): [{params, mz, y, centroids: [[mz, value, area, height, flags], ...]}] and the estimates of a few scan sets.
"""
import json
import math
import random
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from centroidi_nucleo import *  # noqa: E402,F401,F403
from centroidi_nucleo import CONTINUOUS, COUNTS, P, Buf, background, centroids, fill, noise, rnd, smooth  # noqa: E402


# ---- estimate


def mode_of(v):
    v = sorted(v)
    key = lambda x: f"{x:.5e}"  # noqa: E731
    best, best_n, i = v[0], 0, 0
    while i < len(v):
        j = i
        while j < len(v) and key(v[j]) == key(v[i]):
            j += 1
        if j - i > best_n:
            best_n, best = j - i, v[i]
        i = j
    return best


def unit_and_mode(scans):
    mins = [min(v for v in y if v > 0.0) for _, y in scans if any(v > 0.0 for v in y)]
    if not mins:
        return 1.0, CONTINUOUS
    u = mode_of(mins)
    allc = ok = 0
    for _, y in scans:
        for v in y:
            if v > 0.0:
                allc += 1
                q = v / u
                if rnd(q) >= 1.0 and abs(q - rnd(q)) <= 0.01:
                    ok += 1
    return u, (COUNTS if allc > 0 and ok >= 0.995 * allc else CONTINUOUS)


def step_and_grid(scans):
    d = sorted(mz[i + 1] - mz[i] for mz, _ in scans for i in range(len(mz) - 1) if mz[i + 1] - mz[i] > 0.0)
    if not d:
        return 1.0, False
    mn = d[0]
    reg = all(abs(x / mn - rnd(x / mn)) <= 0.01 * max(rnd(x / mn), 1.0) for x in d)
    return (mn if reg else d[(len(d) - 1) // 2]), reg


def fwhms(buf, p, sigma, out):
    n = len(buf.s)
    wb = p.wb()
    u = p.unit if p.mode == COUNTS else 1.0
    for j in range(1, n - 1):
        s = buf.s[j]
        if not (s >= buf.s[j - 1] and s > buf.s[j + 1]):
            continue
        h = s - buf.b[j]
        sn = h / u / math.sqrt(max(s / u, 1.0)) if p.mode == COUNTS else h / sigma
        if sn < 20.0:
            continue
        lo, hi = max(j - 3 * wb, 0), min(j + 3 * wb, n - 1)
        iso = all(i == j or not (buf.s[i] >= buf.s[max(i - 1, 0)] and buf.s[i] > buf.s[min(i + 1, n - 1)]) or buf.s[i] <= 0.2 * s for i in range(lo, hi + 1))
        if not iso:
            continue
        half = buf.b[j] + 0.5 * h
        l = r = j
        while l > 0 and buf.s[l] > half:
            l -= 1
        while r + 1 < n and buf.s[r] > half:
            r += 1
        if buf.s[l] > half or buf.s[r] > half:
            continue
        xl = buf.x[l] + (half - buf.s[l]) / (buf.s[l + 1] - buf.s[l]) * (buf.x[l + 1] - buf.x[l])
        xr = buf.x[r - 1] + (buf.s[r - 1] - half) / (buf.s[r - 1] - buf.s[r]) * (buf.x[r] - buf.x[r - 1])
        out.append(xr - xl)


def estimate(scans, fallback_w):
    unit, mode = unit_and_mode(scans)
    step, reg = step_and_grid(scans)
    vmax = 0.0
    for _, y in scans:
        for v in y:
            vmax = max(vmax, v)
    p = P(mode, unit, step, max(fallback_w, 3.0 * step), 3.0, vmax, True)
    widths = []
    for mz, y in scans:
        buf = Buf()
        fill(mz, y, p, buf)
        if len(buf.v) < 5:
            continue
        smooth(buf, 5)
        background(buf, p.wb(), mode == COUNTS)
        sigma = noise(buf, p.wb()) if mode == CONTINUOUS else 1.0
        fwhms(buf, p, sigma, widths)
    peaks = len(widths) if len(widths) >= 8 else 0
    if peaks:
        widths.sort()
        p.w = max(widths[(len(widths) - 1) // 2], 3.0 * step)
    p.area = not p.hr()
    return p, reg, peaks


# ---- synthetic scans


def gauss(x, c, h, w):
    return h * math.exp(-((x - c) ** 2) / (2.0 * (w / 2.355) ** 2))


def counts_scan(rng, step, bins, peaks, bg, unit, start=100.0):
    mz, y = [], []
    for b in range(bins):
        x = start + b * step
        lam = bg + sum(gauss(x, c, h, w) for c, h, w in peaks)
        n = 0
        # Poisson by inversion (the Rust tests use their own generator; here the numbers only have to be the same for both sides)
        lim, k, pr = math.exp(-lam), 0, 1.0
        while True:
            pr *= rng.random()
            if pr <= lim:
                break
            k += 1
        n = k
        if n > 0:
            mz.append(x)
            y.append(n * unit)
    return mz, y


def continuous_scan(rng, mz0, n, jitter, peaks, noise_amp):
    mz, y = [], []
    for i in range(n):
        x = mz0 + i * 0.001 + jitter * rng.random()
        mz.append(x)
        y.append(sum(gauss(x, c, h, w) for c, h, w in peaks) + noise_amp * rng.random())
    return mz, y


def main():
    rng = random.Random(2024)
    rows = []
    for k in range(80):
        unit = rng.choice([1.0, 2.5, 7.3])
        w = 0.7
        peaks = [(100.0 + 1.0 + rng.random() * 4.0 + 3.0 * j, rng.choice([8.0, 30.0, 150.0, 900.0]), w) for j in range(rng.randint(0, 4))]
        mz, y = counts_scan(rng, 0.06, 200, peaks, rng.choice([0.0, 0.3, 2.0]), unit)
        vmax = 0.0
        if y and k % 5 == 0:                       # clip the tallest peaks: a flat top at the saturation level
            vmax = 0.6 * max(y)
            y = [min(v, vmax) for v in y]
        p = P(COUNTS, unit, 0.06, w, 3.0, vmax, True)
        rows.append({"params": [p.mode, p.unit, p.step, p.w, p.k, p.vmax, p.area], "mz": mz, "y": y, "centroids": centroids(mz, y, p)})
    for k in range(30):
        fw = 400.0 / 60000.0
        peaks = [(400.05 + 0.01 * rng.random() + 0.03 * j, 10.0 ** rng.uniform(4.0, 7.0), fw) for j in range(rng.randint(1, 3))]
        mz, y = continuous_scan(rng, 400.0, 120, 0.0003, peaks, 50.0)
        p = P(CONTINUOUS, 1.0, 0.001, fw, 3.0, 0.0, False)
        rows.append({"params": [p.mode, p.unit, p.step, p.w, p.k, p.vmax, p.area], "mz": mz, "y": y, "centroids": centroids(mz, y, p)})
    est = []
    for k in range(4):
        scans = [counts_scan(rng, 0.06, 240, [(110.0 + 0.01 * t, 900.0, 0.7), (112.5, 700.0, 0.7), (115.0, 500.0, 0.7)], 0.3, 2.5, 107.0) for t in range(20)]
        p, reg, peaks = estimate(scans, 0.7)
        est.append({"scans": [{"mz": a, "y": b} for a, b in scans], "params": [p.mode, p.unit, p.step, p.w, p.k, p.vmax, p.area], "regular": reg, "peaks": peaks})
    json.dump({"scans": rows, "estimates": est}, open(sys.argv[1], "w"))


if __name__ == "__main__":
    main()
