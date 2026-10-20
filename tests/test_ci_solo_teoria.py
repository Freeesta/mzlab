"""tools/ci_solo_teoria.py: only a pull request whose files are ALL Teoria chapters gets the reduced checks of test.yml; any other file, even one, or no file, does not."""
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import ci_solo_teoria as ci  # noqa: E402

TEORIA = ["mzlab/web/teoria/src/12-dati.md", "mzlab/web/teoria/12-dati.html", "mzlab/web/teoria/indice-ricerca.js"]
OTHER = ["mzlab/web/teoria/teoria.js", "mzlab/web/teoria/teoria.css", "mzlab/web/teoria/tema.js", "mzlab/web/teoria/pratica/ei-dati.js", "mzlab/web/teoria/glossario-dati.js",
         "mzlab/web/teoria/src/LEGGIMI.md", "mzlab/web/teoria/src/12-dati.html", "mzlab/web/teoria/12-dati.md", "mzlab/web/teoria/sub/12-dati.html", "mzlab/web/teoria/12-Dati.html",
         "mzlab/web/index.html", "tools/libro/pagina.py", "tools/ci_solo_teoria.py", ".github/workflows/test.yml", "tests/test_libro.py", "crates/mzlab-libro/src/lib.rs",
         "docs/agenti/teoria.md", "mzlab/web/teoria/src/12-dati.md/../../teoria.js", "x/mzlab/web/teoria/12-dati.html", "mzlab/web/teoria/12-dati.html.bak"]


def test_only_teoria_files_is_solo():
    assert ci.solo(TEORIA) and ci.solo(TEORIA[:1])


def test_no_file_is_not_solo():
    assert not ci.solo([])


@pytest.mark.parametrize("other", OTHER)
def test_one_file_outside_the_list_is_not_solo(other):
    assert not ci.solo(TEORIA + [other])
    assert not ci.solo([other])


def test_the_selection_names_existing_tests():
    for t in ci.PYTEST:
        assert (ROOT / "tests" / f"{t}.py").exists(), t
    for t in ci.E2E:
        assert (ROOT / "tests_e2e" / f"{t}.py").exists(), t


def git(repo, *a):
    return subprocess.run(["git", "-C", str(repo), "-c", "user.email=t@t", "-c", "user.name=t", *a], check=True, capture_output=True, encoding="utf-8").stdout


def test_changed_sees_renames_and_deletions(tmp_path, monkeypatch):
    git(tmp_path, "init", "-q")
    (tmp_path / "a.txt").write_text("x" * 200)
    git(tmp_path, "add", "-A")
    git(tmp_path, "commit", "-qm", "base")
    base = git(tmp_path, "rev-parse", "HEAD").strip()
    (tmp_path / "mzlab/web/teoria").mkdir(parents=True)
    git(tmp_path, "mv", "a.txt", "mzlab/web/teoria/12-dati.html")
    git(tmp_path, "commit", "-qm", "move")
    monkeypatch.chdir(tmp_path)
    files = ci.changed(base)
    assert sorted(files) == ["a.txt", "mzlab/web/teoria/12-dati.html"]
    assert not ci.solo(files)


def test_main_writes_the_output_and_never_reduces_without_a_base(tmp_path, monkeypatch, capsys):
    out = tmp_path / "out"
    monkeypatch.setenv("GITHUB_OUTPUT", str(out))
    assert ci.main([]) == 0
    assert ci.main(["not-a-sha"]) == 0
    assert out.read_text() == "solo=false\nsolo=false\n"
