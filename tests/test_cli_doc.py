"""The command list of docs/agenti/cli.md is the one `mzlab-cli help-md` prints from the command table, and tools/cli_doc.py writes it the same way twice."""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import cli_doc  # noqa: E402


def test_replacing_the_block_keeps_what_is_outside_it():
    text = "before\n<!--cli-->\nold\n<!--/cli-->\nafter\n"
    new = cli_doc.replaced(text, "`a`, `b`\n")
    assert new == "before\n<!--cli-->\n`a`, `b`\n<!--/cli-->\nafter\n"
    assert cli_doc.replaced(new, "`a`, `b`\n") == new


def test_the_command_list_of_the_doc_is_current():
    r = subprocess.run([sys.executable, str(ROOT / "tools/cli_doc.py"), "--check"], cwd=ROOT, capture_output=True, text=True)
    assert r.returncode == 0, r.stdout + r.stderr


def test_the_list_names_every_command_and_old_name():
    block = cli_doc.listing()
    for name in ("features", "flow run", "mol-label", "validate", "channels"):
        assert f"`{name}`" in block
    assert "`channels` (old: `canali`)" in block
    assert "help-md" not in block
