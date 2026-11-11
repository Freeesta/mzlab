"""A readers module after a wasm trap (mzlab/web/carica-modulo.js), in the readers worker (lettori-worker.js) and in the engine's sessions of a `.raw` (lettore-esterno.js): the broken instance is dropped and the next request loads a fresh one; a plain error keeps it (tests/lettori_trap_node.mjs)."""
import re
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
WEB = ROOT / "mzlab" / "web"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def test_a_trapped_readers_module_is_loaded_afresh(tmp_path):
    todo, seen = ["lettori-worker.js", "lettore-esterno.js"], set()
    while todo:
        n = todo.pop()
        if n in seen:
            continue
        seen.add(n)
        text = (WEB / n).read_text(encoding="utf-8")
        (tmp_path / n).write_text(text, encoding="utf-8")
        todo += [m for m in re.findall(r'from "\./([\w.-]+\.js)"', text) if (WEB / m).exists()]
    (tmp_path / "package.json").write_text('{"type":"module"}')
    glue = ROOT / "tests" / "lettori_trap_glue.js"
    r = subprocess.run(["node", str(ROOT / "tests" / "lettori_trap_node.mjs"), str(tmp_path), str(glue)], capture_output=True, text=True, check=False)
    assert r.returncode == 0, (r.stdout + r.stderr)[-3000:]
    assert r.stdout.strip() == "ok"
