"""The Adducts toolbar icon is square and its button shrinks to fit (icon + label side by side)."""
import re
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
ICONS = (WEB / "icons-modi.js").read_text(encoding="utf-8")
CSS = (WEB / "app.css").read_text(encoding="utf-8")


def square_keys():
    m = re.search(r"const w = \[([^\]]*)\]\.includes\(k\)", ICONS)
    assert m, "square-icon list not found in QICON.get"
    return re.findall(r'"(\w+)"', m.group(1))


def test_square_list_names_real_icons():
    # a key that names no icon silently renders a 24x24 viewBox in a 38x16 box (tiny glyph, wide button)
    for k in square_keys():
        assert re.search(rf"^\s+{k}: `<svg", ICONS, re.M), f"square-list key without icon: {k}"
    assert "ad" in square_keys()


def test_adducts_icon_is_square_viewbox():
    m = re.search(r"^\s+ad: `<svg[^>]*viewBox=\"0 0 24 24\"", ICONS, re.M)
    assert m


def test_toolbar_button_has_no_fixed_width_or_spread():
    rules = [r for r in re.findall(r"([^{}]*#stbar[^{}]*\.tb[^{}]*)\{([^}]*)\}", CSS)
             if "tb-sep" not in r[0] and "tb-car" not in r[0]]
    assert rules
    for sel, body in rules:
        assert not re.search(r"(?<![-\w])width\s*:", body), sel
        assert "space-between" not in body and "flex-grow" not in body and not re.search(r"flex:\s*[1-9]", body), sel
