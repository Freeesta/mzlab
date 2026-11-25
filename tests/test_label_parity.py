"""The label of a sample of a multi-sample `.wiff`: `mzlab-cli features --sample` (`crates/mzlab-cli/src/sample_label.rs`) and the site (`wiffName` of `mzlab/web/conversione.js`, run by node)
give the same name for every sample whose name is not taken: trimmed, the characters `/ \\ : * ? " < > | #` as `_`, at most 120 characters. When two names collide the rules differ on purpose: the site
counts (`S.wiff`, `S_2.wiff`, ...) the files of one batch, the command writes `NAME (FILESTEM #ID)` and says so on stderr (`note.features.columnTie`); the owner decides which one wins, see the PR that added this
file. Fails, never skips, in CI when a tool is missing: `test_the_tools_are_there_in_ci`."""
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest
from cargo_util import cli

ROOT = Path(__file__).resolve().parents[1]
TOOLS = all(shutil.which(x) for x in ("cargo", "node", "python3"))
CI = bool(os.environ.get("GITHUB_ACTIONS"))
pytestmark = pytest.mark.skipif(not TOOLS and not CI, reason="cargo, node or python3 not available")

# names with no comma (the list of `wiff_grande.py --names` is comma-separated); one name per sample of one file
NAMES = ["Sample_A", "  padded  ", "A/B\\:C*D?E\"F<G>H|I", "x" * 150, "with space 1.5", "at@5", "tag#7", "Ünï"]
JS = ROOT / "mzlab" / "web" / "conversione.js"


def test_the_tools_are_there_in_ci():
    assert TOOLS or not CI, "cargo, node and python3 must be installed in CI: this test would not run"


def site(names):
    code = f"const c=require({str(JS)!r});console.log(JSON.stringify({json.dumps(names)}.map((n,i)=>c.wiffName('grande',i+1,true,[],n))))"
    return json.loads(subprocess.run(["node", "-e", code], check=True, capture_output=True, encoding="utf-8").stdout)


def test_the_column_of_a_sample_is_the_name_the_site_gives_its_mzml(tmp_path):
    d = tmp_path / "in"
    made = subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_grande.py"), str(d), "--samples", str(len(NAMES)), "--scans", "40", "--points", "50", "--names", ",".join(NAMES)],
                          capture_output=True, encoding="utf-8")
    assert made.returncode == 0, made.stderr
    ids = ",".join(str(i + 1) for i in range(len(NAMES)))
    plan = json.loads(cli("features", str(d / "grande.wiff"), "--sample", ids, "--dry-run").stdout)
    columns = [m["column"] for m in plan["members"]]
    assert len(columns) == len(NAMES), plan
    want = [n.removesuffix(".mzML") for n in site(NAMES)]
    assert [c.removesuffix(".wiff") for c in columns] == want
