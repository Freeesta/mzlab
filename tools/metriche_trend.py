"""Trend logic and report of tools/metriche.py: flatten the merged commit records into named series, compare the last value with 7 and 30 days
earlier (better/worse/flat with a noise threshold), print a table and write one self-contained HTML page with inline SVG sparklines.
Standard library only, no network."""
from __future__ import annotations

import html
import re
from datetime import datetime, timedelta

REL_NOISE = 0.03          # a relative change under 3 % is noise ...
ABS_NOISE = {"count": 1.0, "pct": 0.5}   # ... and so is less than this many units (points of headroom, runs)
LOWER, HIGHER = "lower", "higher"


def ts(s: str) -> datetime:
    return datetime.fromisoformat(s.replace("Z", "+00:00"))


def kind_of(name: str) -> tuple[str | None, str]:
    """(better direction, noise unit) of a series from its name; None = tracked without a verdict (unknown meaning)."""
    if re.search(r"(_per_s|_x|speedup)$", name):
        return HIGHER, "rel"
    if name.endswith("head_pct") or name.endswith("_pct") and "head" in name:
        return HIGHER, "pct"
    if re.search(r"(failed|flaky)", name):
        return LOWER, "count"
    if re.search(r"(^tests\.|merged_24h|merged_window)", name):
        return HIGHER, "rel"
    if re.search(r"(_ms|_s|_mb|_b|[._]raw|[._]gz|[._]br|wall|median|duration)$", name):
        return LOWER, "rel"
    return None, "rel"


def flatten(rec: dict) -> dict[str, float]:
    """Scalar series of one commit record: totals instead of 50 modules, plus the tightest wasm headroom and every bench number."""
    out: dict[str, float] = {}
    w = rec.get("wasm") or {}
    if w:
        for k in ("raw", "gz", "br"):
            vals = [m[k] for m in w.values() if m.get(k) is not None]
            if vals and len(vals) == len(w):
                out[f"wasm.total_{k}"] = float(sum(vals))
        heads = [m["head_pct"] for m in w.values() if m.get("head_pct") is not None]
        if heads:
            out["wasm.min_head_pct"] = min(heads)
    s = rec.get("startup") or {}
    for k in ("raw", "gz", "head_pct"):
        if k in s:
            out[f"startup.{k}" if k != "head_pct" else "startup.head_pct"] = float(s[k])
    for k, v in (rec.get("tests") or {}).items():
        out[f"tests.{k}"] = float(v)
    ci = rec.get("ci") or {}
    if ci.get("complete"):
        for k in ("wall_s", "failed_runs", "flaky_runs", "merged_24h", "open_to_merge_median_s", "pr_open_to_merge_s"):
            if ci.get(k) is not None:
                out[f"ci.{k}"] = float(ci[k])
        if ci.get("required"):
            out["ci.required_max_s"] = float(max(v for v in ci["required"].values() if v is not None))
    for sec in ("bench", "manual"):
        for k, v in (rec.get(sec) or {}).items():
            out[f"{sec}.{k}"] = float(v)
    return out


def series(commits: list[dict]) -> dict[str, list[tuple[datetime, float]]]:
    out: dict[str, list[tuple[datetime, float]]] = {}
    for rec in commits:
        for k, v in flatten(rec).items():
            out.setdefault(k, []).append((ts(rec["ts"]), v))
    return out


def before(points: list[tuple[datetime, float]], days: int) -> float | None:
    """Value of the latest point at least `days` before the last one (None when the history is shorter)."""
    cut = points[-1][0] - timedelta(days=days)
    older = [v for t, v in points if t <= cut]
    return older[-1] if older else None


def verdict(name: str, old: float | None, new: float) -> str:
    """better / worse / flat / n/a (no baseline) / tracked (no direction for this metric)."""
    if old is None:
        return "n/a"
    direction, unit = kind_of(name)
    if direction is None:
        return "tracked"
    delta = new - old
    small = abs(delta) < ABS_NOISE.get(unit, 0.0) or (unit == "rel" and abs(delta) <= REL_NOISE * abs(old))
    if small or delta == 0:
        return "flat"
    return "better" if (delta < 0) == (direction == LOWER) else "worse"


