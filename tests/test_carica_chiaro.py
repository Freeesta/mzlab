"""The loading card tells ONE story: plain words (no «scan index» / «mzML» in the phase lines) and a row for every file of the batch from the start (the count over the table = the headline)."""
import re
from pathlib import Path

W = Path(__file__).resolve().parent.parent / "mzlab" / "web"


def _cat(lang):
    return dict(re.findall(r'"([\w.]+)":\s*"((?:[^"\\]|\\.)*)"', (W / "lang" / lang / "conv.js").read_text(encoding="utf-8")))


def test_plain_words_in_the_card():
    for lang in ("en", "it"):
        c = _cat(lang)
        for k in ("conv.phase.converting", "conv.phase.indexing", "conv.detail.wait", "conv.beat", "conv.beat.indexing"):
            assert not re.search(r"scan index|indice|mzML|indicizz|index", c[k], re.I), (lang, k, c[k])


def test_rows_registered_before_the_first_conversion():
    src = (W / "shell.js").read_text(encoding="utf-8")
    body = src[src.index("async function moduloToMzml"):src.index("out.panel=g;return out}")]
    assert body.index('CONVROWS.add(nm,"wait")') < body.index("let i=0;")
    assert 'CONVROWS.set(nm,"open")' in body and 'CONVROWS.add(nm,"open")' not in body


def test_indexing_detail_does_not_repeat_the_count():
    src = (W / "conversione-carta.js").read_text(encoding="utf-8")
    i = src.index('if (stage === "indexing") d =')
    assert "CONVROWS.ready()" not in src[i:i + 200]
