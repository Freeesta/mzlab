#!/usr/bin/env python3
"""Metrics of every main commit, to see whether the project improves over time (docs/agenti/metriche.md). Standard library only.

    python3 tools/metriche.py --registra --sito SITE --dati DIR [--sha SHA] [--ci]   # measure and append one line to DIR/metriche-YYYY-MM.jsonl
    python3 tools/metriche.py --aggiungi nome=valore ... --dati DIR [--sha SHA]       # hand-measured numbers (private-data benches) for a commit
    python3 tools/metriche.py --bench FILE.json ...                                    # synthetic bench numbers {name: number} with --registra/--aggiungi
    python3 tools/metriche.py --rapporto [--dati DIR] [--html FILE]                    # trend table and HTML page (tools/metriche_trend.py)

A line is `{v, sha, ts, part, ...}`; a commit may have several lines (parts `site`, `ci`, `manual`) and readers merge them by sha
(`leggi_commit`). Lines are only appended; the data live on the branch `stat-ci`, never in main. The report is tools/metriche_trend.py."""
from __future__ import annotations

import argparse
import glob
import gzip
import json
import os
import re
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

RADICE = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(Path(__file__).resolve().parent))
VERSIONE = 1   # field `v`; bump when a field changes meaning
STEM = "metriche"


def comprimi(dati: bytes) -> dict:
    """Raw, gzip -9 and brotli -q 11 sizes; brotli only if the `brotli` tool is installed (CI runners have it), else None."""
    out = {"raw": len(dati), "gz": len(gzip.compress(dati, 9, mtime=0)), "br": None}
    exe = shutil.which("brotli")
    if exe:
        r = subprocess.run([exe, "-c", "-q", "11"], input=dati, capture_output=True)
        if r.returncode == 0:
            out["br"] = len(r.stdout)
    return out


def misura_wasm(sito: Path) -> dict:
    """Every built .wasm: raw/gz/br next to its cap and the headroom in bytes and %. Caps come from tools/ci_wasm_dimensioni.py."""
    import ci_wasm_dimensioni as dim
    out = {}
    for nome, m in dim.misura(sito).items():
        c = comprimi((sito / nome).read_bytes())
        tetto = m["tetto"]
        c.update({"cap": tetto, "head_b": tetto - m["byte"] if tetto else None, "head_pct": round(100 * (1 - m["byte"] / tetto), 2) if tetto else None})
        out[nome] = c
    return out


def misura_avvio(radice: Path = RADICE) -> dict:
    """Bytes of the start-up scripts (list `L` of mzlab/web/carica.js) against MAX_TOTAL of tests/test_peso_avvio.py."""
    web = radice / "mzlab" / "web"
    corpo = re.search(r"var L = \[(.*?)\];", (web / "carica.js").read_text(encoding="utf-8"), re.S).group(1)
    nomi = [n[2:] if n.startswith("m:") else n for n in re.findall(r'"([^"]+)"', corpo)]
    raw = gz = 0
    for n in nomi:
        dati = (web / n).read_bytes()
        raw += len(dati)
        gz += len(gzip.compress(dati, 9, mtime=0))
    tetto = int(re.search(r"^MAX_TOTAL\s*=\s*([\d_]+)", (radice / "tests" / "test_peso_avvio.py").read_text(encoding="utf-8"), re.M).group(1).replace("_", ""))
    return {"files": len(nomi), "raw": raw, "gz": gz, "cap": tetto, "head_b": tetto - raw, "head_pct": round(100 * (1 - raw / tetto), 2)}


def conta_test(radice: Path = RADICE) -> dict:
    """Test counts by reading the sources (no build, no run): pytest `def test_` (a parametrised test counts once), Rust `#[test]`, e2e scripts."""
    def conta(rx, files):
        return sum(len(re.findall(rx, f.read_text(encoding="utf-8", errors="replace"), re.M)) for f in files)
    return {"pytest": conta(r"^\s*(?:async )?def test_", sorted((radice / "tests").rglob("test_*.py"))),
            "cargo": conta(r"#\[(?:tokio::)?test\b", sorted((radice / "crates").rglob("*.rs"))),
            "e2e": len(list((radice / "tests_e2e").glob("e2e*.py")))}


def git(*args: str) -> str:
    return subprocess.run(["git", *args], cwd=RADICE, capture_output=True, text=True, check=True).stdout.strip()


def carica_bench(path: str | None, extra: list[str]) -> dict:
    """Numbers from a JSON `{name: number}` file and from `name=value` pairs; anything else is refused."""
    out = {}
    if path:
        for k, v in json.loads(Path(path).read_text(encoding="utf-8")).items():
            out[k] = float(v)
    for par in extra:
        k, _, v = par.partition("=")
        if not k or not _:
            raise SystemExit(f"--aggiungi wants name=value, got {par!r}")
        out[k] = float(v)
    return out


