"""The undo/redo history module uses English names (HISTORY, HistoryStack, history.js, pushFields, history.* keys); the old Italian ones must not come back."""
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OLD = re.compile(r"\b(STORIA|StoriaPila|pushCampi)\b|(?<![/\w-])storia\.js|[\"'`]storia\.[a-z]")
SKIP = {"tests/test_no_old_history_names.py", "tests/test_controlla_inglese.py", "tools/english_rename_map.py"}
KINDS = {".js", ".mjs", ".py", ".md", ".html", ".json"}


def test_old_names_are_gone():
    files = subprocess.run(["git", "ls-files", "-co", "--exclude-standard"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.split("\n")
    hits = []
    for f in files:
        p = ROOT / f
        if f.startswith(("mzlab/web/vendor/", "mzlab/web/esempi/")) or p.suffix not in KINDS or f in SKIP or not p.is_file():
            continue
        for n, line in enumerate(p.read_text(encoding="utf8", errors="ignore").splitlines(), 1):
            if OLD.search(line):
                hits.append(f"{f}:{n}")
    assert not hits, hits[:20]
