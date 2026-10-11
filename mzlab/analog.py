"""Analog traces of an mzML (pump pressure, flow, temperature...) and the LC gradient model.

Pure functions: unit conversion, reduction for drawing, the parsers of a gradient (the Chromeleon script text that Thermo .raw files keep, a CSV typed or
pasted by the student) and the one gradient model every source ends in::

    {"source": "dam" | "raw" | "manual" | "csv" | "course", "points": [{"t": min, "b": %B, "flow"?: mL/min, "curve"?: int}], "run_time"?: min, "solvents"?: {"a", "b"}}

Between two points the %B is drawn as a straight ramp (Chromeleon curve 5); another curve is kept in the point and flagged in ``nonlinear``.
"""
from __future__ import annotations

import re

import numpy as np

from .i18n import UserError

# ---------------------------------------------------------------- pressure units
_TO_BAR = (("megapascal", 10.0), ("mpa", 10.0), ("kilopascal", 0.01), ("kpa", 0.01), ("pascal", 1e-5), ("pa", 1e-5), ("psi", 0.0689476), ("pound", 0.0689476), ("bar", 1.0))
BAR_MAX = 2000.0       # no HPLC pump reaches this: values above it come from a unit written ten times too small (Thermo files declare pascal and store 10x)


def pressure_to_bar(y, unit: str | None):
    """(values in bar, note or None). The note is an i18n message ``{"key", "params"}`` when the unit had to be corrected."""
    y = np.asarray(y, dtype=float)
    u = (unit or "").strip().lower()
    f = next((f for name, f in _TO_BAR if name == u), None) or next((f for name, f in _TO_BAR if name in u), 1.0)
    out = y * f
    top = float(np.nanmax(np.abs(out))) if len(out) else 0.0
    if top > BAR_MAX:
        return out / 10.0, {"key": "analog.unit.tenfold", "params": {"unit": unit or ""}}
    return out, None


def reduce_minmax(t, y, n: int = 2000):
    """(t, y) with at most about 2 n points: per interval of equal width the lowest and the highest value, in time order (a peak never disappears)."""
    t, y = np.asarray(t, dtype=float), np.asarray(y, dtype=float)
    if len(t) <= 2 * n or n < 1:
        return t, y
    edges = np.linspace(0, len(t), n + 1).astype(int)
    tt, yy = [], []
    for a, b in zip(edges[:-1], edges[1:]):
        if b <= a:
            continue
        seg = y[a:b]
        i, j = a + int(np.argmin(seg)), a + int(np.argmax(seg))
        for k in sorted({i, j}):
            tt.append(t[k]); yy.append(y[k])
    return np.array(tt), np.array(yy)


# ---------------------------------------------------------------- gradient model
def _num(s):
    try:
        return float(str(s).strip().replace(",", "."))
    except ValueError:
        return None


def gradient_model(source: str, points: list[dict], run_time: float | None = None, solvents: dict | None = None) -> dict:
    """Checks and normalizes a gradient: points sorted by time (a step = two points with the same time), %B 0-100; at least two points."""
    pts = []
    for p in points:
        t, b = _num(p.get("t")), _num(p.get("b"))
        if t is None or b is None or t < 0 or not 0 <= b <= 100:
            raise UserError("gradient.err.point", {"t": p.get("t"), "b": p.get("b")}, f"bad gradient point {p!r}")
        q = {"t": round(t, 4), "b": round(b, 4)}
        fl = _num(p.get("flow")) if p.get("flow") not in (None, "") else None
        if fl is not None:
            q["flow"] = round(fl, 4)
        if p.get("curve") not in (None, ""):
            q["curve"] = int(p["curve"])
        pts.append(q)
    if len(pts) < 2:
        raise UserError("gradient.err.few", {}, "a gradient needs at least two points")
    if any(b["t"] < a["t"] for a, b in zip(pts, pts[1:])):
        raise UserError("gradient.err.order", {}, "the times of a gradient must not decrease")
    out = {"source": source, "points": pts, "run_time": run_time if run_time is not None else pts[-1]["t"],
           "nonlinear": any(p.get("curve") not in (None, 5) for p in pts)}
    if solvents:
        out["solvents"] = solvents
    return out


def gradient_at(model: dict, t: float) -> dict | None:
    """%B (and flow) at time t by linear ramps; None outside the programmed time."""
    pts = model["points"]
    if not pts or t < pts[0]["t"] or t > pts[-1]["t"]:
        return None
    res = None
    for a, b in zip(pts, pts[1:]):
        if a["t"] <= t <= b["t"]:
            w = 0.0 if b["t"] == a["t"] else (t - a["t"]) / (b["t"] - a["t"])
            res = {"b": a["b"] + w * (b["b"] - a["b"])}
            if "flow" in a and "flow" in b:
                res["flow"] = a["flow"] + w * (b["flow"] - a["flow"])
            if t == b["t"] and b["t"] == a["t"]:
                continue
    return res


