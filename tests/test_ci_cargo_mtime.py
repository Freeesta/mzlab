"""tools/ci_cargo_mtime.py: a cache entry vouches only for sources whose crate cargo really compiled, so a run that failed before compiling
(or restored an older target/) cannot make the next restore treat changed sources as built."""
import os
import subprocess
import sys
import time
from pathlib import Path

TOOL = Path(__file__).resolve().parents[1] / "tools" / "ci_cargo_mtime.py"
OLD = 978307200


def run(repo: Path, mode: str) -> None:
    subprocess.run([sys.executable, str(TOOL), mode], cwd=repo, check=True, capture_output=True)


def git(repo: Path, *args: str) -> None:
    subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", *args], cwd=repo, check=True, capture_output=True)


def compile_crate(repo: Path, name: str) -> None:
    """What cargo leaves behind for a compiled crate: a fresh file in its .fingerprint folder."""
    fp = repo / "target/debug/.fingerprint" / f"{name}-0123abcd"
    fp.mkdir(parents=True, exist_ok=True)
    time.sleep(0.02)
    (fp / "lib-x.json").write_text("{}")


def mtime(repo: Path, rel: str) -> int:
    return int((repo / rel).stat().st_mtime)


def make_repo(tmp_path: Path) -> Path:
    for rel in ("crates/mzlab-lib/src/lib.rs", "crates/mzlab-core/src/lib.rs"):
        (tmp_path / rel).parent.mkdir(parents=True, exist_ok=True)
        (tmp_path / rel).write_text("// v1\n")
    git(tmp_path, "init", "-q")
    git(tmp_path, "add", ".")
    git(tmp_path, "commit", "-qm", "one")
    return tmp_path


def test_unchanged_sources_of_a_compiled_cache_keep_the_old_mtime(tmp_path):
    repo = make_repo(tmp_path)
    run(repo, "restore")
    compile_crate(repo, "mzlab-lib")
    compile_crate(repo, "mzlab-core")
    run(repo, "record")
    os.utime(repo / "crates/mzlab-lib/src/lib.rs", None)
    run(repo, "restore")
    assert mtime(repo, "crates/mzlab-lib/src/lib.rs") == OLD
    assert mtime(repo, "crates/mzlab-core/src/lib.rs") == OLD


def test_a_run_that_did_not_compile_does_not_vouch_for_changed_sources(tmp_path):
    repo = make_repo(tmp_path)
    run(repo, "restore")
    compile_crate(repo, "mzlab-lib")
    compile_crate(repo, "mzlab-core")
    run(repo, "record")
    # a later commit changes mzlab-lib; the next run restores the old cache, fails before cargo, and records anyway
    (repo / "crates/mzlab-lib/src/lib.rs").write_text("// v2\n")
    git(repo, "commit", "-qam", "two")
    run(repo, "restore")
    assert mtime(repo, "crates/mzlab-lib/src/lib.rs") != OLD
    run(repo, "record")
    # the run after that must still rebuild mzlab-lib, and keep reusing mzlab-core
    os.utime(repo / "crates/mzlab-lib/src/lib.rs", None)
    run(repo, "restore")
    assert mtime(repo, "crates/mzlab-lib/src/lib.rs") != OLD
    assert mtime(repo, "crates/mzlab-core/src/lib.rs") == OLD


def test_a_compiled_changed_crate_is_vouched_for(tmp_path):
    repo = make_repo(tmp_path)
    run(repo, "restore")
    compile_crate(repo, "mzlab-lib")
    compile_crate(repo, "mzlab-core")
    run(repo, "record")
    (repo / "crates/mzlab-lib/src/lib.rs").write_text("// v2\n")
    git(repo, "commit", "-qam", "two")
    run(repo, "restore")
    compile_crate(repo, "mzlab-lib")
    run(repo, "record")
    os.utime(repo / "crates/mzlab-lib/src/lib.rs", None)
    run(repo, "restore")
    assert mtime(repo, "crates/mzlab-lib/src/lib.rs") == OLD


def test_without_a_record_restore_changes_nothing(tmp_path):
    repo = make_repo(tmp_path)
    before = mtime(repo, "crates/mzlab-lib/src/lib.rs")
    run(repo, "restore")
    assert mtime(repo, "crates/mzlab-lib/src/lib.rs") == before
