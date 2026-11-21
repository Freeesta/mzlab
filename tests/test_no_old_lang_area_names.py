"""The lang area files are English (`lang/en/errors-*.js`, `draw-*.js`, ...): no Italian area name is left in a path, a loader list or a doc."""
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OLD = ["altre-a11y-fair", "altre-feat-lst", "altre-lv-predef", "altre-press-xic", "altre-xicbtn-zoom", "errori-a-m", "errori-file", "errori-flow", "errori-llm", "errori-n-w", "errori-*"]
OLD_PATH = re.compile(r"lang/(?:en|<L>|\*)/(?:grafo-msn|librerie|canali|comandi|mappe|tabpicchi|valida|disegno(?:-cip|-evidenza|-sottostruttura|-verifica)?)\.js")
SKIP = {"tests/test_no_old_lang_area_names.py", "docs/agenti/english-remaining.md"}


def test_old_area_names_are_gone():
    files = subprocess.run(["git", "ls-files"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.split("\n")
    hits = []
    for f in files:
        if not f or f in SKIP or "/vendor/" in f or f.startswith("mzlab/web/esempi/") or not f.endswith((".py", ".rs", ".md", ".yml", ".json", ".sh", ".js", ".toml", ".html")):
            continue
        try:
            text = (ROOT / f).read_text(encoding="utf8")
        except (OSError, UnicodeDecodeError):
            continue
        for n, line in enumerate(text.splitlines(), 1):
            if OLD_PATH.search(line) or any(o in line for o in OLD):
                hits.append(f"{f}:{n}")
    assert not hits, hits[:20]


def test_area_files_are_the_english_ones():
    names = {p.stem for p in (ROOT / "mzlab/web/lang/en").glob("*.js")}
    assert not names & {"errori-flow", "altre-lv-predef", "disegno", "librerie", "canali", "comandi", "mappe", "tabpicchi", "valida", "grafo-msn"}
    loader = (ROOT / "mzlab/web/lang/en.js").read_text(encoding="utf8")
    assert all(f'"{n}"' in loader for n in names), sorted(n for n in names if f'"{n}"' not in loader)