def analyse(commits: list[dict]) -> list[dict]:
    rows = []
    for name, pts in sorted(series(commits).items()):
        last = pts[-1][1]
        row = {"name": name, "last": last, "points": [v for _, v in pts], "n": len(pts)}
        for d in (7, 30):
            old = before(pts, d)
            row[f"d{d}"] = None if old is None else last - old
            row[f"v{d}"] = verdict(name, old, last)
        rows.append(row)
    return rows


def fmt(v: float | None) -> str:
    if v is None:
        return "-"
    return f"{v:,.0f}" if abs(v) >= 1000 else f"{v:.2f}".rstrip("0").rstrip(".")


def text_report(rows: list[dict]) -> str:
    if not rows:
        return "No metrics recorded yet."
    w = max(len(r["name"]) for r in rows)
    lines = [f"{'metric':<{w}}  {'last':>12}  {'d7':>10} {'7d':<7}  {'d30':>10} {'30d':<7}  n"]
    for r in rows:
        lines.append(f"{r['name']:<{w}}  {fmt(r['last']):>12}  {fmt(r['d7']):>10} {r['v7']:<7}  {fmt(r['d30']):>10} {r['v30']:<7}  {r['n']}")
    for label, key in (("7 days", "v7"), ("30 days", "v30")):
        for what in ("better", "worse"):
            names = [r["name"] for r in rows if r[key] == what]
            lines.append(f"Compared with {label} ago, {what.upper()}: {', '.join(names) if names else 'none'}")
    return "\n".join(lines)


def sparkline(values: list[float], w: int = 140, h: int = 30) -> str:
    if len(values) < 2:
        return ""
    lo, hi = min(values), max(values)
    span = (hi - lo) or 1.0
    pts = " ".join(f"{i * (w - 4) / (len(values) - 1) + 2:.1f},{h - 3 - (v - lo) / span * (h - 6):.1f}" for i, v in enumerate(values))
    return f'<svg viewBox="0 0 {w} {h}" width="{w}" height="{h}" role="img" aria-label="trend of {len(values)} values"><polyline points="{pts}" fill="none" stroke="var(--line)" stroke-width="1.6"/></svg>'


def html_report(rows: list[dict], generated: str) -> str:
    tr = "".join(
        f"<tr><th scope='row'>{html.escape(r['name'])}</th><td>{fmt(r['last'])}</td><td>{sparkline(r['points'][-60:])}</td>"
        f"<td>{fmt(r['d7'])} <b class='{r['v7']}'>{r['v7']}</b></td><td>{fmt(r['d30'])} <b class='{r['v30']}'>{r['v30']}</b></td><td>{r['n']}</td></tr>" for r in rows)
    good = ", ".join(r["name"] for r in rows if r["v7"] == "better") or "none"
    bad = ", ".join(r["name"] for r in rows if r["v7"] == "worse") or "none"
    return f"""<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>mzLab metrics</title>
<style>:root{{--bg:#fff;--fg:#1a1a1a;--line:#0072B2;--bd:#ccc;--better:#006b4f;--worse:#a83200}}
@media (prefers-color-scheme:dark){{:root{{--bg:#16181c;--fg:#e8e8e8;--line:#56B4E9;--bd:#444;--better:#4fd1a5;--worse:#ff9a6b}}}}
body{{background:var(--bg);color:var(--fg);font:15px system-ui,sans-serif;margin:0 auto;padding:16px;max-width:1000px}}
table{{border-collapse:collapse;width:100%;display:block;overflow-x:auto}}th,td{{border-bottom:1px solid var(--bd);padding:4px 8px;text-align:right;white-space:nowrap}}th{{text-align:left}}
.better{{color:var(--better)}}.worse{{color:var(--worse)}}</style></head><body>
<h1>mzLab metrics</h1><p>Generated {html.escape(generated)}. Change against 7 and 30 days earlier; a change under {REL_NOISE:.0%} (or one unit of count/point) is flat. Words, not colours, carry the verdict.</p>
<p><b>Better than 7 days ago:</b> {html.escape(good)}.<br><b>Worse than 7 days ago:</b> {html.escape(bad)}.</p>
<table><thead><tr><th>Metric</th><th>Last</th><th>Trend</th><th>vs 7 days</th><th>vs 30 days</th><th>Points</th></tr></thead><tbody>{tr}</tbody></table></body></html>"""
