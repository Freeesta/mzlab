"""The MS² spectrum card keeps only the wrapped header line (fragments controls, class .ms2c); the info rows (notes, retention times, CE, sample) are gone.
Static guard: no CSS, script or i18n key of those rows comes back, and the wrapped header stays scoped to .ms2c (the #3435 slip: every `.pnl.spec`). Real behaviour: e2e_spettro_righe."""
import re
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
HEADER_SELECTOR = re.compile(r"\.pnl\.spec(?!\.ms2c) \.hd\{flex-wrap:wrap|\.pnl\.spec(?!\.ms2c) \.hd \.ctl\{display:flex;flex:1 1 100%")
ROW_REMNANTS = re.compile(r"\.srows|\.sr\b(?!-only)|\.sr-(?!only)|\.srl\b")
GONE_KEYS = ("srow.notes", "srow.rt", "srow.ce", "srow.sample")


def test_wrapped_header_is_scoped_to_ms2_cards():
    css = (WEB / "app.css").read_text(encoding="utf-8")
    assert ".pnl.spec.ms2c .hd{flex-wrap:wrap" in css
    assert not HEADER_SELECTOR.findall(css), "wrapped spectrum header not scoped to .ms2c"


def test_row_css_is_gone():
    css = (WEB / "app.css").read_text(encoding="utf-8")
    assert not ROW_REMNANTS.findall(css), f"CSS of the removed spectrum rows is back: {ROW_REMNANTS.findall(css)}"


def test_rows_script_builds_no_rows_and_is_gated_on_ms2():
    js = (WEB / "spettro-righe.js").read_text(encoding="utf-8")
    assert 'classList.toggle("ms2c"' in js and "p.level > 1" in js
    assert "srows" not in js and "createElement" not in js, "the rows script builds elements again"
    assert not any(k in js for k in GONE_KEYS)


def test_row_keys_are_gone_from_the_language_files():
    for f in (WEB / "lang").rglob("*"):
        if f.is_file() and f.suffix in (".js", ".json"):
            text = f.read_text(encoding="utf-8")
            for k in GONE_KEYS:
                assert f'"{k}' not in text, f"{k} still in {f.relative_to(WEB)}"
