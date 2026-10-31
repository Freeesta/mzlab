"""Leaf decoration: only the side-panel hint keeps it (decorative, swaying just when motion is allowed); the drop area has none and no dead CSS is left."""
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
    assert "leaf-fall" not in CSS and "leaf-drop" not in CSS


def test_animation_only_when_motion_allowed():
    outside = re.sub(r"@media \(prefers-reduced-motion:no-preference\)\{.*?\}\}\n", "", CSS, flags=re.S)
    assert "leaf-sway 6s" not in outside
    assert "prefers-reduced-motion:no-preference){:root:not([data-a11y-reduce]) #fl-hint .leaf" in CSS
