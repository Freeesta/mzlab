"""Downloads the big native files listed in `GRANDI.tsv` (docs/studi/ROSINA.md D) into DEST: resumes a partial file, checks the bytes and the SHA-256, refuses to pass the ceiling.
A tool for the local chat (the big files never go to git or to the cloud). No data inside.
    python3 tools/rosina/scarica.py GRANDI.tsv DEST [--tetto-gb 20] [--solo SOTTOSTRINGA] [--prova]
Columns used: url, destinazione (relative path under DEST), byte, sha256 (a manifest of several files is not checked here: leave it empty)."""
import argparse
import csv
import hashlib
import os
import sys
import urllib.error
import urllib.request
from pathlib import Path

TETTO_GB = 20


def leggi(tsv: Path) -> list:
    with tsv.open(newline="", encoding="utf-8") as f:
        return [r for r in csv.DictReader(f, delimiter="\t") if (r.get("url") or "").strip() and not r["url"].startswith("#")]


def dentro(dest: Path, rel: str) -> Path:
    """The destination path, refused when it leaves DEST."""
    p = (dest / rel).resolve()
    if dest.resolve() not in p.parents:
        raise ValueError(f"destination outside DEST: {rel}")
    return p


def sha256(p: Path) -> str:
    h = hashlib.sha256()
    with p.open("rb") as f:
        for b in iter(lambda: f.read(1 << 20), b""):
            h.update(b)
    return h.hexdigest()


def totale(righe: list) -> int:
    return sum(int(r.get("byte") or 0) for r in righe)


def scarica(url: str, p: Path, byte: int) -> None:
    p.parent.mkdir(parents=True, exist_ok=True)
    parziale = p.with_name(p.name + ".parte")
    fatto = parziale.stat().st_size if parziale.exists() else 0
    if byte and fatto > byte:
        parziale.unlink()
        fatto = 0
    if byte and fatto == byte:
        os.replace(parziale, p)
        return
    req = urllib.request.Request(url, headers={"Range": f"bytes={fatto}-"} if fatto else {})
    try:
        r = urllib.request.urlopen(req, timeout=60)
    except urllib.error.HTTPError as e:
        if e.code == 416 and fatto:
            os.replace(parziale, p)
            return
        raise
    with r:
        if fatto and r.status != 206:
            fatto = 0
        with parziale.open("ab" if fatto else "wb") as f:
            for b in iter(lambda: r.read(1 << 20), b""):
                f.write(b)
    os.replace(parziale, p)


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("tsv", type=Path)
    ap.add_argument("dest", type=Path)
    ap.add_argument("--tetto-gb", type=float, default=TETTO_GB)
    ap.add_argument("--solo", help="only the rows whose `destinazione` contains this text")
    ap.add_argument("--prova", action="store_true", help="list what would be downloaded")
    a = ap.parse_args(argv)
    righe = [r for r in leggi(a.tsv) if not a.solo or a.solo in (r.get("destinazione") or "")]
    peso = totale(righe)
    if peso > a.tetto_gb * 1e9:
        print(f"refused: {peso / 1e9:.1f} GB asked, the ceiling is {a.tetto_gb} GB", file=sys.stderr)
        return 2
    errori = 0
    for r in righe:
        rel, url, byte = r.get("destinazione") or r.get("percorso") or "", r["url"].strip(), int(r.get("byte") or 0)
        if not url.startswith("https://"):
            print(f"skip (not https): {rel}", file=sys.stderr)
            errori += 1
            continue
        try:
            p = dentro(a.dest, rel)
        except ValueError as e:
            print(e, file=sys.stderr)
            errori += 1
            continue
        if a.prova:
            print(f"{rel}\t{byte}")
            continue
        if not (p.exists() and (not byte or p.stat().st_size == byte)):
            scarica(url, p, byte)
        if byte and p.stat().st_size != byte:
            print(f"wrong size: {rel} ({p.stat().st_size} of {byte})", file=sys.stderr)
            errori += 1
            continue
        atteso = (r.get("sha256") or "").strip().lower()
        if len(atteso) == 64 and sha256(p) != atteso:
            print(f"wrong SHA-256: {rel}", file=sys.stderr)
            errori += 1
            continue
        print(f"ok {rel}")
    return 1 if errori else 0


if __name__ == "__main__":
    raise SystemExit(main())
