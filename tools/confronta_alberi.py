#!/usr/bin/env python3
"""Compare two built sites (a folder, a .tar or a .zip each): path and SHA-256 of every file.

Used to prove that a change of pages.yml leaves the published site byte for byte the same. The commit named in
static/build.json of each tree is replaced by a placeholder in every file that carries it (build.json, version.js),
so two builds of different commits compare equal when only the commit differs. Exit 0 = identical, 1 = differences.
Standard library only: python3 tools/confronta_alberi.py A B [--lista]"""
import argparse
import hashlib
import json
import sys
import tarfile
import zipfile
from pathlib import Path

PLACEHOLDER = b"<COMMIT>"


def read_tree(path: Path) -> dict[str, bytes]:
    """The files of a folder, tar or zip as {path with / separators: bytes}; a leading ./ is dropped."""
    files: dict[str, bytes] = {}
    if path.is_dir():
        for f in sorted(path.rglob("*")):
            if f.is_file():
                files[f.relative_to(path).as_posix()] = f.read_bytes()
    elif zipfile.is_zipfile(path):
        with zipfile.ZipFile(path) as z:
            for n in z.namelist():
                if not n.endswith("/"):
                    files[n] = z.read(n)
    else:
        with tarfile.open(path) as t:
            for m in t:
                if m.isfile():
                    files[m.name.removeprefix("./")] = t.extractfile(m).read()
    return files


def commit_of(files: dict[str, bytes]) -> bytes | None:
    raw = files.get("static/build.json")
    return json.loads(raw)["commit"].encode() if raw else None


def digests(files: dict[str, bytes]) -> dict[str, str]:
    """SHA-256 per path, with the commit of the tree replaced by a placeholder first."""
    commit = commit_of(files)
    out = {}
    for name, data in files.items():
        if commit and commit != b"dev" and commit in data:
            data = data.replace(commit, PLACEHOLDER)
        out[name] = hashlib.sha256(data).hexdigest()
    return out


def compare(a: dict[str, str], b: dict[str, str]) -> tuple[list[str], list[str], list[str]]:
    return sorted(a.keys() - b.keys()), sorted(b.keys() - a.keys()), sorted(k for k in a.keys() & b.keys() if a[k] != b[k])


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("a")
    ap.add_argument("b")
    ap.add_argument("--lista", action="store_true", help="print path and SHA-256 of every file of the first tree")
    args = ap.parse_args()
    a, b = digests(read_tree(Path(args.a))), digests(read_tree(Path(args.b)))
    if args.lista:
        for k in sorted(a):
            print(f"{a[k]}  {k}")
    only_a, only_b, differ = compare(a, b)
    print(f"files: {len(a)} / {len(b)}; only in A: {len(only_a)}; only in B: {len(only_b)}; different: {len(differ)}")
    for label, names in (("only in A", only_a), ("only in B", only_b), ("different", differ)):
        for n in names[:50]:
            print(f"  {label}: {n}")
    print("IDENTICAL" if not (only_a or only_b or differ) else "DIFFERENT")
    return 0 if not (only_a or only_b or differ) else 1


if __name__ == "__main__":
    sys.exit(main())
