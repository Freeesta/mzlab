"""tools/ci_toolchain.py: the pinned toolchain is read from rust-toolchain.toml, a broken install is removed and retried, and the e2e jobs of the
workflows install it through the tool (not a bare `rustup show`, which exits 0 on a rolled-back install and lets parallel tests race)."""
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import ci_toolchain as ct  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
WF = ROOT / ".github" / "workflows"


def test_pinned_reads_the_file():
    p = ct.pinned()
    assert re.fullmatch(r"\d+\.\d+\.\d+", p["channel"])
    assert "wasm32-unknown-unknown" in p["targets"] and "clippy" in p["components"]


def test_ensure_retries_and_removes_the_broken_install(monkeypatch):
    calls = []
    answers = iter(["target x is missing", "target x is missing", ""])
    monkeypatch.setattr(ct.shutil, "which", lambda n: "/bin/" + n)
    monkeypatch.setattr(ct, "run", lambda cmd, root: (calls.append(cmd[:3]), (0, ""))[1])
    monkeypatch.setattr(ct, "problem", lambda root=ct.ROOT: next(answers))
    monkeypatch.setattr(ct.time, "sleep", lambda s: None)
    assert ct.ensure() == ""
    assert [c for c in calls if c[1] == "toolchain"] == [["rustup", "toolchain", "uninstall"]] * 2


def test_ensure_gives_the_reason_after_the_last_try(monkeypatch):
    monkeypatch.setattr(ct.shutil, "which", lambda n: "/bin/" + n)
    monkeypatch.setattr(ct, "run", lambda cmd, root: (0, ""))
    monkeypatch.setattr(ct, "problem", lambda root=ct.ROOT: "cargo does not run")
    monkeypatch.setattr(ct.time, "sleep", lambda s: None)
    assert ct.ensure() == "cargo does not run"


def test_e2e_jobs_install_the_toolchain_through_the_tool():
    # the jobs that run tools/verifica.py with e2e (and so tests that call cargo side by side)
    for name in ("test.yml", "notte.yml", "sito-rust.yml"):
        text = (WF / name).read_text(encoding="utf-8")
        assert "python tools/ci_toolchain.py" in text, name
    sito = (WF / "sito-rust.yml").read_text(encoding="utf-8")
    e2e_job = sito[sito.index("  sito-e2e:"):]
    assert e2e_job.index("tools/ci_toolchain.py") < e2e_job.index("python tools/verifica.py")
