"""Every key a reader writes in `method/1.warnings` has a text (lang/en/errori-*.js, the catalogue the CLI reads too) and is in the
list the Method window translates (`NOTE_KEYS` of metodo-doc.js): a warning nobody can read is a silent loss."""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
# a `key` of `warnings` is a `meta.<word>` literal; `meta.compare` is the id of a Flow step, not a warning
NOT_WARNINGS = {"meta.compare"}


def emitted():
    keys = set()
    for f in (ROOT / "crates").rglob("*.rs"):
        if "target" in f.parts:
            continue
        keys |= set(re.findall(r'"(meta\.[a-z][A-Za-z]+)"', f.read_text(encoding="utf-8", errors="replace")))
    return keys - NOT_WARNINGS


def test_every_method_warning_has_a_text_and_a_place():
    keys = emitted()
    assert {"meta.notDecoded", "meta.missing", "meta.truncated"} <= keys, keys
    page = (ROOT / "mzlab/web/metodo-doc.js").read_text(encoding="utf-8")
    cat = "\n".join(p.read_text(encoding="utf-8") for p in sorted((ROOT / "mzlab/web/lang/en").glob("errori-*.js")))
    for k in sorted(keys):
        m = re.search(rf'\n  "{re.escape(k)}": "([^"\n]*)"', cat)
        assert m, f"{k}: no text in lang/en/errori-*.js"
        assert "{place}" in m[1], f"{k}: the text does not name the place"
    for k in sorted(keys):
        assert f'"{k}"' in page, f"{k}: not in NOTE_KEYS of metodo-doc.js"


def test_the_window_passes_every_placeholder_the_texts_use():
    """The Method window renders each warning with `T(key, {...})`: a `{name}` in a text the call does not fill would print literally."""
    page = (ROOT / "mzlab/web/metodo-doc.js").read_text(encoding="utf-8")
    call = re.search(r"NOTE_KEYS\.includes\(x\.key\) \? T\(x\.key, \{([^}]*)\}\)", page)
    assert call, "the call that renders the warnings moved: update this test"
    given = set(re.findall(r"(\w+)\s*:", call[1]))
    cat = "\n".join(p.read_text(encoding="utf-8") for p in sorted((ROOT / "mzlab/web/lang/en").glob("errori-*.js")))
    for k in sorted(emitted()):
        m = re.search(rf'\n  "{re.escape(k)}": "([^"\n]*)"', cat)
        assert set(re.findall(r"\{(\w+)\}", m[1])) <= given, f"{k}: placeholders {set(re.findall(r'[{](\w+)[}]', m[1]))} not all given ({given})"
