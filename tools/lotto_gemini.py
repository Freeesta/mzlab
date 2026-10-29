#!/usr/bin/env python3
"""Joins several project files into ONE document to hand to an assistant that takes a single upload.

    python3 tools/lotto_gemini.py LISTA.txt -o lotto.txt            # every file of the list
    python3 tools/lotto_gemini.py docs/agenti/caricamento-mappa.md --lotto 2 -o lotto2.txt
    python3 tools/lotto_gemini.py LISTA.txt --max-kb 200 --controlla  # sizes only, writes nothing

The list has one path per line (relative to the repository root); empty lines and lines starting with `#` are ignored.
A line `@lotto N` starts the batch N: with `--lotto N` only the paths after it (up to the next `@lotto`) are joined, so a Markdown file
that holds the lists (docs/agenti/caricamento-mappa.md) can be passed as it is: only lines that are paths of existing files, or `@lotto`, count.
Each file is written between `=== FILE: path (n of N) — ... ===` and `=== FINE FILE ===`. A warning on stderr (exit code 0, or 1 with `--strict`)
says when the document is over the limit (`--max-kb`, 200 by default); a listed path that does not exist is an error. Standard library only.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MAX_KB = 200
INTESTAZIONE = "=== FILE: {path} ({n} di {tot}) — file separati nel progetto, qui uniti in un unico documento ==="
FINE = "=== FINE FILE ==="


def leggi_elenco(testo: str, lotto: int | None = None, radice: Path = ROOT) -> list[str]:
    """The paths of a list; with `lotto`, only those of that `@lotto N` section. Lines that are not paths of existing files are skipped
    (a Markdown file with prose can be passed), except in a plain list without `@lotto` where a missing path is reported by `unisci`."""
    corrente: int | None = None
    ha_sezioni = any(r.strip().startswith("@lotto") for r in testo.splitlines())
    out: list[str] = []
    for riga in testo.splitlines():
        r = riga.strip().strip("`")
        if not r or r.startswith("#"):
            continue
        if r.startswith("@lotto"):
            try:
                corrente = int(r.split()[1])
            except (IndexError, ValueError):
                raise SystemExit(f"bad lotto line: {riga!r}")
            continue
        if lotto is not None and corrente != lotto:
            continue
        if ha_sezioni and (corrente is None or not (radice / r).is_file()):
            continue
        if not ha_sezioni and lotto is None and not (radice / r).is_file() and " " in r:
            continue
        out.append(r)
    return out


def unisci(percorsi: list[str], radice: Path = ROOT) -> tuple[str, int]:
    """The joined document and its size in bytes. A missing file raises FileNotFoundError."""
    parti: list[str] = []
    for i, p in enumerate(percorsi, 1):
        f = radice / p
        if not f.is_file():
            raise FileNotFoundError(p)
        parti.append(INTESTAZIONE.format(path=p, n=i, tot=len(percorsi)) + "\n" + f.read_text(encoding="utf-8", errors="replace").rstrip("\n") + "\n" + FINE + "\n")
    doc = "\n".join(parti)
    return doc, len(doc.encode("utf-8"))


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("elenco", help="file with the list of paths (or a Markdown file with `@lotto N` sections)")
    ap.add_argument("-o", "--out", help="output file (default: stdout)")
    ap.add_argument("--lotto", type=int, help="only the batch N of the list")
    ap.add_argument("--max-kb", type=int, default=MAX_KB, help=f"warn above this size (default {MAX_KB})")
    ap.add_argument("--strict", action="store_true", help="exit code 1 when over the limit")
    ap.add_argument("--controlla", action="store_true", help="only print the sizes, write nothing")
    a = ap.parse_args(argv)
    percorsi = leggi_elenco(Path(a.elenco).read_text(encoding="utf-8"), a.lotto)
    if not percorsi:
        print("no files in the list", file=sys.stderr)
        return 1
    try:
        doc, n = unisci(percorsi)
    except FileNotFoundError as e:
        print(f"missing file: {e}", file=sys.stderr)
        return 1
    kb = n / 1024
    if a.controlla:
        for p in percorsi:
            print(f"{(ROOT / p).stat().st_size / 1024:7.1f} KB  {p}")
    print(f"{len(percorsi)} files, {kb:.1f} KB", file=sys.stderr)
    over = kb > a.max_kb
    if over:
        print(f"WARNING: {kb:.1f} KB is over the limit of {a.max_kb} KB", file=sys.stderr)
    if not a.controlla:
        if a.out:
            Path(a.out).write_text(doc, encoding="utf-8")
        else:
            sys.stdout.write(doc)
    return 1 if over and a.strict else 0


if __name__ == "__main__":
    sys.exit(main())
