"""tools/coo_unisci.py with a temporary origin: green, red CI, different SHA, conflict, Cargo risk."""
import json
import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import coo_unisci as c  # noqa: E402


def sh(cwd, *a):
    return subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", *a], cwd=cwd, capture_output=True, text=True, check=True).stdout.strip()


def commit(repo, name, testo, msg):
    p = Path(repo) / name
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(testo)
    sh(repo, "add", "-A")
    sh(repo, "commit", "-qm", msg)
    return sh(repo, "rev-parse", "HEAD")


@pytest.fixture
def world(tmp_path):
    """(clone, origin, pr(file, testo) -> sha, eventi(sha, test, rust) -> None)."""
    origin, clone = tmp_path / "origin.git", tmp_path / "clone"
    sh(tmp_path, "init", "-q", "--bare", "-b", "main", str(origin))
    sh(tmp_path, "clone", "-q", str(origin), str(clone))
    commit(clone, "a.txt", "1\n", "base")
    sh(clone, "push", "-q", "origin", "HEAD:main")
    # The checks run in the worktree: stub them (the real ones are tested elsewhere).
    (clone / "tools").mkdir()
    for n in ("controlla.py", "indice.py"):
        (clone / "tools" / n).write_text("import sys; sys.exit(0)\n")
    sh(clone, "add", "-A")
    sh(clone, "commit", "-qm", "stubs")
    sh(clone, "push", "-q", "origin", "HEAD:main")
    base = sh(clone, "rev-parse", "HEAD")
    n = [0]

    def pr(file, testo):
        n[0] += 1
        sh(clone, "checkout", "-q", "-B", f"pr{n[0]}", base)
        sha = commit(clone, file, testo, f"pr {n[0]}")
        sh(clone, "push", "-q", "origin", f"HEAD:refs/pull/{n[0]}/head")
        sh(clone, "checkout", "-q", "main")
        return n[0], sha

    def events(sha, test="success", rust=None):
        sh(clone, "checkout", "-q", "--orphan", "stat-ci")
        sh(clone, "rm", "-rqf", ".")
        rows = [{"tipo": "run", "sha": sha, "workflow": "test", "esito": test, "ts_utc": "2026-10-21T10:00:00Z"}]
        if rust:
            rows.append({"tipo": "run", "sha": sha, "workflow": "rust", "esito": rust, "ts_utc": "2026-10-21T10:00:00Z"})
        commit(clone, "eventi.jsonl", "".join(json.dumps(r) + "\n" for r in rows), "ev")
        sh(clone, "push", "-qf", "origin", "HEAD:stat-ci")
        sh(clone, "checkout", "-qf", "main")
    return clone, pr, events


def test_green(world):
    clone, pr, events = world
    n, sha = pr("b.txt", "x\n")
    events(sha)
    r = c.evaluate(clone, n, sha)
    assert r["verde"] and r["combine"] and r["unire"] and r["rischi"] == []
    assert sh(clone, "worktree", "list").count("\n") == 0


def test_red(world):
    clone, pr, events = world
    n, sha = pr("b.txt", "x\n")
    events(sha, rust="failure")
    r = c.evaluate(clone, n, sha)
    assert not r["verde"] and not r["unire"] and r["motivo"] == "CI non verde: rust"


def test_different_sha(world):
    clone, pr, events = world
    n, sha = pr("b.txt", "x\n")
    events(sha)
    r = c.evaluate(clone, n, "0" * 40)
    assert not r["unire"] and "differs" in r["motivo"]


def test_conflict(world):
    clone, pr, events = world
    n, sha = pr("a.txt", "pr\n")
    commit(clone, "a.txt", "main\n", "main cambia a")
    sh(clone, "push", "-q", "origin", "HEAD:main")
    events(sha)
    r = c.evaluate(clone, n, sha)
    assert r["verde"] and r["combine"] is False and not r["unire"] and "a.txt" in r["motivo"]


def test_rischio_cargo(world):
    clone, pr, events = world
    n, sha = pr("crates/x/Cargo.toml", "[package]\n")
    events(sha)
    r = c.evaluate(clone, n, sha)
    assert r["verde"] and r["combine"] and not r["unire"] and r["rischi"] == ["VIA richiesto: crates/x/Cargo.toml"]
