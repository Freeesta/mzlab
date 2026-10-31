"""Leaf decoration: only the side-panel hint keeps it (decorative, tilted near-horizontal, falling and swaying just when motion is allowed); the drop area has none and no dead CSS is left."""
import re
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
CSS = (WEB / "app.css").read_text(encoding="utf-8")
HTML = (WEB / "index.html").read_text(encoding="utf-8")


def test_leaf_svg_is_decorative():
    m = re.search(r'<svg class="leaf"[^>]*>', HTML)
    assert m and 'aria-hidden="true"' in m.group(0) and 'focusable="false"' in m.group(0)


def test_drop_area_has_no_leaf():
    drop = HTML[HTML.index('<div id="drop"'):][:900]
    assert "leaf" not in drop
    assert "leaf-drop" not in CSS


def test_animation_only_when_motion_allowed():
    outside = re.sub(r"@media \(prefers-reduced-motion:no-preference\)\{.*?\}\}\n", "", CSS, flags=re.S)
    assert "leaf-fall 9s" not in outside
    assert "prefers-reduced-motion:no-preference){:root:not([data-a11y-reduce]) #fl-hint .leaf{animation:leaf-fall 9s" in CSS


def test_leaf_is_tilted_near_horizontal_and_falls():
    m = re.search(r'<svg class="leaf".*?<g transform="rotate\((-?[\d.]+) ', HTML, re.S)
    assert m and 65 <= abs(float(m.group(1))) <= 90
    kf = re.search(r"@keyframes leaf-fall\{(.*)\}\}", CSS).group(1)
    ys = [int(v) for v in re.findall(r"translate\(-?\d+px,(-?\d+)px\)", kf)]
    xs = [int(v) for v in re.findall(r"translate\((-?\d+)px", kf)]
    assert ys[0] < ys[-1] and min(xs) < 0 < max(xs), "drifts down while swaying side to side"
    assert not re.search(r"(width|height|top|left|margin)\s*:", kf), "transform/opacity only: no layout shift"
