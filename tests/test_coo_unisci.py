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


def commit(repo, nome, testo, msg):
    p = Path(repo) / nome
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(testo)
    sh(repo, "add", "-A")
    sh(repo, "commit", "-qm", msg)
    return sh(repo, "rev-parse", "HEAD")


@pytest.fixture
def mondo(tmp_path):
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

    def eventi(sha, test="success", rust=None):
        sh(clone, "checkout", "-q", "--orphan", "stat-ci")
        sh(clone, "rm", "-rqf", ".")
        righe = [{"tipo": "run", "sha": sha, "workflow": "test", "esito": test, "ts_utc": "2026-10-21T10:00:00Z"}]
        if rust:
            righe.append({"tipo": "run", "sha": sha, "workflow": "rust", "esito": rust, "ts_utc": "2026-10-21T10:00:00Z"})
        commit(clone, "eventi.jsonl", "".join(json.dumps(r) + "\n" for r in righe), "ev")
        sh(clone, "push", "-qf", "origin", "HEAD:stat-ci")
        sh(clone, "checkout", "-qf", "main")
    return clone, pr, eventi


def test_verde(mondo):
    clone, pr, eventi = mondo
    n, sha = pr("b.txt", "x\n")
    eventi(sha)
    r = c.valuta(clone, n, sha)
    assert r["verde"] and r["combine"] and r["unire"] and r["rischi"] == []
    assert sh(clone, "worktree", "list").count("\n") == 0


def test_rosso(mondo):
    clone, pr, eventi = mondo
    n, sha = pr("b.txt", "x\n")
    eventi(sha, rust="failure")
    r = c.valuta(clone, n, sha)
    assert not r["verde"] and not r["unire"] and r["motivo"] == "CI non verde: rust"


def test_sha_diverso(mondo):
    clone, pr, eventi = mondo
    n, sha = pr("b.txt", "x\n")
    eventi(sha)
    r = c.valuta(clone, n, "0" * 40)
    assert not r["unire"] and "differs" in r["motivo"]


def test_conflitto(mondo):
    clone, pr, eventi = mondo
    n, sha = pr("a.txt", "pr\n")
    commit(clone, "a.txt", "main\n", "main cambia a")
    sh(clone, "push", "-q", "origin", "HEAD:main")
    eventi(sha)
    r = c.valuta(clone, n, sha)
    assert r["verde"] and r["combine"] is False and not r["unire"] and "a.txt" in r["motivo"]


def test_rischio_cargo(mondo):
    clone, pr, eventi = mondo
    n, sha = pr("crates/x/Cargo.toml", "[package]\n")
    eventi(sha)
    r = c.valuta(clone, n, sha)
    assert r["verde"] and r["combine"] and not r["unire"] and r["rischi"] == ["VIA richiesto: crates/x/Cargo.toml"]
