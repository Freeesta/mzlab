#!/usr/bin/env python3
"""Notes and file names of the rolling pre-release `app-prebuilt` (app-main.yml): the commit and the dates on top, then the steps of docs/download-app.md.
The one place that names the files, so the workflow and the notes cannot drift apart.

    python3 tools/note_app_main.py --repo Freeesta/mzlab --sha <40 hex> --commit-date 2026-11-26 --built 2026-11-26T10:00Z > NOTES.md
    python3 tools/note_app_main.py --names     # one "<artifact>|<glob>|<final name>" per line, for the workflow
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TAG = "app-prebuilt"
TITLE = "mzLab app built from main (NOT a release)"

# (artifact of desktop.yml, glob inside it, name attached to the pre-release); fixed names without a version, so a link never changes
FILES = [
    ("mzlab-desktop-macOS", "*.dmg", "mzLab-latest-Mac.dmg"),
    ("mzlab-desktop-Windows", "*.exe", "mzLab-latest-Windows.exe"),
    ("mzlab-desktop-Linux", "*.AppImage", "mzLab-latest-Linux.AppImage"),
    ("mzlab-desktop-Linux", "*.deb", "mzLab-latest-Linux.deb"),
]


def steps() -> str:
    """The sections of docs/download-app.md from the first `##` heading to the maintainers' one (the intro is the notes' own table)."""
    t = (ROOT / "docs" / "download-app.md").read_text(encoding="utf-8")
    return ("## " + t.split("\n## ", 1)[1]).split("## For maintainers")[0].strip()


def notes(repo: str, sha: str, commit_date: str, built: str) -> str:
    if not re.fullmatch(r"[0-9a-f]{40}", sha):
        raise ValueError(f"not a full commit: {sha!r}")
    base = f"https://github.com/{repo}/releases/download/{TAG}/"
    link = lambda n: f"[{n}]({base}{n})"  # noqa: E731
    return (
        "**Not a release of mzLab** (version 1.0 is not published). This is the app built from the newest `main`; every change replaces these files.\n\n"
        f"- Commit: [`{sha[:12]}`](https://github.com/{repo}/commit/{sha}) (`{sha}`)\n"
        f"- Commit date: {commit_date}\n"
        f"- Built: {built}\n"
        "- Unsigned and not notarised: see the steps below. The same commit and date are in the app's About box (the i button in the header).\n\n"
        "| Mac (Apple silicon and Intel) | Windows | Linux |\n|---|---|---|\n"
        f"| {link(FILES[0][2])} | {link(FILES[1][2])} | {link(FILES[2][2])} · {link(FILES[3][2])} |\n\n"
        f"Checksums: {link('SHA256SUMS')}. Command line for agents (Linux): the pre-release "
        f"[`cli-prebuilt`](https://github.com/{repo}/releases/tag/cli-prebuilt), built by its own workflow.\n\n"
        + steps() + "\n"
    )


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--names", action="store_true")
    p.add_argument("--repo")
    p.add_argument("--sha")
    p.add_argument("--commit-date", default="unknown")
    p.add_argument("--built", default="unknown")
    a = p.parse_args(argv)
    if a.names:
        print("\n".join("|".join(f) for f in FILES))
        return 0
    if not (a.repo and a.sha):
        p.error("--repo and --sha are required")
    try:
        sys.stdout.write(notes(a.repo, a.sha, a.commit_date, a.built))
    except ValueError as e:
        print(f"error: {e}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
