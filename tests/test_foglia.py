"""Falling-leaf decoration: decorative only, animated just when motion is allowed, no script added."""
import re
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
CSS = (WEB / "app.css").read_text(encoding="utf-8")
HTML = (WEB / "index.html").read_text(encoding="utf-8")


def test_leaf_svg_is_decorative():
    m = re.search(r'<svg class="leaf leaf-fall"[^>]*>', HTML)
    assert m and 'aria-hidden="true"' in m.group(0) and 'focusable="false"' in m.group(0)


def test_animation_only_when_motion_allowed():
    outside = re.sub(r"@media \(prefers-reduced-motion:no-preference\)\{.*?\}\}\n", "", CSS, flags=re.S)
    assert "leaf-drop 16s" not in outside and "leaf-sway 6s" not in outside
    assert "prefers-reduced-motion:no-preference){:root:not([data-a11y-reduce]) #drop .leaf-fall" in CSS


def test_leaf_out_of_flow_and_theme_variables():
    seg = CSS[CSS.index("#drop .leaf-fall{position:absolute"):][:300]
    assert "pointer-events:none" in seg and "var(--accent)" in seg and "#" not in re.sub(r"#drop[^{]*", "", seg.split("}")[0].split("{")[1])
