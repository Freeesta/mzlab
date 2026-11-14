"""Every key a reader writes in `metodo/1.avvisi` has a text in both languages (lang/*/errori-*.js, the catalogue the CLI reads too) and is in the
list the Method window translates (`NOTE_KEYS` of metodo-doc.js): a warning nobody can read is a silent loss."""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
# a `chiave` of `avvisi` is a `meta.<word>` literal; `meta.compare` is the id of a Flow step, not a warning
NOT_WARNINGS = {"meta.compare"}


def emitted():
    keys = set()
    for f in (ROOT / "crates").rglob("*.rs"):
        if "target" in f.parts:
            continue
        keys |= set(re.findall(r'"(meta\.[a-z][A-Za-z]+)"', f.read_text(encoding="utf-8", errors="replace")))
    return keys - NOT_WARNINGS


def test_every_method_warning_has_a_text_in_both_languages_and_a_place():
    keys = emitted()
    assert {"meta.nonDecodificato", "meta.mancante", "meta.troncato"} <= keys, keys
    page = (ROOT / "mzlab/web/metodo-doc.js").read_text(encoding="utf-8")
    for lang in ("en", "it"):
        cat = "\n".join(p.read_text(encoding="utf-8") for p in sorted((ROOT / f"mzlab/web/lang/{lang}").glob("errori-*.js")))
        for k in sorted(keys):
            m = re.search(rf'\n  "{re.escape(k)}": "([^"\n]*)"', cat)
            assert m, f"{k}: no text in lang/{lang}/errori-*.js"
            assert "{dove}" in m[1], f"{k} ({lang}): the text does not name the place"
    for k in sorted(keys):
        assert f'"{k}"' in page, f"{k}: not in NOTE_KEYS of metodo-doc.js"
