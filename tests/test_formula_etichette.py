"""A «formula» is never alone in an export or in a Tabui label: it is the neutral formula or the one of an ion ([M+H]+ ...)."""
import json, re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BARE = re.compile(r'"(?:formula|Formula)"')


def test_golden_export_heads_have_no_bare_formula():
    for f in (ROOT / "tests/golden/attributio/esportazione").glob("*.json"):
        if f.name == "gradiente.json":      # its identity head is an input of the test, not an export of the program
            continue
        d = json.loads(f.read_text(encoding="utf-8"))
        stack = [d]
        while stack:
            x = stack.pop()
            if isinstance(x, dict):
                h = x.get("head")
                if isinstance(h, list):
                    assert not [c for c in h if isinstance(c, str) and c.strip().lower() == "formula"], f.name
                stack.extend(x.values())
            elif isinstance(x, list):
                stack.extend(x)


def test_tabui_headers_and_labels_have_no_bare_formula():
    for f in sorted((ROOT / "TP_Mine/js").glob("*.js")):
        for n, line in enumerate(f.read_text(encoding="utf-8").splitlines(), 1):
            if f.name == "40_tabui_testi.js" or "head" not in line and "label:" not in line and "placeholder=" not in line and "<th" not in line:
                continue
            if re.search(r'(head: *\[[^\]]*|label: *|placeholder=)"(?:[Ff]ormula)"|\[[^\]]*"(?:Formula|formula)"[^\]]*\]', line) and "head" in line or re.search(r'(label: *|placeholder=)"[Ff]ormula"|<th[^>]*>[Ff]ormula<', line):
                raise AssertionError(f"{f.name}:{n}: bare «Formula»")


def test_catalog_labels_have_no_bare_formula():
    for lang in ("it", "en"):
        t = (ROOT / f"mzlab/web/lang/{lang}.js").read_text(encoding="utf-8")
        for k in ("lst.col.formula", "cx.formula", "draw.labels.formula", "fair.formulaOpt"):
            m = re.search(r'"%s": *"([^"]*)"' % re.escape(k), t)
            assert m and m.group(1).strip().lower() not in ("formula", "formula (optional)", "formula (facoltativa)"), (lang, k)
