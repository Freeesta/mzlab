"""Ground truth `verita/1` (docs/studi/ROSINA.md F.1) from a `scandump/1` written by an oracle (RFR) or by our reader (docs/agenti/scandump.md).
A tool of the test bench, never part of the site: no calculation of the program lives here.
    python3 tools/rosina/verita_scandump.py DUMP.scandump.jsonl.gz [--oracolo NOME VERSIONE OPZIONI] > FILE.verita.json
Standard library only. The peaks come from the `peaks` rows of the dump; the metadata of the header go in `metadati` as the oracle wrote them."""
import argparse
import datetime
import gzip
import importlib.util
import json
import platform
import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent


def _verita():
    """tools/rosina/verita.py by path: tools/bench/verita.py has the same name."""
    spec = importlib.util.spec_from_file_location("rosina_verita", _HERE / "verita.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


KINDS = {"tic": "tic", "bpc": "bpc", "pressure": "pressione", "pda": "pda", "analog": "analogico", "srm": "srm", "sim": "sim"}
POLARITY = {"+": 1, "-": -1}


def _rows(path: Path):
    opener = gzip.open if str(path).endswith(".gz") else open
    with opener(path, "rt", encoding="utf-8") as f:
        for line in f:
            if line.strip():
                yield json.loads(line)


def _scan(s: dict) -> dict:
    last = (s.get("reactions") or [None])[-1]
    iso = None
    prec = None
    ce = None
    act = None
    if last:
        prec = last.get("precursor")
        ce = last.get("energy")
        act = last.get("activation")
        if prec is not None:
            half = last["iso_width"] / 2.0 if last.get("iso_width") is not None else None
            iso = {"mz": prec, "inf": half, "sup": half}
    return {
        "id": f"controllerType=0 controllerNumber=1 scan={s['n']}",
        "rt_min": s["rt"],
        "livello": s["level"],
        "polarita": POLARITY.get(s.get("polarity"), 0),
        "precursore_mz": prec,
        "isolamento": iso,
        "ce": ce,
        "attivazione": act,
        "profilo": s.get("mode") == "profile",
        "n_punti": s.get("n_peaks", 0),
        "tic": s["tic"],
        "bpi_mz": s.get("bp_mz"),
        "bpi": s.get("bp_int"),
    }


def _chrom(c: dict, points: int) -> dict:
    t, y = c.get("time", []), c.get("values", [])
    n = len(y)
    idx = sorted({round(i * (n - 1) / (points - 1)) for i in range(points)}) if n > 1 else list(range(n))
    return {
        "id": c["id"],
        "tipo": KINDS.get(c.get("kind"), "altro"),
        "n": n,
        "somma": sum(y),
        "massimo": max(y) if y else 0.0,
        "campione": [[t[i] if i < len(t) else 0.0, y[i]] for i in idx],
    }


def da_scandump(path: Path, oracolo: tuple) -> dict:
    v = _verita()
    head, scans, peaks, chroms = None, [], {}, []
    for r in _rows(path):
        t = r.get("type")
        if t == "file":
            head = r
        elif t == "scan":
            scans.append(r)
        elif t == "peaks":
            peaks[r["n"]] = r
        elif t == "chromatogram":
            chroms.append(r)
    if head is None or head.get("schema") != "scandump/1":
        raise SystemExit(f"{path}: not a scandump/1")
    scans.sort(key=lambda s: s["n"])
    scansioni = [_scan(s) for s in scans]
    at = {s["n"]: i for i, s in enumerate(scans)}
    pt = {str(at[n]): {"mz": p["mz"], "intensita": p["int"]} for n, p in peaks.items() if n in at}
    cromatogrammi = [_chrom(c, v.PUNTI_CROMATOGRAMMA) for c in chroms]
    cs, cc = {}, {}
    for s in scansioni:
        cs[f"ms{s['livello']}"] = cs.get(f"ms{s['livello']}", 0) + 1
    for c in cromatogrammi:
        cc[c["tipo"]] = cc.get(c["tipo"], 0) + 1
    metadati = {}
    for k, nome in (("instrument", "scandump:instrument"), ("format_version", "scandump:format_version")):
        if head.get(k) is not None:
            metadati[nome] = {"valore": head[k], "produttore": head.get("producer", "")}
    nome, ver, opz = oracolo
    return {
        "schema": "verita/1",
        "file": {"nome": head["name"], "sha256": head.get("sha256"), "campione": None},
        "oracolo": {
            "nome": nome or head.get("producer", "scandump"),
            "versione": ver or head.get("producer_version", "?"),
            "opzioni": opz,
            "data": datetime.date.today().isoformat(),
            "piattaforma": f"{platform.system()} {platform.machine()}",
        },
        "conteggi": {"spettri": cs, "cromatogrammi": cc},
        "scansioni": scansioni,
        "punti": pt,
        "cromatogrammi": cromatogrammi,
        "metadati": metadati,
    }


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("dump", type=Path)
    ap.add_argument("--oracolo", nargs=3, metavar=("NOME", "VERSIONE", "OPZIONI"), default=["", "", "scandump/1"])
    a = ap.parse_args(argv)
    d = da_scandump(a.dump, tuple(a.oracolo))
    for problema in _verita().valida(d):
        print(problema, file=sys.stderr)
    json.dump(d, sys.stdout, indent=1, ensure_ascii=False)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
