"""The theme snippet is the first thing in the <head> of every page (before any script or stylesheet) and is up to date (tools/tema_testa.py)."""
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import tema_testa  # noqa: E402


def test_snippet_aggiornato():
    assert subprocess.run([sys.executable, str(ROOT / "tools" / "tema_testa.py"), "--check"]).returncode == 0


def test_snippet_prima_di_script_e_stili():
    n = 0
    for p, _ in tema_testa.pages():
        t = p.read_text(encoding="utf-8")
        head = t[:t.index("</head>")]
        i = head.index(tema_testa.OPEN)
        assert i < 120, p
        for m in re.finditer(r'<script\b|<link\b[^>]*stylesheet|<style\b', head):
            if m.start() < head.index(tema_testa.CLOSE):
                assert m.start() > i, p            # only the snippet's own <style>/<script> come before the close marker
            else:
                assert m.start() > head.index(tema_testa.CLOSE), p
        assert 'name="color-scheme"' in head
        n += 1
    assert n > 40


def test_pagina_di_ingresso_generata():
    src = (ROOT / "tools" / "build_site.py").read_text(encoding="utf-8")
    assert "tema_testa" in src
