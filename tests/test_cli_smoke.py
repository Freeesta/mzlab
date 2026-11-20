"""tools/cli_smoke.py, the smoke test of the release binaries: passes on a good binary, fails on a bad one or on a different output."""
import stat
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import cli_smoke  # noqa: E402
import pytest  # noqa: E402


def fake(tmp: Path, name: str, version: str = "mzlab-cli 1.0.0 (commit abc1234) mzLab", pattern: str = "A") -> str:
    """A stand-in for mzlab-cli: a script that answers the four commands without input."""
    p = tmp / name
    p.write_text(
        "#!/bin/sh\n"
        f'case "$1 $2" in\n'
        f'  "--version ") echo "{version}";;\n'
        '  "commands --json") echo \'{"commands":[{"path":"x"}]}\';;\n'
        '  "flow steps") echo \'[{"id":"s"}]\';;\n'
        '  "schema --list") echo \'{"schemas":[]}\';;\n'
        f'  "ion pattern") echo "{pattern}";;\n'
        "  *) exit 2;;\nesac\n"
    )
    p.chmod(p.stat().st_mode | stat.S_IXUSR)
    return str(p)


def test_a_good_binary_passes_alone_and_against_an_identical_reference(tmp_path):
    a, b = fake(tmp_path, "a"), fake(tmp_path, "b")
    assert cli_smoke.main([a]) == 0
    assert cli_smoke.main([a, "--reference", b]) == 0


def test_a_different_output_fails(tmp_path):
    a, b = fake(tmp_path, "a"), fake(tmp_path, "b", pattern="B")
    with pytest.raises(SystemExit) as e:
        cli_smoke.main([a, "--reference", b])
    assert e.value.code == 1


def test_a_binary_that_does_not_name_mzlab_fails(tmp_path):
    with pytest.raises(SystemExit) as e:
        cli_smoke.main([fake(tmp_path, "a", version="other 1.0")])
    assert e.value.code == 1


def test_a_binary_without_a_commit_fails(tmp_path):
    with pytest.raises(SystemExit) as e:
        cli_smoke.main([fake(tmp_path, "a", version="mzlab-cli 1.0.0 (commit unknown) mzLab")])
    assert e.value.code == 1
