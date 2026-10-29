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
