"""Every chapter source mzlab/web/teoria/src/*.md must match the committed HTML next to it (regenerate with
python3 tools/libro/cli.py --src mzlab/web/teoria/src --out mzlab/web/teoria --teoria --pubblico)."""
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
from tools.libro.pagina import converti  # noqa: E402

TEORIA = ROOT / "mzlab" / "web" / "teoria"
FONTI = sorted((TEORIA / "src").glob("[0-9]*.md"))


@pytest.mark.parametrize("fonte", FONTI, ids=lambda f: f.name)
def test_html_generato_aggiornato(fonte):
    atteso = converti(fonte.read_text(encoding="utf-8"), fonte.name, pubblico=True, teoria=True)
    assert (TEORIA / (fonte.stem + ".html")).read_text(encoding="utf-8") == atteso, f"{fonte.name}: HTML out of date, regenerate it"


def test_nessuna_fonte_senza_html():
    assert all((TEORIA / (f.stem + ".html")).exists() for f in FONTI)
