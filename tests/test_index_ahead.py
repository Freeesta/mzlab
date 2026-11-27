"""Index ahead of the «Choose samples» dialog (mzlab/web/rust-index-ahead.js), run by node with a fake OPFS and index pool and the engine's real copy gate: the index of a ticked sample is built after its copy under a neutral name, Open moves it to the final name without asking the pool again, untick and X cancel and remove it, at most 2 builds at once. Node is required: the test fails, it never skips."""
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def test_index_is_built_ahead_adopted_once_and_removed():
    r = subprocess.run(["node", str(ROOT / "tests" / "index_ahead_node.mjs"), str(ROOT / "mzlab" / "web")], capture_output=True, text=True, check=False)
    assert r.returncode == 0, (r.stdout + r.stderr)[-3000:]
    assert r.stdout.strip() == "ok"
