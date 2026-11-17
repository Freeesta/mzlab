"""Cargo helpers shared by the tests that drive the Rust binaries and .wasm: `run` reports the stderr of a failed command, `build` / `cli` / `example` / `build_example` build once per machine
(several xdist workers may ask at the same time) or, when MZLAB_PREBUILT names the folder of the `prebuilt` jobs of test.yml (tools/ci_prebuilt.py), install the file that CI already built;
without the variable, or for a build that folder does not list, cargo runs exactly as before."""
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
from ci_prebuilt import canon, directory, manifest, outputs  # noqa: E402
from flock_util import lock_exclusive  # noqa: E402

_manifest: dict[str, str] | None = None
_native: set[str] = set()   # native files installed by this process (a .wasm is installed again: other builds of the same crate write the same path)


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


def installed(*cargo_args):
    """True when the file of every package of `cargo build ARGS` was installed from MZLAB_PREBUILT; a build that is not listed there is recorded in MZLAB_PREBUILT_MISSES."""
    global _manifest
    d, keys = directory(), canon(["build", *cargo_args])
    if d is None or not keys:
        return False
    if _manifest is None:
        _manifest = manifest(d)
    missing = [k for k in keys if k not in _manifest]
    if missing:
        if os.environ.get("MZLAB_PREBUILT_MISSES"):
            with open(os.environ["MZLAB_PREBUILT_MISSES"], "a", encoding="utf-8") as f:
                f.writelines(f"{k}\n" for k in missing)
        return False
    for k in keys:
        dst = ROOT / outputs(k)[0]
        if k in _native and dst.exists():
            continue
        dst.parent.mkdir(parents=True, exist_ok=True)
        tmp = dst.with_name(f"{dst.name}.{os.getpid()}.tmp")   # replaced atomically: a worker that runs the old file keeps it
        shutil.copyfile(d / _manifest[k], tmp)
        tmp.chmod(0o755)
        os.replace(tmp, dst)
        if not dst.name.endswith(".wasm"):
            _native.add(k)
    return True


def build(*cargo_args, runner=run):
    """`cargo build --release --locked -q ARGS` (ARGS as in the command line), or the prebuilt file. `runner` is the `run` of a test file that has its own."""
    if installed(*cargo_args):
        return None
    return runner("cargo", "build", "--release", "--locked", "-q", *cargo_args)


def cli_argv(*args):
    """The command line of `mzlab-cli ARGS` (release): the installed binary, else `cargo run`."""
    if installed("-p", "mzlab-cli"):
        return [str(ROOT / "target/release/mzlab-cli"), *args]
    return ["cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-cli", "--", *args]


def example_argv(package, name, *args, features=None):
    """The command line of `cargo run --release -p PACKAGE [--features F] --example NAME -- ARGS`: the installed example, else `cargo run`."""
    sel = ["-p", package, *(["--features", features] if features else []), "--example", name]
    if installed(*sel):
        return [str(ROOT / "target/release/examples" / name), *args]
    return ["cargo", "run", "--release", "--locked", "-q", *sel, "--", *args]


def cli(*args, runner=run):
    return runner(*cli_argv(*args))


def example(package, name, *args, features=None, runner=run):
    return runner(*example_argv(package, name, *args, features=features))


def build_example(package, example):
    """`cargo build --release --locked -p PACKAGE --example EXAMPLE`, serialised across processes by a lock file so that a cold cache is filled by one worker while the others wait."""
    with open(Path(tempfile.gettempdir()) / f"mzlab-cargo-{package}-{example}.lock", "w") as lock:
        lock_exclusive(lock)
        if not installed("-p", package, "--example", example):
            run("cargo", "build", "--release", "--locked", "-q", "-p", package, "--example", example)
