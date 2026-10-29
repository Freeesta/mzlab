"""avanza().set() of mzlab/web/avanzamento.js (run by node): a non-finite fraction must not poison the forward-only maximum of the bar."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

MJS = Path(__file__).resolve().parent / "avanzamento_node.mjs"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def test_non_finite_fraction_is_indeterminate_and_does_not_poison_max():
    out = json.loads(subprocess.run(["node", str(MJS)], check=True, capture_output=True, encoding="utf-8").stdout)
    assert out == [
        ["0.2", "20", "20%"],
        ["NaN", None, "0%"],            # indeterminate, not "NaN"
        ["0.5", "50", "50%"],           # a finite value resumes: max was not poisoned
        ["Infinity", None, "0%"],
        ["null", None, "0%"],
        ["undefined", None, "0%"],
        ["0.4", "50", "50%"],           # still forward only
    ]
