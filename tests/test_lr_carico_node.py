"""Loading flow of the page, run by node on copies of the modules: mixesResolutions takes the resolution of a plain file from its entry (no lr_items RPC), a mixed file still asks the engine."""
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
WEB = ROOT / "mzlab" / "web"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def _run(script: str, modules: list[str], tmp_path: Path) -> None:
    for m in modules:
        (tmp_path / m).write_text((WEB / m).read_text(encoding="utf-8"), encoding="utf-8")
    (tmp_path / "package.json").write_text('{"type":"module"}')
    r = subprocess.run(["node", str(ROOT / "tests" / script), str(tmp_path)], capture_output=True, text=True, check=False)
    assert r.returncode == 0, (r.stdout + r.stderr)[-3000:]
    assert r.stdout.strip() == "ok"


def test_mixes_resolutions_without_one_rpc_per_file(tmp_path):
    _run("lr_sessione_node.mjs", ["lr-sessione.js"], tmp_path)
