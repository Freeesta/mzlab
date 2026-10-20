"""Libro converter (tools/libro): golden HTML on the CC0 fixture, determinism, public build, and the errors that must carry file:line."""
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
from tools.libro.errori import LibroError  # noqa: E402
from tools.libro.pagina import converti  # noqa: E402

FIX = ROOT / "tests" / "data" / "libro"
SRC = (FIX / "S1_prova.md").read_text(encoding="utf-8")


def test_golden():
    assert converti(SRC, "S1_prova.md") == (FIX / "S1_prova.html").read_text(encoding="utf-8")


def test_deterministic_and_cli(tmp_path):
    out = []
    for k in (1, 2):
        subprocess.run([sys.executable, str(ROOT / "tools/libro/cli.py"), "--src", str(FIX), "--out", str(tmp_path / f"o{k}")], check=True, capture_output=True)
        out.append((tmp_path / f"o{k}" / "S1_prova.html").read_bytes())
    assert out[0] == out[1] == (FIX / "S1_prova.html").read_bytes()


def test_raw_html_is_escaped():
    h = converti(SRC, "S1_prova.md")
    assert "<script>" not in h.split("<main>")[1] and "&lt;script&gt;" in h


def test_public_build_drops_private_and_refuses_hypothesis():
    assert "12345" in converti(SRC, "S1_prova.md")
    with pytest.raises(LibroError, match=r"S1_prova\.md:1: 1 unresolved"):
        converti(SRC, "S1_prova.md", pubblico=True)
    pub = converti(SRC.replace("[I]", ""), "S1_prova.md", pubblico=True)
    assert "12345" not in pub and "riservato" not in pub


@pytest.mark.parametrize("text,msg", [
    ("# T\n\n| a | b |\n|---|---|\n| 1 | 2 |\n", r":3: a table needs a caption"),
    ("# T\n\n> semplice\n", r":3: a quote must start"),
    ("# T\n\n$$ x\n", r":3: equation \$\$ without closing"),
    ("# T\n\nvedi @eq:nessuna\n", r"points to nothing"),
    ("# T\n\n[x](javascript:alert(1))\n", r":3: unsupported link target"),
    ("# T\n\n<!-- a\nb -->\n", r":3: multi-line"),
    ("# T\n\n![](a.svg){#fig:a}\n", r":3: figure alt"),
    ("# T\n\n<!-- privato -->\nx\n", r":3: <!-- privato -->"),
])
def test_errors_carry_line(text, msg):
    with pytest.raises(LibroError, match=msg):
        converti(text, "X.md")


CAP = """<!-- script: sim-misc.js -->
# Dalla prova {kicker: Capitolo 8}

{.lead} Un riassunto breve.

## Sezione

```html
<span class="eq"><i>x</i> = 1<span class="no">(8.1)</span></span>

<div class="sim" id="sim-a"><h4>Prova</h4></div>
```

::: duo
> [!lr] Quadrupolo
> Testo uno.

> [!hr] Orbitrap
> Testo due.
:::

{.note} Una nota.
"""


def test_teoria_chapter_constructs():
    h = converti(CAP, "08-prova.md", teoria=True)
    assert '<div class="kicker">Capitolo 8</div>\n<h1>Dalla prova</h1>' in h
    assert '<p class="lead">Un riassunto breve.</p>' in h and '<p class="note">Una nota.</p>' in h
    assert '<div class="sim" id="sim-a"><h4>Prova</h4></div>' in h and '<span class="no">(8.1)</span>' in h
    assert '<div class="duo"><div class="box lr"><b>Quadrupolo</b>' in h and '<div class="box hr"><b>Orbitrap</b>' in h
    assert h.rstrip().endswith('<script src="../appname.js"></script><script src="teoria.js"></script><script src="sim-misc.js"></script>\n</body></html>')
    assert "noindex" not in h and "data-esteso" not in h and "· Teoria QqQ lab</title>" in h
    assert converti(CAP, "08-prova.md", teoria=True) == h


@pytest.mark.parametrize("text,msg", [
    ("# T\n\n```html\n<script>x</script>\n```\n", r":3: raw HTML block contains '<script'"),
    ("# T\n\n```html\n<a onclick=\"x\">y</a>\n```\n", r"which is not allowed"),
    ("# T\n\n```html\n<p>a</p>\n", r":3: ```html block without closing"),
    ("# T\n\n::: duo\n> [!lr] a\n> b\n", r":3: ::: duo without closing"),
])
def test_raw_and_duo_errors(text, msg):
    with pytest.raises(LibroError, match=msg):
        converti(text, "X.md")
