"""Loading in the worker of the engine (mzlab/web/rust-carica.js, rust-nativo.js), run by node on copies of the modules: the room asked for a .wiff sample counts only the members still to write,
fingerprints are found for a shared member already in OPFS, a stale native stub is forgotten, the plain copy is cancellable and releases its room, and an unchanged index is not opened twice."""
import re
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
WEB = ROOT / "mzlab" / "web"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def _copy_modules(dest: Path) -> None:
    todo, seen = ["rust-nativo.js", "rust-carica.js"], set()
    while todo:
        n = todo.pop()
        if n in seen:
            continue
        seen.add(n)
        text = (WEB / n).read_text(encoding="utf-8")
        (dest / n).write_text(text, encoding="utf-8")
        todo += [m for m in re.findall(r'from "\./([\w.-]+\.js)"', text) if (WEB / m).exists()]
    (dest / "package.json").write_text('{"type":"module"}')


def test_loading_helpers_of_the_worker(tmp_path):
    _copy_modules(tmp_path)
    r = subprocess.run(["node", str(ROOT / "tests" / "carica_worker_node.mjs"), str(tmp_path)], capture_output=True, text=True, check=False)
    assert r.returncode == 0, (r.stdout + r.stderr)[-3000:]
    assert r.stdout.strip() == "ok"


def test_native_members_are_read_through_read_only_handles_for_the_call(tmp_path):
    """toMzml and srm of a maker's sample read the OPFS copies through read-only handles (0 FileReaderSync reads, same bytes); a reader kept after close() falls back to the Blob."""
    _copy_modules(tmp_path)
    r = subprocess.run(["node", str(ROOT / "tests" / "lettura_nativi_node.mjs"), str(tmp_path)], capture_output=True, text=True, check=False)
    assert r.returncode == 0, (r.stdout + r.stderr)[-3000:]
    assert r.stdout.strip() == "ok"


def test_every_close_of_an_lr_slot_forgets_the_open_index_gate():
    """lrOpen skips open_index for an index it already opened: whatever closes the engine's slots must make the gate forget, or the next lrOpen finds the slot empty (badIndex, files never ready)."""
    src = (WEB / "rust-worker.js").read_text(encoding="utf-8")
    closers = [ln for ln in src.splitlines() if "engine.close_all()" in ln or re.search(r"engine\.close\(slotOf\(", ln)]
    assert len(closers) >= 3
    assert all("openGate." in ln or "for (const n of [...lrFile.keys()]) engine.close" in ln for ln in closers), closers
