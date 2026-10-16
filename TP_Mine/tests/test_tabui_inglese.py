"""Tabui is English only: no Italian in the strings of its interface (TP_Mine/js, TP_Mine/lazy, TP_Mine/files), in its data shown to the user (rule profiles, the column
names of composti.csv), in the `lang/` catalogs (Tabui has no key there) and no bare «formula» column. Internal values of the engine ('forte', 'progenitore'...) are not text."""
import csv, json, re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TP = ROOT / "TP_Mine"
ITALIAN = re.compile(
    r"\b(nessun[a-z]*|della|delle|degli|dello|nella|nelle|sono|questo|questa|perch[eé]|anche|pi[uù]|pu[oò]|oppure|tabella|banco|scegli|apri|chiudi|salva|scarica|errore|avviso|"
    r"campion[ei]|file aperti|lacune|radicali|ozono|metabolismo|fase I|non (?!-)(?=[a-zàèéìòù]))\b", re.I)
LITERAL = re.compile(r'"((?:[^"\\\n]|\\.)*)"|\'((?:[^\'\\\n]|\\.)*)\'|`((?:[^`\\]|\\.)*)`', re.S)


def strip_comments(src):
    src = re.sub(r"/\*.*?\*/", "", src, flags=re.S)
    return "\n".join(re.sub(r"(^|\s)//.*$", "", ln) if "//" in ln and not re.search(r"https?://", ln) else ln for ln in src.split("\n"))


def literals(path):
    for m in LITERAL.finditer(strip_comments(path.read_text(encoding="utf-8"))):
        yield next(g for g in m.groups() if g is not None)


def test_no_italian_in_tabui_strings():
    bad = []
    for sub in ("js", "lazy", "files"):
        for p in sorted((TP / sub).glob("*.js")):
            for s in literals(p):
                text = re.sub(r"<[^>]*>", " ", s)       # the markup (ids, classes, routes) is code, not text
                if " " in text.strip() and ITALIAN.search(text):
                    bad.append(f"{p.relative_to(ROOT)}: {s[:80]}")
    assert not bad, "Italian in Tabui:\n" + "\n".join(bad[:30])


def test_rule_profiles_in_english():
    prof = json.loads((TP / "data" / "regole_tp_2.json").read_text(encoding="utf-8"))
    def names(o):
        if isinstance(o, dict):
            for k, v in o.items():
                if k == "nome" and isinstance(v, str):
                    yield v
                else:
                    yield from names(v)
        elif isinstance(o, list):
            for v in o:
                yield from names(v)
    bad = [n for n in names(prof) if ITALIAN.search(n)]
    assert not bad, bad


def test_composti_has_no_bare_formula_column():
    with open(TP / "data" / "composti.csv", encoding="utf-8", newline="") as f:
        head = next(csv.reader(f))
    assert "formula" not in head and "neutral_formula" in head, head


def test_tabui_has_no_lang_keys():
    for lang in ("it", "en"):
        src = (ROOT / "mzlab" / "web" / "lang" / f"{lang}.js").read_text(encoding="utf-8")
        assert not re.search(r'"tpmine\.', src), f"lang/{lang}.js has keys of Tabui"
