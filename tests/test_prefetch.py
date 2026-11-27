"""Copy ahead of the «Choose samples» dialog (mzlab/web/rust-prefetch.js), run by node with a fake OPFS and the engine's copy gate: at most 2 copies at once, untick, X with queued samples, stores kept for a stub, an opening that waits for a copy running. Node is required: the test fails, it never skips."""
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def test_prefetch_gate_untick_and_close():
    r = subprocess.run(["node", str(ROOT / "tests" / "prefetch_node.mjs"), str(ROOT / "mzlab" / "web")], capture_output=True, text=True, check=False)
    assert r.returncode == 0, (r.stdout + r.stderr)[-3000:]
    assert r.stdout.strip() == "ok"
