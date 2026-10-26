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
