"""Checks the roadmap queue in docs/coda/: CODA.md (one table row per task), BRIEF.md and docs/coda/prompt/<id>.md.

Every row has an existing prompt file, no prompt file is an orphan, every prompt's first line is exactly the BRIEF line, it has the five sections (goal, read first, done means, out of scope,
follow-ups) and stays small,
ids are unique upper-case words of at most 24 characters, the `needs first` cell names only ids that are still rows (a chat
that deletes its row deletes its id there too) or a running chat written `chat:<ID>` (a chat started with an inline prompt has no
row; its PR deletes the token), and no document is over its byte cap. Standard library only.

    python3 tools/controlla_coda.py      # prints one FAIL line per problem, exit 1 if any
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BRIEF_LINE = "Read docs/coda/BRIEF.md."
MAX_CODA, MAX_BRIEF, MAX_PROMPT, MAX_ID = 25_600, 6_000, 3_500, 24
ID_RE = re.compile(r"[A-Z][A-Z0-9]*(?:-[A-Z0-9]+)*")
SECTIONS = ("**Goal.**", "**Read first.**", "**Done means.**", "**Out of scope.**", "**Follow-ups.**")
OWNER = "needs owner decision:"
CHAT = "chat:"


def rows(text: str) -> list[tuple[int, list[str]]]:
    """The data rows of the first table: (line number, the four cells)."""
    out = []
    for n, line in enumerate(text.splitlines(), 1):
        if not line.startswith("|"):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) != 4 or cells[0] == "id" or set(cells[0]) <= set("-: "):
            continue
        out.append((n, cells))
    return out


def peggiori(root: Path = ROOT) -> list[str]:
    coda_dir = root / "docs" / "coda"
    coda = coda_dir / "CODA.md"
    if not coda.is_file():
        return ["docs/coda/CODA.md: missing -> the queue file is required"]
    bad: list[str] = []
    size = coda.stat().st_size
    if size > MAX_CODA:
        bad.append(f"docs/coda/CODA.md: {size} B over {MAX_CODA} B -> shorten the notes")
    brief = coda_dir / "BRIEF.md"
    if not brief.is_file():
        bad.append("docs/coda/BRIEF.md: missing -> every prompt starts from it")
    elif brief.stat().st_size > MAX_BRIEF:
        bad.append(f"docs/coda/BRIEF.md: {brief.stat().st_size} B over {MAX_BRIEF} B -> shorten it")
    table = rows(coda.read_text(encoding="utf-8"))
    ids = [c[0] for _, c in table]
    seen: set[str] = set()
    for n, (rid, prompt, needs, _notes) in table:
        where = f"docs/coda/CODA.md:{n}"
        if not ID_RE.fullmatch(rid) or len(rid) > MAX_ID:
            bad.append(f"{where}: id `{rid}` must be upper case words joined by hyphens, at most {MAX_ID} characters")
        if rid in seen:
            bad.append(f"{where}: id `{rid}` appears twice -> ids are unique")
        seen.add(rid)
        want = f"docs/coda/prompt/{rid}.md"
        if prompt.strip("`") != want:
            bad.append(f"{where}: prompt cell of {rid} must be `{want}`")
        if not (root / want).is_file():
            bad.append(f"{where}: {want} does not exist -> every row has a ready prompt")
        if needs != "-" and not needs.startswith(OWNER):
            for dep in (d.strip() for d in needs.split(",")):
                if dep.startswith(CHAT):
                    if not ID_RE.fullmatch(dep[len(CHAT):]):
                        bad.append(f"{where}: `needs first` of {rid} has a bad chat id `{dep}`")
                elif dep not in ids:
                    bad.append(f"{where}: `needs first` of {rid} names `{dep}`, which is not a row -> remove it, or write `-`")
                elif dep == rid:
                    bad.append(f"{where}: {rid} needs itself")
    folder = coda_dir / "prompt"
    for p in sorted(folder.glob("*.md")) if folder.is_dir() else []:
        rel = p.relative_to(root).as_posix()
        if p.stem not in ids:
            bad.append(f"{rel}: orphan prompt, no row in CODA.md -> delete it or add the row")
        if p.stat().st_size > MAX_PROMPT:
            bad.append(f"{rel}: {p.stat().st_size} B over {MAX_PROMPT} B -> shorten it")
        body = p.read_text(encoding="utf-8")
        if body.split("\n", 1)[0].strip() != BRIEF_LINE:
            bad.append(f"{rel}: its first line must be exactly `{BRIEF_LINE}`")
        for sec in SECTIONS:
            if not re.search(rf"^{re.escape(sec)}", body, re.M):
                bad.append(f"{rel}: missing the section {sec}")
    return bad


def main() -> int:
    bad = peggiori()
    for b in bad:
        print("FAIL " + b)
    if not bad:
        print("docs/coda: queue consistent")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
