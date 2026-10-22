"""P7: pre-push hook installer, refusal of a bad push, warning at 90% of the cap for hot files."""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import controlla  # noqa: E402
import installa_hook as ih  # noqa: E402


def git(cwd, *a):
    return subprocess.run(["git", *a], cwd=cwd, capture_output=True, text=True)


def repo(tmp_path):
    git(tmp_path, "init", "-q")
    git(tmp_path, "config", "user.email", "t@example.org")
    git(tmp_path, "config", "user.name", "t")
    return tmp_path


def test_installs_removes_and_respects_foreign_hook(tmp_path):
    r = repo(tmp_path)
    assert "installed" in ih.installa(r)
    h = r / ".git" / "hooks" / "pre-push"
    assert h.is_file() and h.stat().st_mode & 0o111 and "pre_pr.py --veloce" in h.read_text()
    assert "removed" in ih.togli(r) and not h.exists()
    h.write_text("#!/bin/sh\necho mine\n")
    assert "left alone" in ih.installa(r) and h.read_text().endswith("mine\n")
    assert "no hook" in ih.togli(r)


def test_hook_refuses_when_checks_fail_and_ignores_deletions(tmp_path):
    r = repo(tmp_path)
    (r / "tools").mkdir()
    (r / "tools" / "pre_pr.py").write_text("import sys; sys.exit(1)\n")
    hook = ROOT / "tools" / "hook_pre_push.sh"
    refs = "refs/heads/x abc123 refs/heads/x 0000\n"
    assert subprocess.run(["sh", str(hook)], cwd=r, input=refs, text=True, capture_output=True).returncode == 1
    dele = "(delete) 0000000000000000000000000000000000000000 refs/heads/x abc\n"
    assert subprocess.run(["sh", str(hook)], cwd=r, input=dele, text=True, capture_output=True).returncode == 0


def test_hot_files_warn_from_90_percent(tmp_path, monkeypatch):
    monkeypatch.setattr(controlla.indice, "ROOT", tmp_path)
    (tmp_path / "a.js").write_text("x\n" * 455)
    (tmp_path / "b.js").write_text("x\n" * 300)
    (tmp_path / "c.js").write_text("x\n" * 460)
    doc = tmp_path / "docs" / "agenti"
    doc.mkdir(parents=True)
    (doc / "d.md").write_text("y" * 24000)
    tocchi = {"a.js": 6, "b.js": 9, "c.js": 5, "docs/agenti/d.md": 7, "gone.js": 8}
    out = controlla.vicini_caldi(tocchi)
    assert len(out) == 2 and any(o.startswith("a.js has 456 lines") for o in out)
    assert any(o.startswith("docs/agenti/d.md is 23.4 KB") for o in out)


def test_caldi_counts_first_parent_commits(tmp_path, monkeypatch):
    r = repo(tmp_path)
    for i in range(3):
        (r / "f.txt").write_text(str(i))
        if i == 2:
            (r / "g.txt").write_text("g")
        git(r, "add", ".")
        git(r, "commit", "-qm", f"c{i}")
    monkeypatch.setattr(controlla.indice, "ROOT", r)
    assert controlla.caldi("HEAD") == {"f.txt": 3, "g.txt": 1}
    assert controlla.caldi("no-such-ref") == {}
