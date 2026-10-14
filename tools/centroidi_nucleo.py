"""Core of the Python reference of crates/mzlab-core/src/centroidi (grid, smoothing, background, detection, centroids), same operations in the same
order. Used by tools/centroidi_riferimento.py; only a reference for the tests."""
import math

TRUNCATED, SATURATED, ASYMMETRIC, SHOULDER = 1, 2, 4, 8
COUNTS, CONTINUOUS = "counts", "continuous"


def rnd(x):
    """Rust f64::round (half away from zero)."""
    return math.floor(x + 0.5) if x >= 0 else -math.floor(-x + 0.5)


class P:
    def __init__(self, mode, unit, step, w, k=3.0, vmax=0.0, area=True):
        self.mode, self.unit, self.step, self.w, self.k, self.vmax, self.area = mode, unit, step, w, k, vmax, area

    def wb(self):
        return max(int(rnd(self.w / self.step)), 3)

    def hr(self):
        return self.mode == CONTINUOUS and self.wb() >= 7

    def sg_len(self):
        if self.mode == CONTINUOUS and self.wb() < 7:
            return 5
        n = int(rnd(0.7 * self.wb()))
        n = n + 1 if n % 2 == 0 else n
        return min(max(n, 5), 15)


class Buf:
    def __init__(self):
        self.x, self.v, self.s, self.b = [], [], [], []


def regular(mz, step):
    for m in mz:
        q = (m - mz[0]) / step
        if abs(q - rnd(q)) > 0.05:
            return False
    return True


def fill(mz, y, p, buf):
    buf.x, buf.v = [], []
    n = min(len(mz), len(y))
    if n == 0:
        return
    mz, y = mz[:n], y[:n]
    if regular(mz, p.step):
        last = int(rnd((mz[-1] - mz[0]) / p.step))
        buf.x = [mz[0] + b * p.step for b in range(last + 1)]
        buf.v = [0.0] * (last + 1)
        for i, m in enumerate(mz):
            b = int(rnd((m - mz[0]) / p.step))
            buf.x[b] = m
            buf.v[b] += y[i]
        return
    for i in range(n):
        if i > 0 and mz[i] - mz[i - 1] > 1.5 * p.step:
            gap = mz[i] - mz[i - 1]
            buf.x.append(mz[i - 1] + p.step)
            buf.v.append(0.0)
            if gap > 2.0 * p.step:
                buf.x.append(mz[i] - p.step)
                buf.v.append(0.0)
        buf.x.append(mz[i])
        buf.v.append(y[i])


