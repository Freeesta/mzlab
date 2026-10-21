"""tools/fondi_main.py in a temporary repository: a conflict on a generated JSON resolves itself, one on a .rs file stops."""
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import fondi_main as f  # noqa: E402


def sh(cwd, *a):
    return subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", *a], cwd=cwd, capture_output=True, text=True, check=True).stdout.strip()


def scrivi(repo, nome, testo, msg):
    (Path(repo) / nome).write_text(testo)
    sh(repo, "add", "-A")
    sh(repo, "commit", "-qm", msg)


def prepara(tmp_path, file):
    origin, clone = tmp_path / "o.git", tmp_path / "c"
    sh(tmp_path, "init", "-q", "--bare", "-b", "main", str(origin))
    sh(tmp_path, "clone", "-q", str(origin), str(clone))
    scrivi(clone, file, "base\n", "base")
    sh(clone, "push", "-q", "origin", "HEAD:main")
    sh(clone, "checkout", "-q", "-b", "claude/x")
    scrivi(clone, file, "ramo\n", "ramo")
    sh(clone, "checkout", "-q", "main")
    scrivi(clone, file, "main\n", "main")
    sh(clone, "push", "-q", "origin", "HEAD:main")
    sh(clone, "checkout", "-q", "claude/x")
    return clone


def test_generato_si_risolve(tmp_path):
    clone = prepara(tmp_path, "gen.json")
    gen = {"gen.json": [sys.executable, "-c", "open('gen.json','w').write('rigenerato\\n')"]}
    righe = []
    assert f.fondi(clone, gen, righe.append) == 0
    assert (clone / "gen.json").read_text() == "rigenerato\n"
    assert sh(clone, "status", "--porcelain") == "" and len(sh(clone, "log", "--merges", "--format=%h").split()) == 1


def test_umano_si_ferma(tmp_path):
    clone = prepara(tmp_path, "motore.rs")
    righe = []
    assert f.fondi(clone, {"gen.json": ["true"]}, righe.append) == 1
    assert "  motore.rs" in righe
    assert "motore.rs" in sh(clone, "diff", "--name-only", "--diff-filter=U")
