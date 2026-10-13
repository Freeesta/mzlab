"""Download of the files of a benchmark manifest (`bench/manifesto/1`, kept in the private repository) with SHA-256 verification.

Resumable (HTTP Range on a `.part` file), one file at a time, never inside git: the raw data go to a cache directory given on the command line.
The manifest lists `file_grezzi` (raw data), `file_verita` (truth tables) and `file_metadati`; `--gruppo` picks which. A file whose hash differs is deleted
and the run fails. `--elimina` removes a downloaded file (disk is scarce: raw files of D1 weigh 7.4 GB).

    python3 tools/bench/scarica.py MANIFESTO.json CARTELLA [--gruppo file_grezzi] [--solo SA1.raw ...] [--elimina SA1.raw ...]
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

GRUPPI = ("file_grezzi", "file_verita", "file_metadati")
BLOCCO = 1 << 20
TENTATIVI = 5


def sha256_file(path, blocco: int = BLOCCO) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while chunk := f.read(blocco):
            h.update(chunk)
    return h.hexdigest()


def voci(manifesto: dict, gruppi=GRUPPI, solo=None) -> list[dict]:
    """Entries `{nome, url, sha256, byte?}` of the chosen groups, in manifest order."""
    out = [dict(v) for g in gruppi for v in manifesto.get(g, [])]
    if solo:
        mancanti = sorted(set(solo) - {v["nome"] for v in out})
        if mancanti:
            raise SystemExit(f"not in the manifest: {', '.join(mancanti)}")
        out = [v for v in out if v["nome"] in set(solo)]
    return out


def _scarica_parte(url: str, parte: Path, atteso: int | None) -> None:
    for tentativo in range(TENTATIVI):
        pos = parte.stat().st_size if parte.exists() else 0
        if atteso is not None and pos >= atteso:
            return
        req = urllib.request.Request(url, headers={"Range": f"bytes={pos}-"} if pos else {})
        try:
            with urllib.request.urlopen(req, timeout=60) as r:
                if pos and r.status != 206:               # server ignored Range: start over
                    pos = 0
                with open(parte, "ab" if pos else "wb") as f:
                    while chunk := r.read(BLOCCO):
                        f.write(chunk)
            return
        except urllib.error.HTTPError as e:
            if e.code == 416:                              # already complete
                return
            err = e
        except (urllib.error.URLError, TimeoutError, ConnectionError) as e:
            err = e
        print(f"  attempt {tentativo + 1}/{TENTATIVI} failed ({err}); retrying", file=sys.stderr)
        time.sleep(min(60, 2 ** (tentativo + 1)))
    raise SystemExit(f"download failed: {url}")


def scarica(voce: dict, cartella: Path) -> Path:
    """Download one entry into `cartella` and verify it; return the path. An existing file with the right hash is kept."""
    cartella.mkdir(parents=True, exist_ok=True)
    dest = cartella / voce["nome"]
    atteso = voce.get("sha256")
    if dest.exists() and atteso and sha256_file(dest) == atteso:
        return dest
    parte = dest.with_name(dest.name + ".part")
    _scarica_parte(voce["url"], parte, voce.get("byte"))
    os.replace(parte, dest)
    dim = dest.stat().st_size
    if voce.get("byte") is not None and dim != voce["byte"]:
        dest.unlink()
        raise SystemExit(f"{voce['nome']}: size {dim} != {voce['byte']}")
    got = sha256_file(dest)
    if atteso and got != atteso:
        dest.unlink()
        raise SystemExit(f"{voce['nome']}: SHA-256 {got} != {atteso}")
    if not atteso:
        print(f"  {voce['nome']}: no hash in the manifest, computed {got}", file=sys.stderr)
    return dest


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("manifesto")
    ap.add_argument("cartella")
    ap.add_argument("--gruppo", action="append", choices=GRUPPI, help="default: file_grezzi")
    ap.add_argument("--solo", nargs="*", help="only these file names")
    ap.add_argument("--elimina", nargs="*", help="delete these downloaded files and exit")
    a = ap.parse_args(argv)
    cartella = Path(a.cartella)
    if a.elimina:
        for n in a.elimina:
            (cartella / n).unlink(missing_ok=True)
        return 0
    m = json.loads(Path(a.manifesto).read_text(encoding="utf-8"))
    for v in voci(m, a.gruppo or ("file_grezzi",), a.solo):
        print(f"{v['nome']} ...", flush=True)
        p = scarica(v, cartella)
        print(f"  ok {p.stat().st_size} B sha256 {v.get('sha256', '')[:12]}", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
