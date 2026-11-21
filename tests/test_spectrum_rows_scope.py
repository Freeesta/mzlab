"""The extra rows of the spectrum card (notes, retention times, CE, sample) and the wrapped header line belong to the MS² card only.
Static guard for the #3435 slip: a selector scoped to every `.pnl.spec` made them show on MS¹ cards. Real behaviour: e2e_spettro_righe."""
import re
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
ROW_SELECTOR = re.compile(r"\.pnl\.spec(?!\.ms2c)[^{,]*(?:\.srows|\.sr\b|\.sr-|\.srl)|\.pnl\.spec(?!\.ms2c) \.hd\{flex-wrap:wrap|\.pnl\.spec(?!\.ms2c) \.hd \.ctl\{display:flex;flex:1 1 100%")


def test_row_css_is_scoped_to_ms2_cards():
    css = (WEB / "app.css").read_text(encoding="utf-8")
    assert ".pnl.spec.ms2c .srows" in css
    bad = ROW_SELECTOR.findall(css)
    assert not bad, f"spectrum row CSS not scoped to .ms2c: {bad}"


def test_rows_script_is_gated_on_ms2():
    js = (WEB / "spettro-righe.js").read_text(encoding="utf-8")
    assert 'classList.toggle("ms2c"' in js and "p.level > 1" in js
    assert js.index("if (!on)") < js.index('box.className = "srows"'), "rows are built before the MS² gate"
