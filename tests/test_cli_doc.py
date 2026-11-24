"""The generated blocks of docs/agenti/cli.md and cli-agenti.md are the ones `mzlab-cli help-md` prints from the command table, and tools/cli_doc.py writes them the same way twice."""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import cli_doc  # noqa: E402


def test_replacing_the_block_keeps_what_is_outside_it():
    text = "before\n<!--cli-->\nold\n<!--/cli-->\nafter\n"
    new = cli_doc.replaced(text, "cli", "`a`, `b`\n")
    assert new == "before\n<!--cli-->\n`a`, `b`\n<!--/cli-->\nafter\n"
    assert cli_doc.replaced(new, "cli", "`a`, `b`\n") == new


def test_the_command_list_of_the_doc_is_current():
    r = subprocess.run([sys.executable, str(ROOT / "tools/cli_doc.py"), "--check"], cwd=ROOT, capture_output=True, text=True)
    assert r.returncode == 0, r.stdout + r.stderr


def test_the_list_names_every_command_group():
    block = cli_doc.listing([])
    for name in ("features", "flow run|check", "mol label|anal", "file list", "peak auto|auto-lr|noise|edges"):
        assert name in block
    assert "old:" not in block
    assert "canali" not in block
    assert "help-md" not in block


def test_the_json_list_and_the_exit_table_come_from_the_binary():
    json_list, exits = cli_doc.listing(["json"]), cli_doc.listing(["exits"])
    assert "`flow run|check|steps|defaults|verify|command`" in json_list and "`features`" in json_list
    assert "| 5 | selector not found" in exits and "| 6 |" in exits
