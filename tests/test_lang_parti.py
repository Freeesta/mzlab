"""The catalog is split in area files (lang/en/<area>.js): the loader lists them all, no key twice."""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import lang_catalog  # noqa: E402

MAX_ROWS = 500


def test_the_catalog_has_area_files_and_keys():
    assert lang_catalog.parti("en") and lang_catalog.catalog("en")


def test_loader_lists_every_area_file_in_order():
    for lang in ("en",):
        src = (lang_catalog.LANG / f"{lang}.js").read_text(encoding="utf-8")
        lista = re.search(r"document\.write\(\[([^\]]*)\]", src).group(1)
        assert re.findall(r'"([^"]+)"', lista) == [p.stem for p in lang_catalog.parti(lang)], lang


def test_area_files_stay_small():
    for lang in ("en",):
        for p in lang_catalog.parti(lang):
            assert len(p.read_text(encoding="utf-8").splitlines()) <= MAX_ROWS, f"{p.name}: split the area ({MAX_ROWS} lines at most)"


def _fake_catalog(tmp_path, monkeypatch, files_per_language):
    """Synthetic lang folder: {lang: {file name: body lines}}."""
    for lang, files in files_per_language.items():
        for name, rows in files.items():
            d = tmp_path / lang
            d.mkdir(exist_ok=True)
            (d / name).write_text(f'I18N.add("{lang}", {{\n' + "\n".join(rows) + "\n});\n", encoding="utf-8")
    monkeypatch.setattr(lang_catalog, "LANG", tmp_path)


def test_duplicate_key_in_the_same_file_fails(tmp_path, monkeypatch):
    _fake_catalog(tmp_path, monkeypatch, {"en": {"a.js": ['  "load.slow": "one",', '  "load.x": "x",', '  "load.slow": "two"']}})
    with __import__("pytest").raises(ValueError, match="load.slow.*twice"):
        lang_catalog.catalog("en")


def test_duplicate_key_in_two_files_fails(tmp_path, monkeypatch):
    _fake_catalog(tmp_path, monkeypatch, {"en": {"a.js": ['  "k.one": "1"'], "b.js": ['  "k.one": "2"']}})
    with __import__("pytest").raises(ValueError, match="k.one"):
        lang_catalog.catalog("en")


def test_controlla_i18n_reports_the_duplicate(tmp_path, monkeypatch):
    import controlla_i18n
    _fake_catalog(tmp_path, monkeypatch, {"en": {"a.js": ['  "k.a": "1",', '  "k.a": "2"']}})
    assert any("twice" in e for e in controlla_i18n.run())


def test_description_in_the_wrong_area_file_is_reported(tmp_path, monkeypatch):
    _fake_catalog(tmp_path, monkeypatch, {"en": {"a.js": ['  "a.x": "{n}"'], "b.js": ['  "b.y": "{n}"']}})
    (tmp_path / "contesto").mkdir()
    (tmp_path / "contesto" / "a.json").write_text('{"a.x": "d", "b.y": "d"}', encoding="utf-8")
    errs = lang_catalog.contesto_problems(lang_catalog.catalog("en"))
    assert errs == ["contesto/a.json: description of b.y belongs in contesto/b.json"]
