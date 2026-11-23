"""The saved-format ids are English and versioned; the old Italian ids must not come back (an old record is refused, never converted)."""
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OLD = re.compile(
    r"(?<![\w-])(livelli/1|ipotesi/1|riga/1|feature_pipeline-2\.json)(?![\w])"
    r"|riga_tracce|(?<![\w-])riga\.json"  # bare names too: output file names, ids, prose
    r"|\"sources\"\s*:\s*\[[^\]]*\"(regola|sospetti|rottura|rete|famiglia|importata)\""  # old hypothesis source labels in fixtures
    r"|\"rottura\+"  # the old label of a cleavage product
)
# The refusal code and its tests name the superseded feature_pipeline versions on purpose.
OLD_PIPELINE = re.compile(r"feature_pipeline/[12](?!\d)")
PIPELINE_OK = {"mzlab/web/workflow.js", "tests/test_pipeline_recipe.py", "tests/test_no_old_tp_names.py"}
SKIP = {"tests/test_no_old_schema_ids.py", "docs/agenti/english-remaining.md"}
# The old registry id is the input of the refusal test.
REFUSAL_TESTS = {"crates/mzlab-feat/tests/hypo_registry_format.rs"}


def _files():
    out = subprocess.run(["git", "ls-files"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.split("\n")
    for f in out:
        if not f or f in SKIP or "/vendor/" in f or f.startswith(("mzlab/web/esempi/", "docs/studies/")):
            continue
        if f.endswith((".py", ".rs", ".md", ".yml", ".json", ".sh", ".js", ".toml")):
            yield f


def test_old_schema_ids_are_gone():
    hits = []
    for f in _files():
        if f in REFUSAL_TESTS:
            continue
        try:
            text = (ROOT / f).read_text(encoding="utf8")
        except (OSError, UnicodeDecodeError):
            continue
        for n, line in enumerate(text.splitlines(), 1):
            if OLD.search(line) or (f not in PIPELINE_OK and OLD_PIPELINE.search(line)):
                hits.append(f"{f}:{n}")
    assert not hits, hits[:20]


def test_old_schema_file_is_gone():
    assert not (ROOT / "docs/schemi/feature_pipeline-2.json").exists()
    assert (ROOT / "docs/schemi/feature_pipeline-3.json").exists()