def from_dam(lc: dict | None) -> dict | None:
    """The gradient of an Analyst method (already read by methodinfo._lc)."""
    if not lc or not lc.get("gradient") or len(lc["gradient"]) < 2:
        return None
    return gradient_model("dam", [{"t": g["t"], "b": g["b"], "flow": g.get("flow")} for g in lc["gradient"]], lc.get("run_time"))


# ---------------------------------------------------------------- parsers
def _clean(text: str) -> str:
    """Printable text only: a script read from a binary file may carry NUL bytes and broken characters."""
    return "".join(ch if (ch == "\n" or ch == "\t" or ch >= " ") and ch not in "\x7f�" else " " for ch in (text or "").replace("\r", "\n"))


_TIME = re.compile(r"^\s*(\d+(?:[.,]\d+)?)\s*(?:\[?\s*min\s*\]?)?\s*(?=[A-Za-z%(]|$)")
_B = re.compile(r"%\s*B\s*(?:\.\s*Value)?\s*[:=]\s*(-?\d+(?:[.,]\d+)?)", re.I)
_FLOW = re.compile(r"Flow\s*(?:\.\s*Nominal)?\s*[:=]\s*(-?\d+(?:[.,]\d+)?)", re.I)
_CURVE = re.compile(r"Curve\s*[:=]\s*(\d+)", re.I)
_RUN = re.compile(r"Run\s*Time\s*[:=]\s*(\d+(?:[.,]\d+)?)", re.I)
_EQ = re.compile(r"%\s*([AB])\d?\s*_?\s*Equate\s*[:=]\s*\"?([^\"\n;]+)", re.I)


def parse_chromeleon(text: str) -> dict:
    """Gradient from the text of a Chromeleon instrument method (``PumpModule.Pump.%B.Value: 60.0 [%]``, ``Flow.Nominal``, ``Curve: 5``, rows that start with the time in min).

    Tolerant: any module name, a statement on the row of the time or on the following lines, values with a decimal comma, broken characters.
    """
    lines = _clean(text).split("\n")
    rows: dict[float, dict] = {}
    initial: dict = {}
    t_cur = None
    run_time, solv = None, {}
    for ln in lines:
        m = _RUN.search(ln)
        if m and run_time is None:
            run_time = _num(m.group(1))
        for e in _EQ.finditer(ln):
            solv.setdefault(e.group(1).lower(), e.group(2).strip())
        m = _TIME.match(ln)
        if m and (_B.search(ln) or _FLOW.search(ln) or not any(w in ln for w in ("Run", "Time", ":"))):
            t_cur = _num(m.group(1))
        slot = rows.setdefault(t_cur, {}) if t_cur is not None else initial
        for rx, key, cast in ((_B, "b", float), (_FLOW, "flow", float), (_CURVE, "curve", int)):
            m2 = rx.search(ln)
            if m2:
                v = _num(m2.group(1))
                if v is not None:
                    slot[key] = cast(v)
    times = sorted(t for t in rows if "b" in rows[t] or "flow" in rows[t])
    pts, b, flow = [], initial.get("b"), initial.get("flow")
    if times and times[0] > 0 and b is not None:
        pts.append({"t": 0.0, "b": b, **({"flow": flow} if flow is not None else {})})
    for t in times:
        r = rows[t]
        b, flow = r.get("b", b), r.get("flow", flow)
        if b is None:
            continue
        p = {"t": t, "b": b}
        if flow is not None:
            p["flow"] = flow
        if "curve" in r:
            p["curve"] = r["curve"]
        pts.append(p)
    if len(pts) < 2:
        raise UserError("gradient.err.noScript", {}, "no gradient found in the text")
    return gradient_model("raw", pts, run_time, {"a": solv.get("a"), "b": solv.get("b")} if solv else None)


