"""Leaf decoration: only the side-panel hint keeps it (decorative, drawn horizontal, gliding down once just when motion is allowed); the drop area has none and no dead CSS is left."""
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
    assert "leaf-fall 3.4s" not in outside
    assert "prefers-reduced-motion:no-preference){:root:not([data-a11y-reduce]) #fl-hint .leaf{animation:leaf-fall 3.4s ease-in-out 1 forwards}" in CSS


def test_leaf_is_horizontal_and_glides():
    m = re.search(r'<svg class="leaf"[^>]*viewBox="0 0 (\d+) (\d+)"', HTML)
    assert m and int(m.group(1)) > 1.4 * int(m.group(2)) and "rotate(" not in re.search(r'<svg class="leaf".*?</svg>', HTML, re.S).group(0)
    kf = re.search(r"@keyframes leaf-fall\{(.*)\}\}", CSS).group(1)
    pts = [(int(x), int(y), int(r)) for x, y, r in re.findall(r"translate\((-?\d+)(?:px)?,(-?\d+)(?:px)?\) rotate\((-?\d+)(?:deg)?\)", kf)]
    assert pts[0][1] < pts[-1][1] and pts[-1] == (0, 0, 0), "falls down and settles flat"
    assert max(abs(x) for x, _, _ in pts) <= 45, "the swing fits the 280px minimum panel"
    for (x, _, r) in pts[1:-1]:
        assert x * r < 0, "angle is opposite to the side"
    xs = [x for x, _, _ in pts]
    assert all(abs(xs[i]) > abs(xs[i + 1]) for i in range(1, len(xs) - 1)), "swings are damped"
    assert not re.search(r"(width|height|top|left|margin)\s*:", kf), "transform/opacity only: no layout shift"
