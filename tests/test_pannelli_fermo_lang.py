"""The frozen-spectrum RT label of pannelli.js comes from the lang/ catalog (no hard-coded text)."""
import re
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"


def _catalog(lang):
    return (WEB / "lang" / lang / "more-lv-defaults.js").read_text(encoding="utf-8")


def test_label_is_keyed_not_hardcoded():
    src = (WEB / "pannelli.js").read_text(encoding="utf-8")
    assert "fermo a" not in src
    assert 'I18N.t("panel.rt.held", { rt })' in src


def test_catalog_has_the_english_text():
    en = re.search(r'"panel\.rt\.held": "([^"]+)"', _catalog("en"))
    assert en and "{rt}" in en.group(1) and "fermo" not in en.group(1)