def parse_gradient_csv(text: str) -> dict:
    """Gradient from a CSV ``time;%B[;flow]`` (separator ; , or tab; decimal comma or point). A step is two rows with the same time."""
    body = [ln for ln in _clean(text).split("\n") if ln.strip()]
    if not body:
        raise UserError("gradient.err.empty", {}, "empty text")
    sep = ";" if ";" in text else "\t" if "\t" in text else ","
    pts = []
    for i, ln in enumerate(body, 1):
        f = [x.strip() for x in ln.split(sep)]
        t, b = (_num(f[0]), _num(f[1])) if len(f) >= 2 else (None, None)
        if t is None or b is None:
            if i == 1 and not pts:         # header row
                continue
            raise UserError("gradient.err.row", {"line": i}, f"line {i} is not 'time;%B'")
        if not 0 <= b <= 100:
            raise UserError("gradient.err.bRange", {"line": i}, f"line {i}: %B must be between 0 and 100")
        if pts and t < pts[-1]["t"]:
            raise UserError("gradient.err.order", {"line": i}, f"line {i}: times must not decrease")
        p = {"t": t, "b": b}
        if len(f) >= 3 and _num(f[2]) is not None:
            p["flow"] = _num(f[2])
        pts.append(p)
    if len(pts) < 2:
        raise UserError("gradient.err.few", {}, "a gradient needs at least two points")
    return gradient_model("csv", pts)


COURSE = [(0, 10), (2, 10), (12, 60), (13, 70), (15, 100), (17, 100), (17.1, 10), (20, 10)]


def course_gradient() -> dict:
    """The gradient of the course: chosen by hand by the student (never a default of the program)."""
    return gradient_model("course", [{"t": t, "b": b} for t, b in COURSE], 20.0)


# ---------------------------------------------------------------- delay between two detectors in series (PDA before the mass spectrometer)
def _grid(t, y, t0, t1, dt):
    g = np.arange(t0, t1, dt)
    return g, np.interp(g, t, y)


def _flatten(y, width: int):
    """The trace without its slow baseline (running median) and scaled to unit spread: only the peaks are left to compare."""
    y = np.asarray(y, dtype=float)
    k = max(3, width | 1)
    pad = np.pad(y, k // 2, mode="edge")
    idx = np.arange(0, len(y), 8)           # the running median is taken every 8th point and interpolated: the baseline is slow
    base = np.interp(np.arange(len(y)), idx, [np.median(pad[i:i + k]) for i in idx])
    z = y - base
    s = np.std(z)
    return z / s if s > 0 else z


def detector_delay(t_a, y_a, t_b, y_b, max_lag: float = 0.6, dt: float | None = None) -> dict | None:
    """How much later the trace B (mass spectrometer, in series after the PDA A) shows the same peaks: ``{"delay": min, "peaks": n, "corr": 0-1}``.

    Cross-correlation of the two traces on a common grid after removing the baseline, parabolic refinement of the maximum; ``peaks`` counts the strongest
    peaks of A that have a peak of B within two grid steps once the delay is applied (the quality shown to the student). None when the traces share too little time.
    """
    t_a, y_a, t_b, y_b = (np.asarray(v, dtype=float) for v in (t_a, y_a, t_b, y_b))
    if len(t_a) < 20 or len(t_b) < 20:
        return None
    t0, t1 = max(t_a[0], t_b[0]) + max_lag, min(t_a[-1], t_b[-1]) - max_lag
    if t1 - t0 < 1.0:
        return None
    dt = dt or max(float(np.median(np.diff(t_b))), float(np.median(np.diff(t_a))), 0.002) / 2
    _, a = _grid(t_a, y_a, t0 - max_lag, t1 + max_lag, dt)
    _, b = _grid(t_b, y_b, t0 - max_lag, t1 + max_lag, dt)
    w = int(0.5 / dt)
    a, b = _flatten(a, w), _flatten(b, w)
    n = int(max_lag / dt)
    seg = slice(n, len(a) - n)
    A = a[seg]
    lags = np.arange(-n, n + 1)
    cc = np.array([float(np.dot(A, b[seg.start + L:seg.stop + L])) for L in lags]) / (np.linalg.norm(A) * np.linalg.norm(b[seg]) + 1e-12)
    i = int(np.argmax(cc))
    if 0 < i < len(cc) - 1:                       # parabola through the three points around the maximum
        d = cc[i - 1] - 2 * cc[i] + cc[i + 1]
        off = 0.5 * (cc[i - 1] - cc[i + 1]) / d if d != 0 else 0.0
    else:
        off = 0.0
    delay = float((lags[i] + off) * dt)
    # peaks of A found in B at the delay
    def tops(v, k):
        idx = [j for j in range(1, len(v) - 1) if v[j] > v[j - 1] and v[j] >= v[j + 1] and v[j] > 2.0]
        return sorted(idx, key=lambda j: -v[j])[:k]
    pa, pb = tops(A, 8), tops(b[seg], 40)
    L = int(round(delay / dt))
    matched = sum(1 for j in pa if any(abs((j + L) - q) <= 2 for q in pb))
    return {"delay": round(delay, 4), "peaks": matched, "corr": round(float(cc[i]), 3)}
