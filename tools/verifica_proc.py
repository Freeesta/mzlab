"""Child-process side of tools/verifica.py: run one command with its output straight into a log file, in a process group of its own,
and kill the whole group on a timeout (a browser outlives a script killed alone). No pipe exists for an orphan to hold open."""
from __future__ import annotations

import os
import signal
import subprocess
import sys
import time
from pathlib import Path


def kill_tree(p: subprocess.Popen) -> None:
    """Kill the child and everything it started, then reap it with a bounded wait."""
    try:
        if os.name == "nt":
            subprocess.run(["taskkill", "/F", "/T", "/PID", str(p.pid)], capture_output=True, timeout=30)
        else:
            os.killpg(p.pid, signal.SIGKILL)
    except (ProcessLookupError, PermissionError, subprocess.TimeoutExpired):
        pass
    try:
        p.kill()
        p.wait(timeout=10)
    except (ProcessLookupError, subprocess.TimeoutExpired):
        pass


def run_logged(cmd, log: Path, timeout: float, env, cwd, beat: float, t_zero: float | None) -> tuple[int, bool]:
    """(exit code, timed out). With `t_zero` (start of the whole run) a long step prints a line to stderr every `beat` seconds, so a silent build names itself."""
    log.parent.mkdir(parents=True, exist_ok=True)
    own = {"creationflags": subprocess.CREATE_NEW_PROCESS_GROUP} if os.name == "nt" else {"start_new_session": True}
    t0 = time.time()
    if t_zero is not None:      # the name is out BEFORE the command runs: a hang names its script
        print(f"[{t0 - t_zero:5.0f} s] start {log.stem} (cap {timeout} s)", file=sys.stderr, flush=True)
    with open(log, "wb") as fh:
        p = subprocess.Popen(cmd, cwd=cwd, env=env, stdin=subprocess.DEVNULL, stdout=fh, stderr=subprocess.STDOUT, **own)
        while True:
            left = timeout - (time.time() - t0)
            try:
                return p.wait(timeout=max(0.01, min(beat, left))), False
            except subprocess.TimeoutExpired:
                if left <= beat:
                    kill_tree(p)
                    return 124, True
                if t_zero is not None:
                    print(f"[{time.time() - t_zero:5.0f} s] still running {log.stem} ({time.time() - t0:.0f} s of {timeout} s)", file=sys.stderr, flush=True)