def sg_coef(l):
    m = float((l - 1) // 2)
    den = (2.0 * m - 1.0) * (2.0 * m + 1.0) * (2.0 * m + 3.0)
    return [3.0 * (3.0 * m * m + 3.0 * m - 1.0 - 5.0 * (i - m) ** 2) / den for i in range(l)]


def smooth(buf, l):
    c = sg_coef(l)
    m = (l - 1) // 2
    n = len(buf.v)
    buf.s = []
    for j in range(n):
        acc = 0.0
        for t, ck in enumerate(c):
            i = j + t - m
            if 0 <= i < n:
                acc += ck * buf.v[i]
        buf.s.append(max(acc, 0.0))


def lower(vals, q):
    vals.sort()
    return vals[int(math.floor((len(vals) - 1) * q))]


def background(buf, wb, mean):
    n = len(buf.s)
    bs = max(int(rnd(2.5 * wb)), 1)
    half = 5 * wb
    nb = (n + bs - 1) // bs
    cen, val = [], []
    for b in range(nb):
        c = min(b * bs + bs // 2, n - 1)
        lo, hi = max(c - half, 0), min(c + half, n - 1)
        tmp = list(buf.s[lo:hi + 1])
        q = lower(tmp, 0.25)
        if mean:
            k = int(math.floor((len(tmp) - 1) * 0.75))
            v = 0.0
            for t in tmp[:k + 1]:
                v += t
            v = v / (k + 1)
        else:
            v = q
        cen.append(c)
        val.append(v)
    buf.b = []
    b = 0
    for j in range(n):
        while b + 1 < nb and cen[b + 1] <= j:
            b += 1
        if j <= cen[0] or b + 1 >= nb:
            buf.b.append(val[0] if j <= cen[0] else val[nb - 1])
        else:
            f = (j - cen[b]) / (cen[b + 1] - cen[b])
            buf.b.append(val[b] + f * (val[b + 1] - val[b]))


def median(vals):
    vals.sort()
    return vals[(len(vals) - 1) // 2]


def mad_sigma(d):
    med = median(list(d))
    return 1.4826 * median([abs(v - med) for v in d]) / math.sqrt(2.0)


def noise(buf, wb):
    d = [buf.v[i + 1] - buf.v[i] for i in range(len(buf.v) - 1)]
    bl = 10 * wb
    sig = [mad_sigma(d[i:i + bl]) for i in range(0, len(d), bl) if len(d[i:i + bl]) >= 5]
    if not sig:
        if not d:
            return 1e-300
        sig.append(mad_sigma(d))
    return max(lower(sig, 0.25), 1e-300)


# ---- detect


def candidates(buf):
    n = len(buf.s)
    out = []
    for j in range(n):
        l = buf.s[j - 1] if j > 0 else -math.inf
        r = buf.s[j + 1] if j + 1 < n else -math.inf
        if buf.s[j] >= l and buf.s[j] > r and buf.s[j] > buf.b[j] and buf.s[j] > 0.0:
            out.append(j)
    return out


def bounds(buf, apex, wb):
    ext = int(math.ceil(1.5 * wb))
    n = len(buf.s)
    lo = apex
    while lo > 0 and apex - lo < ext and buf.s[lo - 1] <= buf.s[lo] and buf.s[lo - 1] > buf.b[lo - 1] and not (buf.v[lo - 1] == 0.0 and buf.v[lo] == 0.0 and lo != apex):
        lo -= 1
    hi = apex
    while hi + 1 < n and hi - apex < ext and buf.s[hi + 1] <= buf.s[hi] and buf.s[hi + 1] > buf.b[hi + 1] and not (buf.v[hi + 1] == 0.0 and buf.v[hi] == 0.0 and hi != apex):
        hi += 1
    return lo, hi


def valley(buf, a, b):
    v = a
    for j in range(a, b + 1):
        if buf.s[j] < buf.s[v]:
            v = j
    return v


def raw_near(buf, c, h):
    lo, hi = max(c - h, 0), min(c + h, len(buf.v) - 1)
    t = 0.0
    for v in buf.v[lo:hi + 1]:
        t += v
    return t


def insignificant(buf, p, sigma, a, c):
    v = valley(buf, a, c)
    low = c if buf.s[c] < buf.s[a] else a
    if p.mode == COUNTS:
        h = max(int(math.ceil(0.25 * p.wb())), 1)
        ca, cv = raw_near(buf, low, h), raw_near(buf, v, h)
        return ca + cv <= 0.0 or (ca - cv) / math.sqrt(ca + cv) < p.k
    return buf.s[low] - buf.s[v] < p.k * sigma


def groups(buf, p, sigma):
    wb = p.wb()
    out = []
    for c in candidates(buf):
        lo, hi = bounds(buf, c, wb)
        merge = bool(out) and ((c - out[-1]["apex"]) < 0.5 * wb or insignificant(buf, p, sigma, out[-1]["apex"], c))
        if merge:
            last = out[-1]
            if buf.s[c] > buf.s[last["apex"]]:
                last["apex"] = c
            last["lo"], last["hi"], last["members"] = min(last["lo"], lo), max(last["hi"], hi), last["members"] + 1
        else:
            out.append({"apex": c, "lo": lo, "hi": hi, "members": 1})
    for i in range(1, len(out)):
        prev = out[i - 1]["hi"]
        if out[i]["lo"] <= prev:
            out[i]["lo"] = min(prev + 1, out[i]["apex"])
    return out


# ---- emit


def fsum(vals):
    t = 0.0
    for v in vals:
        t += v
    return t


def sums(buf, p, g):
    u = p.unit if p.mode == COUNTS else 1.0
    return fsum(buf.v[g["lo"]:g["hi"] + 1]) / u, fsum(buf.b[g["lo"]:g["hi"] + 1]) / u


def passes(buf, p, sigma, g):
    area, back = sums(buf, p, g)
    nz = sum(1 for v in buf.v[g["lo"]:g["hi"] + 1] if v > 0.0)
    if p.mode == COUNTS:
        z = (area - back) / math.sqrt(area + back) if area + back > 0.0 else 0.0
        return z >= p.k and area - back >= 3.0 and nz >= 2
    return buf.s[g["apex"]] - buf.b[g["apex"]] >= p.k * sigma


def apex_x(buf, j):
    if j == 0 or j + 1 >= len(buf.s):
        return buf.x[j]
    den = buf.s[j - 1] - 2.0 * buf.s[j] + buf.s[j + 1]
    if den >= 0.0:
        return buf.x[j]
    d = min(max(0.5 * (buf.s[j - 1] - buf.s[j + 1]) / den, -1.0), 1.0)
    return buf.x[j] + d * ((buf.x[j + 1] - buf.x[j]) if d > 0 else (buf.x[j] - buf.x[j - 1]))


def centroid_at(buf, p, c, skip, half):
    lo, hi = c - half, c + half
    sw = swm = 0.0
    for i in range(len(buf.x)):
        a, b = buf.x[i] - 0.5 * p.step, buf.x[i] + 0.5 * p.step
        frac = min(max((min(b, hi) - max(a, lo)) / p.step, 0.0), 1.0)
        if frac <= 0.0 or (skip is not None and buf.v[i] >= skip):
            continue
        wt = max(buf.v[i] - buf.b[i], 0.0) * frac
        sw += wt
        swm += wt * buf.x[i]
    return swm / sw if sw > 0.0 else None


def gaussian(buf, sigma, apex):
    m = apex
    for i in range(max(apex - 1, 0), min(apex + 1, len(buf.v) - 1) + 1):
        if buf.v[i] > buf.v[m]:
            m = i
    if m == 0 or m + 1 >= len(buf.v) or buf.v[m - 1] <= 0.0 or buf.v[m] <= 0.0 or buf.v[m + 1] <= 0.0 or (buf.v[m] - buf.b[m]) / sigma < 10.0:
        return None
    a, b, c = math.log(buf.v[m - 1]), math.log(buf.v[m]), math.log(buf.v[m + 1])
    den = a - 2.0 * b + c
    if den >= 0.0:
        return None
    d = 0.5 * (a - c) / den
    return buf.x[m] + d * ((buf.x[m + 1] - buf.x[m]) if d > 0 else (buf.x[m] - buf.x[m - 1]))


def saturated(buf, p, g):
    if p.vmax <= 0.0:
        return False
    t, run = 0.99 * p.vmax, 0
    for j in range(g["lo"], g["hi"] + 1):
        run = run + 1 if buf.v[j] >= t else 0
        if run >= 3:
            return True
    return False


def saturated_centre(buf, p, g):
    t = 0.99 * p.vmax
    top = [j for j in range(g["lo"], g["hi"] + 1) if buf.v[j] >= t]
    if not top:
        return None
    a, b = top[0], top[-1]
    half = 0.5 * (buf.x[b] - buf.x[a]) + 0.5 * p.w
    c = 0.5 * (buf.x[a] + buf.x[b])
    for _ in range(2):
        r = centroid_at(buf, p, c, t, half)
        c = c if r is None else r
    return c


# ---- shoulders


def second(buf, l, j):
    m = float((l - 1) // 2)
    mu = m * (m + 1.0) / 3.0
    den = fsum([((i - m) ** 2 - mu) ** 2 for i in range(l)])
    acc = 0.0
    for i in range(l):
        t = j + i - (l - 1) // 2
        if 0 <= t < len(buf.v):
            acc += 2.0 * ((i - m) ** 2 - mu) / den * buf.v[t]
    return acc


def residual(buf, p, j, x0, h):
    hw = int(math.ceil(0.5 * p.wb()))
    lo, hi = max(j - hw, 0), min(j + hw, len(buf.v) - 1)
    sg = p.w / 2.355
    a = am = top = f = 0.0
    for i in range(lo, hi + 1):
        r = buf.v[i] - buf.b[i] - h * math.exp(-((buf.x[i] - x0) ** 2) / (2.0 * sg * sg))
        if r > 0.0:
            a += r
            am += r * buf.x[i]
            top = max(top, r)
        f += buf.b[i]
    return (a, am / a, top, f) if a > 0.0 else None


def find_shoulders(buf, p, sigma, g, x0, area):
    l = p.sg_len()
    dist = int(math.ceil(0.6 * p.wb()))
    h = buf.s[g["apex"]] - buf.b[g["apex"]]
    out = []
    for side in (-1, 1):
        best = None
        d = dist
        while True:
            j = g["apex"] + side * d
            if j < g["lo"] or j > g["hi"]:
                break
            v = second(buf, l, j)
            if v < 0.0 and (best is None or v < best[0]):
                best = (v, j)
            d += 1
        if best is None:
            continue
        r = residual(buf, p, best[1], x0, h)
        if r is None:
            continue
        a, mz, top, f = r
        if p.mode == COUNTS:
            ac, fc = a / p.unit, f / p.unit
            ok = ac / math.sqrt(ac + 2.0 * fc) >= p.k + 1.0
        else:
            ok = top >= (p.k + 1.0) * sigma
        if ok and a >= 0.10 * area and abs(mz - x0) >= 0.5 * p.w:
            out.append([mz, a if p.area else top, a, top, SHOULDER])
    return out


def one(buf, p, sigma, g, out):
    if not passes(buf, p, sigma, g):
        return
    nz = sum(1 for v in buf.v[g["lo"]:g["hi"] + 1] if v > 0.0)
    truncated = g["lo"] == 0 or g["hi"] + 1 == len(buf.v)
    if truncated and nz < 3:
        return
    flags = TRUNCATED if truncated else 0
    sat = saturated(buf, p, g)
    x0 = apex_x(buf, g["apex"])
    if sat:
        flags |= SATURATED
        c = saturated_centre(buf, p, g)
        mz = x0 if c is None else c
    elif p.hr():
        mz = gaussian(buf, sigma, g["apex"])
        if mz is None:
            mz = centroid_at(buf, p, x0, None, 0.5 * p.w)
        if mz is None:
            mz = x0
    else:
        mz = centroid_at(buf, p, x0, None, 0.5 * p.w)
        if mz is None:
            mz = x0
    if abs(mz - x0) > 0.25 * p.w:
        flags |= ASYMMETRIC
    area = fsum(buf.v[g["lo"]:g["hi"] + 1])
    height = buf.s[g["apex"]]
    sh = find_shoulders(buf, p, sigma, g, x0, area) if g["members"] == 1 and not sat else []
    for s in sh:
        area -= s[2]
    out.append([mz, area if (p.area or sat) else height, area, height, flags])
    out.extend(sh)


def centroids(mz, y, p):
    buf = Buf()
    fill(mz, y, p, buf)
    if len(buf.v) < 3:
        return []
    smooth(buf, p.sg_len())
    background(buf, p.wb(), p.mode == COUNTS)
    sigma = noise(buf, p.wb()) if p.mode == CONTINUOUS else 0.0
    out = []
    for g in groups(buf, p, sigma):
        one(buf, p, sigma, g, out)
    out.sort(key=lambda c: c[0])
    return out
