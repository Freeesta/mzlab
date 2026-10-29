"""Makes cargo reuse the `target` folder restored by the CI cache. Cargo decides that a workspace crate is fresh by the mtime of its sources, and a
checkout gives every file the time of the checkout, so every crate would be compiled again. `restore` gives an old mtime to the tracked files whose
content equals the one recorded in `target/.mzlab-tree` when the cache was saved (changed or new files keep the checkout time and are rebuilt);
`record` writes that file just before the cache is saved, but only for the files whose crate cargo really rebuilt in this run (a fingerprint newer than the
stamp `restore` leaves): a run that failed before compiling, or restored an older cache, must not vouch for sources its `target` never saw. Without a record
`restore` does nothing; it writes `record=true|false` to $GITHUB_OUTPUT so test.yml can let a pull request seed a cache that has none.
The record also names the job that wrote it: a `target` holds only the build variants (profile, target, features) its own job compiles, so a record
of another job (a restore-key fallback) vouches for sources whose variant here may be stale, and `restore` ignores it."""
import os
import subprocess
import sys
from pathlib import Path

RECORD = Path("target/.mzlab-tree")
# touched by `restore`; a crate whose .fingerprint is newer than this was compiled by the run
STAMP = Path("target/.mzlab-stamp")
# the CI job that wrote the record (empty outside CI)
JOB = Path("target/.mzlab-job")
# 2001-01-01: older than anything a cached build wrote
OLD = 978307200


def tree() -> dict[str, str]:
    """Tracked path -> blob id of the working tree (the index is as checked out)."""
    out = subprocess.run(["git", "ls-files", "-s", "-z"], check=True, capture_output=True, encoding="utf-8").stdout
    return {e.split("\t", 1)[1]: e.split()[1] for e in out.split("\0") if e}


def read_record() -> dict[str, str]:
    if not RECORD.is_file():
        return {}
    return dict(line.split("\t", 1)[::-1] for line in RECORD.read_text(encoding="utf-8").splitlines())


def same_job() -> bool:
    written = JOB.read_text(encoding="utf-8") if JOB.is_file() else ""
    return written == os.environ.get("GITHUB_JOB", "")


def crate_of(path: str) -> str | None:
    """Directory name of the workspace crate owning a path (None for files outside crates/)."""
    parts = path.split("/")
    return parts[1] if parts[0] == "crates" and len(parts) > 2 else None


def built_crates(since: float) -> tuple[set[str], bool]:
    """Names of the crates with a fingerprint newer than `since`, and whether any fingerprint at all is (vendored crates, lock file)."""
    names: set[str] = set()
    any_new = False
    for fp in Path("target").glob("**/.fingerprint/*"):
        try:
            if fp.stat().st_mtime > since or any(f.stat().st_mtime > since for f in fp.iterdir()):
                any_new = True
                names.add(fp.name.rsplit("-", 1)[0])
        except OSError:
            continue
    return names, any_new


def restore() -> int:
    found = RECORD.is_file()
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as f:
            f.write(f"record={str(found).lower()}\n")
    STAMP.parent.mkdir(parents=True, exist_ok=True)
    STAMP.write_text("", encoding="utf-8")
    if not found:
        print("no record of the cached tree: every crate is rebuilt")
        return 0
    if not same_job():
        print("the cached tree was recorded by another job: every crate is rebuilt")
        return 0
    prev = read_record()
    kept = 0
    for path, blob in tree().items():
        if prev.get(path) == blob and os.path.isfile(path):
            os.utime(path, (OLD, OLD))
            kept += 1
    print(f"{kept} files unchanged since the cached build")
    return 0


def record() -> int:
    """Writes the files already vouched for plus those of the crates compiled since `restore`; the rest is left out and so rebuilt next time."""
    prev = read_record() if same_job() else {}
    names, any_new = built_crates(STAMP.stat().st_mtime) if STAMP.is_file() else (set(), False)
    out = {}
    for path, blob in tree().items():
        crate = crate_of(path)
        compiled = any_new if crate is None else crate in names
        if prev.get(path) == blob or compiled:
            out[path] = blob
    RECORD.parent.mkdir(parents=True, exist_ok=True)
    RECORD.write_text("".join(f"{blob}\t{path}\n" for path, blob in out.items()), encoding="utf-8")
    JOB.write_text(os.environ.get("GITHUB_JOB", ""), encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit({"restore": restore, "record": record}[sys.argv[1]]())
