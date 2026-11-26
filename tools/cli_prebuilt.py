#!/usr/bin/env python3
"""Downloads the prebuilt mzlab-cli of a commit of main (Linux x86_64, static) and checks it, so nobody has to compile it.

    python3 tools/cli_prebuilt.py                       # the newest build of main
    python3 tools/cli_prebuilt.py --commit SHA          # the build of that commit (at least 12 hex digits)
    python3 tools/cli_prebuilt.py --dry-run --json      # only the URLs

The builds come from .github/workflows/cli-main.yml (a pre-release called `cli-prebuilt`, the last 10 commits of main, not a release). The file is
checked against its published SHA-256 and must report the commit it was named after (`--version --json`). stdout is the path of the binary (a `cli-prebuilt/1`
object under --json), stderr is progress and errors. Stdlib only. Exit: 0 ok, 2 usage, 3 not Linux x86_64, 4 no build for that commit (too old, or main moved
on: use the newest), 6 download failed, 7 checksum or commit differs (the file is deleted).
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import subprocess
import sys
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASE = "https://github.com/Freeesta/mzlab/releases/download/cli-prebuilt"
ARCH = "linux-x86_64"


class Fail(Exception):
    def __init__(self, code: int, msg: str):
        super().__init__(msg)
        self.code = code


def get(url: str, what: str) -> bytes:
    """The bytes of `url`; 404 is exit 4, any other failure exit 6."""
    try:
        with urllib.request.urlopen(url, timeout=120) as r:
            return r.read()
    except urllib.error.HTTPError as e:
        if e.code == 404:
            raise Fail(4, f"no {what} at {url}: the build is older than the last 10 commits of main, or its workflow has not finished; use the newest (no --commit) or build from source") from e
        raise Fail(6, f"{what}: HTTP {e.code} for {url}") from e
    except (urllib.error.URLError, OSError) as e:
        raise Fail(6, f"{what}: {e} ({url})") from e


def fetch(commit: str | None, out_dir: Path, base: str, dry: bool) -> dict:
    if commit is not None and not re.fullmatch(r"[0-9a-f]{12,40}", commit):
        raise Fail(2, f"--commit wants at least 12 hex digits of a commit, got {commit!r}")
    urls = {"latest": f"{base}/LATEST"}
    short = commit[:12] if commit else None
    name = lambda s: f"mzlab-cli-{s}-{ARCH}"  # noqa: E731
    if short:
        urls |= {"binary": f"{base}/{name(short)}", "sha256": f"{base}/{name(short)}.sha256"}
    if dry:
        return {"schema": "cli-prebuilt/1", "dry_run": True, "commit": commit, "urls": urls if short else {"latest": urls["latest"]}}
    if platform.system() != "Linux" or platform.machine() not in ("x86_64", "AMD64"):
        raise Fail(3, f"the prebuilt binary is {ARCH}; this machine is {platform.system()} {platform.machine()}: build from source (cargo build --profile cli -p mzlab-cli)")
    if short is None:
        commit = get(urls["latest"], "LATEST").decode().strip()
        if not re.fullmatch(r"[0-9a-f]{40}", commit):
            raise Fail(6, f"LATEST is not a commit: {commit[:60]!r}")
        short = commit[:12]
        print(f"cli_prebuilt: newest build is {short}", file=sys.stderr)
        urls |= {"binary": f"{base}/{name(short)}", "sha256": f"{base}/{name(short)}.sha256"}
    want = get(urls["sha256"], "SHA-256 file").decode().split()
    if len(want) != 2 or want[1] != name(short) or not re.fullmatch(r"[0-9a-f]{64}", want[0]):
        raise Fail(7, f"the SHA-256 file of {name(short)} is malformed: {' '.join(want)[:80]!r}")
    dest = out_dir / name(short)
    data = dest.read_bytes() if dest.is_file() else b""
    if hashlib.sha256(data).hexdigest() != want[0]:
        print(f"cli_prebuilt: downloading {urls['binary']}", file=sys.stderr)
        data = get(urls["binary"], "binary")
        got = hashlib.sha256(data).hexdigest()
        if got != want[0]:
            raise Fail(7, f"SHA-256 of the download is {got}, the published one is {want[0]}; nothing was written")
        out_dir.mkdir(parents=True, exist_ok=True)
        tmp = dest.with_name(dest.name + ".part")
        tmp.write_bytes(data)
        tmp.chmod(0o755)
        os.replace(tmp, dest)
    else:
        print(f"cli_prebuilt: {dest} already present and verified", file=sys.stderr)
    seen = subprocess.run([str(dest), "--version", "--json"], capture_output=True, text=True, timeout=60, check=False)
    try:
        built = json.loads(seen.stdout)["commit"]
    except (ValueError, KeyError, TypeError):
        built = None
    if not built or not built.startswith(short):
        dest.unlink(missing_ok=True)
        raise Fail(7, f"the binary reports commit {built!r}, not {short}; deleted")
    return {"schema": "cli-prebuilt/1", "dry_run": False, "commit": built, "path": str(dest), "sha256": want[0], "urls": urls}


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0], epilog="Exit: 0 ok, 2 usage, 3 not Linux x86_64, 4 no build for that commit, 6 download failed, 7 checksum or commit differs.")
    ap.add_argument("--commit", help="commit to fetch (at least 12 hex digits); default: the newest build of main")
    ap.add_argument("--dir", type=Path, default=ROOT / "target" / "cli-prebuilt", help="folder for the binary (default: target/cli-prebuilt)")
    ap.add_argument("--dry-run", action="store_true", help="print the URLs, download nothing")
    ap.add_argument("--json", action="store_true", help="print a cli-prebuilt/1 object instead of the path")
    try:
        a = ap.parse_args(argv)
    except SystemExit as e:
        return 0 if e.code == 0 else 2
    try:
        r = fetch(a.commit.lower() if a.commit else None, a.dir, os.environ.get("MZLAB_BASE_URL", BASE).rstrip("/"), a.dry_run)
    except Fail as e:
        print(f"cli_prebuilt: error: {e}", file=sys.stderr)
        return e.code
    print(json.dumps(r, indent=2) if a.json or a.dry_run else r["path"])
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
