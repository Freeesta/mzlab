"""tools/verifica.py sh(): a script that hangs fails after its cap, even when a grandchild (a browser) keeps the output open."""
import sys, time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import verifica

# the child starts a grandchild that inherits stdout/stderr and sleeps, then the child itself sleeps: killing the child alone leaves the output held open
SPAWNER = (
    "import subprocess, sys, time\n"
    "subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(120)'])\n"
    "print('child started', flush=True)\n"
    "time.sleep(120)\n"
)


def test_a_hung_script_with_an_orphan_grandchild_fails_at_its_cap(tmp_path):
    t = time.time()
    rc, out, _ = verifica.sh([sys.executable, "-c", SPAWNER], tmp_path / "x.log", 3)
    assert time.time() - t < 20
    assert rc == 124 and "TIMEOUT after 3 s" in out and "child started" in out
    assert verifica.judge(out, rc)
    assert "TIMEOUT after 3 s" in (tmp_path / "x.log").read_text(encoding="utf-8")


def test_the_grandchild_is_killed_with_the_script(tmp_path):
    pidfile = tmp_path / "pid"
    code = (
        "import subprocess, sys, time\n"
        f"g = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(120)'])\n"
        f"open({str(pidfile)!r}, 'w').write(str(g.pid))\n"
        "time.sleep(120)\n"
    )
    verifica.sh([sys.executable, "-c", code], tmp_path / "x.log", 3)
    pid = int(pidfile.read_text())
    import os
    if os.name == "nt":
        return                      # no portable liveness probe without psutil; the group kill is the taskkill /T above
    time.sleep(0.5)
    try:
        os.kill(pid, 0)
        # a zombie of a killed process still answers kill 0 until its parent reaps it: read its state
        state = Path(f"/proc/{pid}/stat").read_text().split(")")[-1].split()[0] if Path(f"/proc/{pid}/stat").exists() else "Z"
        assert state == "Z"
    except ProcessLookupError:
        pass


def test_a_normal_script_keeps_stdout_and_stderr_and_its_exit_code(tmp_path):
    rc, out, _ = verifica.sh([sys.executable, "-c", "import sys; print('a'); print('b', file=sys.stderr); sys.exit(3)"], tmp_path / "x.log", 30)
    assert rc == 3 and "a" in out and "b" in out


def test_a_long_silent_step_prints_a_heartbeat_in_progress_mode(tmp_path, monkeypatch, capsys):
    monkeypatch.setattr(verifica, "PROGRESS", True)
    monkeypatch.setattr(verifica, "HEARTBEAT", 1)
    rc, _out, _ = verifica.sh([sys.executable, "-c", "import time; time.sleep(3.5)"], tmp_path / "prepara_sito.log", 30)
    err = capsys.readouterr().err
    assert rc == 0 and "start prepara_sito" in err and "still running prepara_sito" in err
