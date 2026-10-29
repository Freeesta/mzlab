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


def test_table_of_a_loading_repaints_once_and_proposes_once(tmp_path):
    _run("ruoli_coalesce_node.mjs", ["ruoli-carica.js"], tmp_path)


def test_a_wiff_pair_is_parsed_once_and_checked_in_the_cache_before_the_probe(tmp_path):
    s = (WEB / "shell.js").read_text(encoding="utf-8")
    a = s.index("async function wiffToMzml")
    fn = tmp_path / "wiff.js"
    fn.write_text(s[a:s.index("// Rule R", a)], encoding="utf-8")
    r = subprocess.run(["node", str(ROOT / "tests" / "wiff_pair_node.cjs"), str(fn), str(WEB)], capture_output=True, text=True, check=False)
    assert r.returncode == 0, (r.stdout + r.stderr)[-3000:]
    assert r.stdout.strip() == "ok"
