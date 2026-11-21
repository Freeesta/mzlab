"""The JSON of the table engine (view spec, view, qc/2, report/2) uses English keys; the old Italian ones must not come back."""
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OLD = re.compile(r"\b(normalizza|filtra|min_rilevati|min_altezza|bianco_larga|repliche_min|celle_bianco|istd_esente|bianco_ripiego|area_totale|cv_mediano|m1_su_m|righe_con_picco)\b")
SKIP = {"tests/test_no_old_table_names.py", "crates/mzlab-feat/src/view.rs", "crates/mzlab-feat/tests/view.rs", "tests/test_controlla_inglese.py", "tools/english_rename_map.py"}
KINDS = {".js", ".mjs", ".py", ".md", ".html", ".json", ".css", ".rs", ".yml"}


def test_old_names_are_gone():
    files = subprocess.run(["git", "ls-files", "-co", "--exclude-standard"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.split("\n")
    hits = []
    for f in files:
        p = ROOT / f
        if f.startswith(("mzlab/web/vendor/", "mzlab/web/esempi/", "graphify-out/", "site/")) or p.suffix not in KINDS or f in SKIP or not p.is_file():
            continue
        for n, line in enumerate(p.read_text(encoding="utf8", errors="ignore").splitlines(), 1):
            if OLD.search(line) and "error.viewSpecOld" not in line:
                hits.append(f"{f}:{n}")
    assert not hits, hits[:20]
