"""The docs folders and files use English names (docs/inventories, literature_cc, release, studies, users, ...); the old Italian paths must not come back."""
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OLD_PATHS = re.compile(
    r"^docs/(inventari|letteratura_cc|rilascio|studi|utenti)/"
    r"|^docs/inventories/TOOLTIP-(disegno|liste|pannelli|spettri)"
    r"|^docs/studies/rosina/(FONTI|SCHEDE|SCHEDA_\w+|WORKFLOW_NOTTE)\.md$|^docs/studies/workflow/PROVE_SU_CARTA\.md$"
    r"|^docs/literature_cc/(RAPPORTO\.md|coppie_isomeri_rt|siti_sperimentali|tp_noti)")
# `documenti/studi/` is a folder of the private data repository, not of this one
OLD_TEXT = re.compile(r"(?<![\w/])docs/(inventari|letteratura_cc|rilascio|studi|utenti)\b|TOOLTIP-(disegno|liste|pannelli|spettri)|SCHEDA_[A-Z]+\.md|SCHEDE\.md|WORKFLOW_NOTTE|PROVE_SU_CARTA|coppie_isomeri_rt|siti_sperimentali|\btp_noti\b")
SKIP = {"tests/test_no_old_docs_names.py", "tests/test_controlla_inglese.py", "tools/english_rename_map.py", "tools/italian_words.tsv"}


def _files():
    out = subprocess.run(["git", "ls-files", "-co", "--exclude-standard"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.split("\n")
    return [f for f in out if f and (ROOT / f).is_file() and not f.startswith(("mzlab/web/vendor/", "mzlab/web/esempi/", "graphify-out/"))]


def test_old_paths_are_gone():
    assert [f for f in _files() if OLD_PATHS.search(f)] == []


def test_old_names_are_not_cited():
    hits = []
    for f in _files():
        if f in SKIP:
            continue
        try:
            text = (ROOT / f).read_text(encoding="utf8")
        except (UnicodeDecodeError, OSError):
            continue
        hits += [f"{f}:{n}" for n, line in enumerate(text.splitlines(), 1) if OLD_TEXT.search(line)]
    assert hits == [], hits[:20]
