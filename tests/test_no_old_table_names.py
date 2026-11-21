"""The JSON of the table engine (view spec, view, qc/2, report/2) uses English keys; the old Italian ones must not come back."""
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OLD = re.compile(r"\b(normalizza|filtra|min_rilevati|min_altezza|bianco_larga|repliche_min|celle_bianco|istd_esente|bianco_ripiego|area_totale|cv_mediano|m1_su_m|righe_con_picco)\b")
# in the files of the table engine alone (the TP family keeps its own keys until its own rename)
WIDE = re.compile(r"\b(etichette|scartate|tenute|fattori|bianchi|rilevate|mirate|riempite|assenti|conteggi|regola|valore|righe)\b")
WIDE_PATHS = (
    "crates/mzlab-feat/src/view.rs", "crates/mzlab-feat/src/qc.rs", "crates/mzlab-feat/src/report.rs", "crates/mzlab-feat/src/norm.rs",
    "crates/mzlab-feat/src/filters.rs", "crates/mzlab-feat/src/filters_blank.rs", "crates/mzlab-ops/src/run/table/", "crates/mzlab-feat-table-wasm/",
    "TP_Mine/js/36_feature_qc.js", "TP_Mine/js/37_feature_table.js", "TP_Mine/js/38_feature_export.js", "mzlab/web/rapporto-flow.js",
    "tests/test_pipeline_contract.py", "tests/test_feat_table_wasm.py", "crates/mzlab-cli/src/flow_table", "crates/mzlab-cli/src/flow_view_tests.rs",
)
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
            if (OLD.search(line) or (f.startswith(WIDE_PATHS) and WIDE.search(line))) and "error.viewSpecOld" not in line:
                hits.append(f"{f}:{n}")
    assert not hits, hits[:20]
