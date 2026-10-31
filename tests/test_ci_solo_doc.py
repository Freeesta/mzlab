"""tools/ci_solo_doc.py: only a pull request whose files are ALL documents gets the reduced checks of test.yml; any other file, even one, or no file, does not."""
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import ci_solo_doc as ci  # noqa: E402

DOCS = ["AGENTS.md", "CHANGELOG.md", "docs/agenti/ci-veloce.md", "docs/studi/ottimizza/x.md", ".claude/settings.json", ".claude/agents/cercatore.md"]
OTHER = ["tools/x.py", "tests/test_x.py", "docs/schemi/strumenti-1.json", "docs/agenti/mappa.png", "docs/agenti/ci-veloce.md.py", "docs/../tools/x.md", "x/AGENTS.md",
         "tools/lab/fixtures/a.md", "crates/mzlab-x/README.md", "mzlab/web/index.html", ".github/workflows/test.yml",
         "docs", ".claudeX/a.md", "Cargo.toml", "mzlab/web/lang/it.js", "doc/a.md"]


def test_only_documents_is_solo():
    assert ci.solo(DOCS) and ci.solo(["AGENTS.md"])


def test_no_file_is_not_solo():
    assert not ci.solo([])


@pytest.mark.parametrize("other", OTHER)
def test_one_file_outside_the_list_is_not_solo(other):
    assert not ci.solo(DOCS + [other])
    assert not ci.solo([other])


def test_the_selection_names_existing_tests():
    for t in ci.PYTEST:
        assert (ROOT / "tests" / f"{t}.py").exists(), t


def test_pytest_selection(capsys):
    assert ci.main(["--pytest"]) == 0
    assert "tests/test_versione.py" in capsys.readouterr().out


def git(repo, *a):
    return subprocess.run(["git", "-C", str(repo), "-c", "user.email=t@t", "-c", "user.name=t", *a], check=True, capture_output=True, encoding="utf-8").stdout


def _repo(tmp_path, monkeypatch, files):
    git(tmp_path, "init", "-q")
    (tmp_path / "a.txt").write_text("x")
    git(tmp_path, "add", "-A")
    git(tmp_path, "commit", "-qm", "base")
    base = git(tmp_path, "rev-parse", "HEAD").strip()
    for f in files:
        (tmp_path / f).parent.mkdir(parents=True, exist_ok=True)
        (tmp_path / f).write_text("y")
    git(tmp_path, "add", "-A")
    git(tmp_path, "commit", "-qm", "change")
    monkeypatch.chdir(tmp_path)
    return base


def test_main_writes_solo_and_kind_only_for_documents(tmp_path, monkeypatch):
    out = tmp_path / "out"
    monkeypatch.setenv("GITHUB_OUTPUT", str(out))
    repo = tmp_path / "r"
    repo.mkdir()
    base = _repo(repo, monkeypatch, ["AGENTS.md", "docs/agenti/a.md"])
    assert ci.main([base]) == 0
    assert out.read_text() == "solo=true\nkind=doc\n"


def test_a_python_file_with_documents_is_not_solo(tmp_path, monkeypatch):
    out = tmp_path / "out"
    monkeypatch.setenv("GITHUB_OUTPUT", str(out))
    repo = tmp_path / "r"
    repo.mkdir()
    base = _repo(repo, monkeypatch, ["AGENTS.md", "tools/x.py"])
    assert ci.main([base]) == 0
    assert not out.exists()


def test_main_never_reduces_without_a_base(tmp_path, monkeypatch):
    out = tmp_path / "out"
    monkeypatch.setenv("GITHUB_OUTPUT", str(out))
    monkeypatch.chdir(tmp_path)
    assert ci.main([]) == 0
    assert ci.main(["not-a-sha"]) == 0
    assert not out.exists()
