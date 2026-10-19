#!/usr/bin/env python3
"""Release notes of the GitHub release: a download table (Windows / Mac / Linux) on top, then the CHANGELOG.md section.
Also the single place that names the release files, so release.yml and the table cannot drift apart. Used by release.yml (job `bozza`).

    python3 tools/note_release.py --tag v1.0.1 --repo Freeesta/mzlab --notes NOTE.md > RELEASE_NOTES.md
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path


def nomi(version: str) -> dict[str, str]:
    """Names of the files attached to the release of `version` (X.Y.Z, no leading v)."""
    p = f"mzLab-{version}-"
    return {
        "windows": p + "Windows.exe",
        "mac": p + "Mac.dmg",
        "appimage": p + "Linux.AppImage",
        "deb": p + "Linux.deb",
        "sito": p + "sito.zip",
        "sbom": p + "sicurezza-SBOM.zip",
    }


def testo(tag: str, repo: str, changelog: str) -> str:
    n = nomi(tag.removeprefix("v"))
    url = lambda k: f"https://github.com/{repo}/releases/download/{tag}/{n[k]}"  # noqa: E731
    return (
        "| Windows | Mac | Linux |\n"
        "|---|---|---|\n"
        f"| [Installer (.exe)]({url('windows')}) | [Disk image (.dmg)]({url('mac')}) "
        f"| [AppImage]({url('appimage')}) · [.deb]({url('deb')}) |\n\n"
        f"`{n['sito']}` is a copy of the website to host yourself or open offline; `{n['sbom']}` holds the software bills of materials "
        "and `SHA256SUMS` the checksums of every file.\n\n" + changelog.strip() + "\n"
    )


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--tag", required=True, help="vX.Y.Z")
    ap.add_argument("--repo", required=True, help="owner/name")
    ap.add_argument("--notes", required=True, type=Path, help="the CHANGELOG section (tools/versione.py --nota)")
    a = ap.parse_args(argv)
    sys.stdout.write(testo(a.tag, a.repo, a.notes.read_text(encoding="utf-8")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
