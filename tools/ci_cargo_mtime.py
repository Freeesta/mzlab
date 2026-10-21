"""Makes cargo reuse the `target` folder restored by the CI cache. Cargo decides that a workspace crate is fresh by the mtime of its sources, and a
checkout gives every file the time of the checkout, so every crate would be compiled again. `restore` gives an old mtime to the tracked files whose
content equals the one recorded in `target/.mzlab-tree` when the cache was saved (changed or new files keep the checkout time and are rebuilt);
`record` writes that file from the current tree, just before the cache is saved. Without a record `restore` does nothing; it writes `record=true|false`
to $GITHUB_OUTPUT so test.yml can let a pull request seed a cache that has none."""
import os
import subprocess
import sys
from pathlib import Path

RECORD = Path("target/.mzlab-tree")
# 2001-01-01: older than anything a cached build wrote
OLD = 978307200


def tree() -> dict[str, str]:
    """Tracked path -> blob id of the working tree (the index is as checked out)."""
    out = subprocess.run(["git", "ls-files", "-s", "-z"], check=True, capture_output=True, encoding="utf-8").stdout
    return {e.split("\t", 1)[1]: e.split()[1] for e in out.split("\0") if e}


def restore() -> int:
    found = RECORD.is_file()
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as f:
            f.write(f"record={str(found).lower()}\n")
    if not found:
        print("no record of the cached tree: every crate is rebuilt")
        return 0
    prev = dict(line.split("\t", 1)[::-1] for line in RECORD.read_text(encoding="utf-8").splitlines())
    kept = 0
    for path, blob in tree().items():
        if prev.get(path) == blob and os.path.isfile(path):
            os.utime(path, (OLD, OLD))
            kept += 1
    print(f"{kept} files unchanged since the cached build")
    return 0


def record() -> int:
    RECORD.parent.mkdir(parents=True, exist_ok=True)
    RECORD.write_text("".join(f"{blob}\t{path}\n" for path, blob in tree().items()), encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit({"restore": restore, "record": record}[sys.argv[1]]())
