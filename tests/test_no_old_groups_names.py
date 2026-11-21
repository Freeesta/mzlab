"""The side-list groups globals and files use English names (GROUPS, GROUPSFOR, GROUPSTAB, groups*.js, TPGROUPS_VIEW, groups/1); the old Italian ones must not come back."""
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OLD = re.compile(r"\b(GRUPPI|GRUPPIPER|GRUPPI_ETI|GRUTAB|TPGRUPPI_VIEW|GRUPPI_FIXTURE_WRITE)\b|(?<![/\w-])gruppi(-nome|-per|-tab|-dnd|-vista)?\.js|gruppi/1|gruppi-1|gruppi-vista|gruppi_(vista|fixture)|e2e_gruppi|test_gruppi|data/gruppi")
SKIP = {"tests/test_no_old_groups_names.py", "tests/test_controlla_inglese.py", "tools/english_rename_map.py"}
KINDS = {".js", ".mjs", ".py", ".md", ".html", ".json"}


def test_old_names_are_gone():
    files = subprocess.run(["git", "ls-files"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.split("\n")
    hits = []
    for f in files:
        p = ROOT / f
        if f.startswith(("mzlab/web/vendor/", "mzlab/web/esempi/", "mzlab/web/lang/")) or p.suffix not in KINDS or f in SKIP or not p.is_file():
            continue
        for n, line in enumerate(p.read_text(encoding="utf8", errors="ignore").splitlines(), 1):
            if OLD.search(line):
                hits.append(f"{f}:{n}")
    assert not hits, hits[:20]
