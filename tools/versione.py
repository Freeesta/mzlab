"""One version for the site and the desktop app: Cargo.toml, tauri.conf.json, CITATION.cff (and .zenodo.json if it has one) must agree.

    python3 tools/versione.py                  # all the files say the same version
    python3 tools/versione.py --tag v0.2.0     # ... and it is the one of the tag
    python3 tools/versione.py --nota v0.2.0    # prints the notes of that version from CHANGELOG.md (fails if missing or empty)
Stdlib only. Used by tests/test_versione.py (required pytest job) and by .github/workflows/release.yml."""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SEMVER = re.compile(r"^\d+\.\d+\.\d+$")


def versioni(root: Path = ROOT) -> dict[str, str]:
    out: dict[str, str] = {}
    cargo = (root / "Cargo.toml").read_text(encoding="utf-8")
    m = re.search(r"\[workspace\.package\][^\[]*?^version\s*=\s*\"([^\"]+)\"", cargo, re.M | re.S)
    if m:
        out["Cargo.toml"] = m.group(1)
    tauri = json.loads((root / "crates/mzlab-desktop/tauri.conf.json").read_text(encoding="utf-8"))
    out["crates/mzlab-desktop/tauri.conf.json"] = str(tauri.get("version", ""))
    m = re.search(r"^version:\s*\"?([^\"\s#]+)\"?", (root / "CITATION.cff").read_text(encoding="utf-8"), re.M)
    if m:
        out["CITATION.cff"] = m.group(1)
    zen = json.loads((root / ".zenodo.json").read_text(encoding="utf-8"))
    if "version" in zen:
        out[".zenodo.json"] = str(zen["version"])
    return out


def controlla(root: Path = ROOT, tag: str | None = None) -> list[str]:
    """Problems found (empty list = fine)."""
    v = versioni(root)
    errors = [f"{f}: no version found" for f in ("Cargo.toml", "crates/mzlab-desktop/tauri.conf.json", "CITATION.cff") if not v.get(f)]
    for f, x in v.items():
        if x and not SEMVER.match(x):
            errors.append(f"{f}: '{x}' is not X.Y.Z")
    if len({x for x in v.values() if x}) > 1:
        errors.append("the versions differ: " + ", ".join(f"{f}={x}" for f, x in v.items()))
    if tag is not None:
        want = tag[1:] if tag.startswith("v") else tag
        bad = [f for f, x in v.items() if x != want]
        if bad:
            errors.append(f"tag {tag}: these files do not say {want}: {', '.join(bad)}")
    return errors


def nota(versione: str, root: Path = ROOT) -> str:
    """The section '## [X.Y.Z]' of CHANGELOG.md (Keep a Changelog), without the title line."""
    v = versione[1:] if versione.startswith("v") else versione
    text = (root / "CHANGELOG.md").read_text(encoding="utf-8")
    m = re.search(rf"^## \[{re.escape(v)}\][^\n]*\n(.*?)(?=^## \[|^\[[^\]]+\]:\s|\Z)", text, re.M | re.S)
    body = m.group(1).strip() if m else ""
    if not body:
        raise SystemExit(f"CHANGELOG.md has no notes for {v}: write the section '## [{v}] - AAAA-MM-GG' before the tag")
    return body + "\n"


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--tag", help="tag of the release (vX.Y.Z): every file must say that version")
    ap.add_argument("--nota", metavar="VERSIONE", help="print the notes of that version from CHANGELOG.md")
    a = ap.parse_args(argv)
    if a.nota:
        sys.stdout.write(nota(a.nota))
        return 0
    errors = controlla(tag=a.tag)
    for e in errors:
        print("ERROR", e, file=sys.stderr)
    if not errors:
        print("version", next(iter(versioni().values())))
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