def riga(part: str, sha: str, ts: str, **sezioni) -> dict:
    return {"v": VERSIONE, "sha": sha, "ts": ts, "part": part, "recorded": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"), **sezioni}


def file_attivo(dati: str, ora: datetime | None = None) -> str:
    return f"{dati}/{STEM}-{(ora or datetime.now(timezone.utc)).strftime('%Y-%m')}.jsonl"


def accoda(righe: list[dict], dati: str) -> int:
    """Append lines to this month's file; a (sha, part) that is already there with the same content is skipped."""
    os.makedirs(dati, exist_ok=True)
    visti = {(r["sha"], r["part"], json.dumps({k: v for k, v in r.items() if k != "recorded"}, sort_keys=True)) for r in leggi_righe(dati)}
    nuove = [r for r in righe if (r["sha"], r["part"], json.dumps({k: v for k, v in r.items() if k != "recorded"}, sort_keys=True)) not in visti]
    with open(file_attivo(dati), "a", encoding="utf-8") as f:
        for r in nuove:
            f.write(json.dumps(r, ensure_ascii=False, separators=(",", ":")) + "\n")
    return len(nuove)


def leggi_righe(dati: str) -> list[dict]:
    out = []
    for p in sorted(glob.glob(f"{dati}/{STEM}-????-??.jsonl")):
        with open(p, encoding="utf-8") as f:
            out += [json.loads(r) for r in f if r.strip()]
    return out


def leggi_commit(dati: str) -> list[dict]:
    """One merged record per commit, oldest first: the parts are merged section by section, the latest line wins."""
    per_sha: dict[str, dict] = {}
    for r in sorted(leggi_righe(dati), key=lambda r: r.get("recorded", "")):
        m = per_sha.setdefault(r["sha"], {"sha": r["sha"], "ts": r["ts"]})
        for k, v in r.items():
            if k in ("v", "sha", "ts", "part", "recorded"):
                continue
            if isinstance(v, dict) and isinstance(m.get(k), dict) and k in ("bench", "manual"):
                m[k] = {**m[k], **v}
            else:
                m[k] = v
    return sorted(per_sha.values(), key=lambda m: m["ts"])


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--registra", action="store_true", help="measure the commit and append the lines")
    ap.add_argument("--aggiungi", nargs="+", metavar="NAME=VALUE", help="hand-measured numbers (e.g. bench_mappa on hrc) for the commit")
    ap.add_argument("--bench", metavar="FILE", help="JSON {name: number} of synthetic bench results")
    ap.add_argument("--sito", help="built site directory (static/ with the .wasm files)")
    ap.add_argument("--ci", action="store_true", help="also read the CI facts of the commit from the GitHub API")
    ap.add_argument("--rapporto", action="store_true", help="print the trend table (better/worse than 7 and 30 days ago)")
    ap.add_argument("--html", metavar="FILE", help="with --rapporto: also write a self-contained HTML page with sparklines")
    ap.add_argument("--sha", help="commit (default HEAD)")
    ap.add_argument("--dati", default="metriche", help="directory with the monthly files (the worktree of stat-ci)")
    a = ap.parse_args(argv)
    if a.rapporto:
        import metriche_trend as t
        rows = t.analyse(leggi_commit(a.dati))
        print(t.text_report(rows))
        if a.html:
            Path(a.html).write_text(t.html_report(rows, datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")), encoding="utf-8")
        return 0
    sha = git("rev-parse", a.sha or "HEAD")
    # commit time in UTC, so lines sort correctly whatever the committer's offset
    ts = datetime.fromisoformat(git("show", "-s", "--format=%cI", sha)).astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    righe = []
    if a.registra:
        sezioni = {"startup": misura_avvio(), "tests": conta_test()}
        if a.sito:
            sezioni["wasm"] = misura_wasm(Path(a.sito))
        if a.bench:
            sezioni["bench"] = carica_bench(a.bench, [])
        righe.append(riga("site", sha, ts, **sezioni))
        if a.ci:
            import metriche_ci
            from ci_metrics import api_get
            righe.append(riga("ci", sha, ts, ci=metriche_ci.facts(sha, git("show", "-s", "--format=%s", sha), api_get)))
    if a.aggiungi:
        righe.append(riga("manual", sha, ts, manual=carica_bench(None, a.aggiungi)))
    if not righe:
        ap.error("nothing to do: use --registra or --aggiungi")
    print(f"{accoda(righe, a.dati)} line(s) appended for {sha[:10]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
