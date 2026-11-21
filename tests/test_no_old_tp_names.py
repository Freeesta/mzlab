"""The TP rules file (regole_tp/3) and the pipeline record (feature_pipeline/2) use English keys; the old Italian ones must not come back."""
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
# keys of regole_tp/2 and feature_pipeline/1 that no other format uses; quoted, as keys or values
OLD = re.compile(r'"(passi_max|livello_previsione|direzione_rt|gruppi_almeno_uno|almeno_uno|piu_polare|meno_polare|fotocatalisi|ozono_cloro|metabolismo_I_II|registro_sha256|congelato|eredita|richiede)"')
# the two readers that name the old version on purpose, and the tests that feed them an old file
ALLOWED = {
    "tests/test_no_old_tp_names.py", "tests/test_controlla_inglese.py", "tools/english_rename_map.py",
    "crates/mzlab-feat/tests/hypo.rs", "crates/mzlab-feat/tests/hypo_rules2.rs",
}
KINDS = {".js", ".mjs", ".py", ".md", ".json", ".rs", ".yml", ".html"}


def test_old_names_are_gone():
    files = subprocess.run(["git", "ls-files", "-co", "--exclude-standard"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.split("\n")
    hits = []
    for f in files:
        p = ROOT / f
        if f.startswith(("mzlab/web/vendor/", "mzlab/web/esempi/", "graphify-out/", "site/", "tests/golden/attributio/", "tools/valida/")) or p.suffix not in KINDS or f in ALLOWED or not p.is_file():
            continue
        for n, line in enumerate(p.read_text(encoding="utf8", errors="ignore").splitlines(), 1):
            if OLD.search(line):
                hits.append(f"{f}:{n}")
    assert not hits, hits[:20]
