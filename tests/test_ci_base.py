"""tools/ci_base.py: a pull request diffs against the first parent of the merge checkout (current main), never against the stale base of the event."""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import ci_base as cb  # noqa: E402


def git(cwd, *a):
    return subprocess.run(["git", "-C", str(cwd), "-c", "user.name=t", "-c", "user.email=t@t", *a], check=True, capture_output=True, encoding="utf-8").stdout.strip()


def commit(cwd, name):
    (cwd / name).write_text(name)
    git(cwd, "add", name)
    git(cwd, "commit", "-qm", name)
    return git(cwd, "rev-parse", "HEAD")


def merge_repo(tmp_path):
    """main moves after the PR branched; HEAD is the merge of the PR into the newer main, like refs/pull/N/merge."""
    git(tmp_path, "init", "-q", "-b", "main")
    old = commit(tmp_path, "a")
    git(tmp_path, "checkout", "-qb", "pr")
    commit(tmp_path, "mine")
    git(tmp_path, "checkout", "-q", "main")
    new = commit(tmp_path, "theirs")
    git(tmp_path, "merge", "-q", "--no-ff", "-m", "m", "pr")
    return old, new


def test_pull_request_uses_first_parent(tmp_path, monkeypatch):
    old, new = merge_repo(tmp_path)
    monkeypatch.chdir(tmp_path)
    assert cb.base("pull_request", old) == new
    changed = git(tmp_path, "diff", "--name-only", cb.base("pull_request", old), "HEAD").split()
    assert changed == ["mine"]
    assert sorted(git(tmp_path, "diff", "--name-only", old, "HEAD").split()) == ["mine", "theirs"]


def test_other_events_keep_the_fallback(tmp_path, monkeypatch):
    old, _ = merge_repo(tmp_path)
    monkeypatch.chdir(tmp_path)
    assert cb.base("push", "abc") == "abc"
    assert cb.base("merge_group", old) == old
    assert cb.base("", "") == ""


def test_not_a_merge_keeps_the_fallback(tmp_path, monkeypatch):
    git(tmp_path, "init", "-q", "-b", "main")
    commit(tmp_path, "a")
    commit(tmp_path, "b")
    monkeypatch.chdir(tmp_path)
    assert cb.base("pull_request", "abc") == "abc"
