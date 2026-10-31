"""Cargo helpers shared by the tests that drive the Rust examples: `run` reports the stderr of a failed command, `build_example` builds an example once per machine even when several xdist workers ask at the same time."""
import fcntl
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


class CommandError(subprocess.CalledProcessError):
    """A CalledProcessError whose message carries the tail of stdout and stderr (the default one hides both)."""

    def __str__(self):
        tail = lambda s: (s or "")[-4000:]
        return f"{super().__str__()}\n--- stdout ---\n{tail(self.stdout)}\n--- stderr ---\n{tail(self.stderr)}"


def run(*a):
    p = subprocess.run(a, cwd=ROOT, capture_output=True, encoding="utf-8")
    if p.returncode:
        raise CommandError(p.returncode, a, p.stdout, p.stderr)
    return p


def build_example(package, example):
    """`cargo build --release --locked -p PACKAGE --example EXAMPLE`, serialised across processes by a lock file so that a cold cache is filled by one worker while the others wait."""
    with open(Path(tempfile.gettempdir()) / f"mzlab-cargo-{package}-{example}.lock", "w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        run("cargo", "build", "--release", "--locked", "-q", "-p", package, "--example", example)
