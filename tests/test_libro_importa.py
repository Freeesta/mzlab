"""Importer HTML -> Libro Markdown: canonical DOM, per-block fallback to a raw block, and the equivalence of the imported chapters with the
pages they replaced (digest of the canonical DOM taken before the import)."""
import hashlib
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
from tools.libro.dom import canon, normalizza, parse  # noqa: E402
from tools.libro.importa import find  # noqa: E402
from tools.libro.importa import importa  # noqa: E402
from tools.libro.pagina import converti  # noqa: E402

TEORIA = ROOT / "mzlab" / "web" / "teoria"
DIGEST = [x.split() for x in (ROOT / "tests/data/libro/equivalenza.txt").read_text(encoding="utf-8").splitlines() if x and not x.startswith("#")]

PAGINA = """<!doctype html>
<html lang="it"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Prova · Teoria QqQ lab</title></head>
<body>
<div class="wrap"><nav id="side"></nav><main>
<div class="kicker">Capitolo 9</div>
<h1>Prova</h1>
<p class="lead">Riassunto.</p>
<h2>Parte</h2>
<p>H<sub>2</sub>O a 10<sup>4</sup> Pa, <i>m/z</i> e <b>forte</b> con <em>enfasi</em>.
<ul><li>uno<li>due <a href="https://x.org/a" target="_blank" rel="noopener">fuori</a></ul>
<table><tr><th>a</th></tr><tr><td class="n">1</td></tr></table>
<p>costo * 2 [x]</p>
</main></div>
</body></html>
"""


def test_canonical_dom_ignores_formatting_not_content():
    a = "<p>uno  due\n<b>tre</b></p>\n\n<ul>\n<li>x</li>\n</ul>"
    assert normalizza(a) == normalizza("<p>uno due <b>tre</b></p><ul><li>x</li></ul>")
    assert normalizza("<p>a</p>") != normalizza("<p>b</p>")
    assert normalizza('<p class="x" id="y">a</p>') == normalizza('<p id="y" class="x">a</p>')
    assert normalizza("<p>a<p>b") == normalizza("<p>a</p><p>b</p>")


def test_import_native_and_raw_fallback(monkeypatch):
    from tools.libro import grezzo
    monkeypatch.setattr(grezzo, "ammesso", lambda f: True)
    md, raw = importa(PAGINA, "09-prova.md")
    assert "H~2~O a 10^4^ Pa, *m/z* e **forte** con _enfasi_." in md
    assert "- uno\n- due [fuori](https://x.org/a)" in md and "{.lead} Riassunto." in md
    # the table and the paragraph with a literal * stay verbatim
    assert raw == 2 and md.count("```html") == 2
    gen = converti(md, "09-prova.md", pubblico=True, teoria=True)
    assert canon(find(parse(gen), "main")) == canon(find(parse(PAGINA), "main"))


@pytest.mark.parametrize("pagina,digest", DIGEST, ids=[d[0] for d in DIGEST])
def test_imported_chapter_is_equivalent_to_the_original(pagina, digest):
    atteso = hashlib.sha256(normalizza((TEORIA / pagina).read_text(encoding="utf-8")).encode()).hexdigest()
    assert atteso == digest, f"{pagina}: the page differs from the chapter that was imported (edit on purpose? delete its line in equivalenza.txt)"


def test_inline_snippet_names_are_a_closed_list():
    from tools.libro.errori import LibroError
    from tools.libro.pagina import converti

    ok = converti("<!-- inline: modi -->\n# T {kicker: K}\n", "99-x.md", pubblico=True, teoria=True)
    assert "QMODI.all()" in ok
    with pytest.raises(LibroError):
        converti("<!-- inline: nope -->\n# T {kicker: K}\n", "99-x.md", pubblico=True, teoria=True)
