"""The catalog is split in area files (lang/<lang>/<area>.js): same areas in it and en, the loaders list them all, no key twice."""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import lang_catalogo  # noqa: E402

MAX_RIGHE = 500


def test_each_area_has_the_same_keys_in_it_and_en():
    nomi = [p.name for p in lang_catalogo.parti("it")]
    assert nomi and nomi == [p.name for p in lang_catalogo.parti("en")]
    for n in nomi:
        assert set(lang_catalogo.parte(lang_catalogo.LANG / "it" / n)) == set(lang_catalogo.parte(lang_catalogo.LANG / "en" / n)), n


def test_no_key_twice_and_same_keys_overall():
    it, en = lang_catalogo.catalogo("it"), lang_catalogo.catalogo("en")
    assert set(it) == set(en)


def test_loader_lists_every_area_file_in_order():
    for lang in ("it", "en"):
        src = (lang_catalogo.LANG / f"{lang}.js").read_text(encoding="utf-8")
        lista = re.search(r"document\.write\(\[([^\]]*)\]", src).group(1)
        assert re.findall(r'"([^"]+)"', lista) == [p.stem for p in lang_catalogo.parti(lang)], lang


def test_area_files_stay_small():
    for lang in ("it", "en"):
        for p in lang_catalogo.parti(lang):
            assert len(p.read_text(encoding="utf-8").splitlines()) <= MAX_RIGHE, f"{p.name}: split the area ({MAX_RIGHE} lines at most)"


def _catalogo_finto(tmp_path, monkeypatch, file_per_lingua):
    """Synthetic lang folder: {lang: {file name: body lines}}."""
    for lang, files in file_per_lingua.items():
        for nome, righe in files.items():
            d = tmp_path / lang
            d.mkdir(exist_ok=True)
            (d / nome).write_text(f'I18N.add("{lang}", {{\n' + "\n".join(righe) + "\n});\n", encoding="utf-8")
    monkeypatch.setattr(lang_catalogo, "LANG", tmp_path)


def test_duplicate_key_in_the_same_file_fails(tmp_path, monkeypatch):
    _catalogo_finto(tmp_path, monkeypatch, {"it": {"a.js": ['  "load.slow": "uno",', '  "load.x": "x",', '  "load.slow": "due"']}})
    with __import__("pytest").raises(ValueError, match="load.slow.*twice"):
        lang_catalogo.catalogo("it")


def test_duplicate_key_in_two_files_fails(tmp_path, monkeypatch):
    _catalogo_finto(tmp_path, monkeypatch, {"en": {"a.js": ['  "k.one": "1"'], "b.js": ['  "k.one": "2"']}})
    with __import__("pytest").raises(ValueError, match="k.one"):
        lang_catalogo.catalogo("en")


def test_controlla_i18n_reports_the_duplicate(tmp_path, monkeypatch):
    import controlla_i18n
    _catalogo_finto(tmp_path, monkeypatch, {"it": {"a.js": ['  "k.a": "1",', '  "k.a": "2"']}, "en": {"a.js": ['  "k.a": "1"']}})
    assert any("twice" in e for e in controlla_i18n.run())
