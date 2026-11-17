"""canon() and isDirty() of mzlab/web/disegno-pulito.js (run by node): the «unsaved changes» state of Draw compares a stable text of the canvas, not event counts."""
import json
import shutil
import subprocess
from pathlib import Path

MJS = Path(__file__).resolve().parent / "disegno_pulito_node.mjs"


def test_dirty_state_compares_a_stable_text_of_the_canvas():
    assert shutil.which("node"), "node is required for this test (CI has it); install it, the test is never skipped"
    out = json.loads(subprocess.run(["node", str(MJS)], check=True, capture_output=True, encoding="utf-8").stdout)
    assert out == [True, False, "not json", False, True, False, True, False, True, False]
