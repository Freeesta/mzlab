"""Builds the public spectral-library packages of the site (never committed): the CI of Pages runs this after build_site.py.

    python tools/librerie_pacchetti.py --out site/static/librerie            # downloads the pinned MassBank release (Zenodo)
    python tools/librerie_pacchetti.py --out DIR --zip MassBank.zip          # an already downloaded copy (tests, local use)
    python tools/librerie_pacchetti.py --check-catalog                       # schema of tools/librerie_catalogo.json (exit 1 on any problem)

1. The MassBank-data release below is downloaded and its SHA-256 is checked (a different file stops the build).
2. Only the ESI-MS2 records whose licence is CC0, CC BY or CC BY-SA are kept (never NC, never the German data licence, never GNPS / MoNA / HMDB / NIST:
   those only come from the user). The 61 records of the UniTO standards (tests/golden/librerie/massbank_unito, CC BY) are a package of their own.
3. The records of each polarity are built into a `.mzlib` by the Rust builder (crates/mzlab-lib, example `build`), cut into pieces of 4 MiB with a SHA-256
   each (resumable download without Range, checked piece by piece, no change to the CSP) and described in `catalogo.json` (format `librerie/2`: the packages plus `fonti`, the hand-written list of tools/librerie_catalogo.json with the mode `site`|`external`|`link` of each source;
   `esterne` stays for the readers of `librerie/1`).

Where the download is not possible (network, Zenodo down) the script writes nothing and exits with 0 and a warning: the site is still published,
without packages (the page says so). A wrong SHA-256 of the download is an error.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import time
import urllib.request
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

VERSION = "2026.03"
ZENODO_RECORD = "19590880"
ZIP_URL = f"https://zenodo.org/api/records/{ZENODO_RECORD}/files/MassBank/MassBank-data-{VERSION}.zip/content"
ZIP_SHA256 = "12c6a922929f9bf2484c46586b6101512a415e44680558883b0630c6e47905fb"
DOI = f"10.5281/zenodo.{ZENODO_RECORD}"
KEEP_LICENSES = ("CC0", "CC BY", "CC BY-SA")
PIECE = 4 * 1024 * 1024
CATALOG_FORMAT = "librerie/2"
CATALOG_SOURCES = ROOT / "tools" / "librerie_catalogo.json"
FREE_LICENSES = ("CC0-1.0", "CC-BY-4.0", "CC-BY-SA-4.0")      # the only ones a `site` source may have (redistribution with attribution)
MODES = ("site", "external", "link")
GROUPS = ("base", "specialised")
KINDS = ("experimental", "in_silico")
TECHS = ("lc_msms", "gc_ei", "msn")
UNITO_DIR = ROOT / "tests" / "golden" / "librerie" / "massbank_unito"

ATTRIBUTION = ("Spettri di MassBank Europe (https://massbank.eu), release " + VERSION + " (doi:" + DOI + "): ogni record conserva autori, accessione e licenza "
               "(CC0, CC BY o CC BY-SA). Il pacchetto nel suo insieme si usa secondo CC BY-SA 4.0.")
UNITO_ATTRIBUTION = ("Spettri degli standard misurati da Cristaudo F, Fabbri D, Calza P, Università di Torino (formato MassBank), licenza CC BY 4.0: "
                     "citare gli autori e l'accessione del record.")


def sha256_file(p: Path) -> str:
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for b in iter(lambda: f.read(1 << 20), b""):
            h.update(b)
    return h.hexdigest()


def download(url: str, dest: Path, want: str) -> None:
    last = None
    for attempt in range(4):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "mzlab-ci"})
            with urllib.request.urlopen(req, timeout=120) as r, open(dest, "wb") as f:
                shutil.copyfileobj(r, f, 1 << 20)
            break
        except Exception as e:                         # network: retried, then reported as «not available»
            last = e
            time.sleep(2 ** attempt * 3)
    else:
        raise ConnectionError(str(last))
    got = sha256_file(dest)
    if got != want:
        raise SystemExit(f"{dest.name}: SHA-256 {got} is not the expected {want}: build stopped")


def _field(text: str, key: str) -> str | None:
    m = re.search(r"^" + re.escape(key) + r"\s*(.*)$", text, re.M)
    return m.group(1).strip() if m else None


def _ms(text: str, key: str) -> str | None:
    m = re.search(r"^AC\$MASS_SPECTROMETRY:\s*" + re.escape(key) + r"\s+(.*)$", text, re.M)
    return m.group(1).strip() if m else None


def keep(text: str):
    """(polarity, licence) of a MassBank record that goes into the packages, else None."""
    lic = _field(text, "LICENSE:")
    if lic not in KEEP_LICENSES:
        return None
    if _ms(text, "MS_TYPE") != "MS2" or not (_ms(text, "IONIZATION") or "").startswith("ESI"):
        return None
    mode = (_ms(text, "ION_MODE") or "").upper()
    if mode not in ("POSITIVE", "NEGATIVE") or "PK$PEAK:" not in text:
        return None
    return ("pos" if mode == "POSITIVE" else "neg"), lic


def filter_zip(zpath: Path, work: Path):
    """Writes pos.txt and neg.txt (the kept records one after the other) and returns the counts per polarity, licence and contributor."""
    out = {"pos": open(work / "pos.txt", "w", encoding="utf-8", newline="\n"), "neg": open(work / "neg.txt", "w", encoding="utf-8", newline="\n")}
    stats = {"pos": {"n": 0, "licenze": {}, "contributori": {}}, "neg": {"n": 0, "licenze": {}, "contributori": {}}}
    try:
        with zipfile.ZipFile(zpath) as z:
            for info in z.infolist():
                base = info.filename.rsplit("/", 1)[-1]
                if info.is_dir() or not (base.startswith("MSBNK-") and base.endswith(".txt")):
                    continue
                text = z.read(info).decode("utf-8", "replace")
                k = keep(text)
                if not k:
                    continue
                pol, lic = k
                parts = info.filename.split("/")
                contrib = parts[-2] if len(parts) >= 3 else "?"
                out[pol].write(text if text.endswith("\n") else text + "\n")
                s = stats[pol]
                s["n"] += 1
                s["licenze"][lic] = s["licenze"].get(lic, 0) + 1
                s["contributori"][contrib] = s["contributori"].get(contrib, 0) + 1
    finally:
        for f in out.values():
            f.close()
    return stats


def build_mzlib(builder: list[str], src: Path, dest: Path) -> None:
    r = subprocess.run([*builder, str(src), str(dest)], capture_output=True, text=True)
    if r.returncode != 0 or not dest.is_file():
        raise SystemExit(f"the .mzlib builder failed on {src.name}: {r.stderr[-400:]}")
    print("  " + r.stdout.strip().splitlines()[-1] if r.stdout.strip() else f"  {dest.name} built")


def header(p: Path) -> dict:
    """What the 36-byte header of a `.mzlib` says (docs/agenti/librerie.md): minimum reader version, spectra, peaks."""
    with open(p, "rb") as f:
        h = f.read(36)
    if h[:4] != b"MZLB":
        raise SystemExit(f"{p.name}: not a .mzlib")
    return {"lettore_min": struct.unpack_from("<H", h, 6)[0], "spettri": struct.unpack_from("<I", h, 8)[0], "picchi": struct.unpack_from("<Q", h, 16)[0]}


def split(p: Path, out_dir: Path, lib_id: str) -> list[dict]:
    """Cuts a file into pieces of PIECE bytes (`<id>/000.bin`, …), each with its SHA-256."""
    d = out_dir / lib_id
    d.mkdir(parents=True, exist_ok=True)
    pieces = []
    with open(p, "rb") as f:
        i = 0
        while True:
            b = f.read(PIECE)
            if not b:
                break
            name = f"{i:03d}.bin"
            (d / name).write_bytes(b)
            pieces.append({"file": f"{lib_id}/{name}", "dimensione": len(b), "sha256": hashlib.sha256(b).hexdigest()})
            i += 1
    return pieces


def entry(lib_id: str, nome: str, pol: int, version: str, source: str, link: str, lic_text: str, lics: dict, contrib: dict, mzlib: Path, out_dir: Path, attribution: str) -> dict:
    h = header(mzlib)
    return {"id": lib_id, "nome": nome, "versione": version, "sorgente": source, "url_sorgente": link, "licenza": lic_text, "licenze": lics, "contributori": contrib,
            "polarita": pol, "spettri": h["spettri"], "picchi": h["picchi"], "dimensione": mzlib.stat().st_size, "sha256": sha256_file(mzlib),
            "lettore_min": h["lettore_min"], "attribuzione": attribution, "pezzi": split(mzlib, out_dir, lib_id)}


def load_sources() -> list[dict]:
    return json.loads(CATALOG_SOURCES.read_text(encoding="utf-8"))["fonti"]


def check_sources(fonti: list[dict]) -> list[str]:
    """Problems of the hand-written list: ids unique and plain, fields present, `site` only with a free licence, `external` only with a download, `verificato` a date."""
    bad, seen = [], set()
    need = ("id", "gruppo", "nome", "url", "licenza", "formato", "kind", "tech", "pol", "doi", "verificato", "modo", "download", "dimensione", "sha256")
    for f in fonti:
        i = f.get("id", "?")
        miss = [k for k in need if k not in f]
        if miss:
            bad.append(f"{i}: missing {', '.join(miss)}")
            continue
        if not re.fullmatch(r"[A-Za-z0-9_-]+", i) or i in seen:
            bad.append(f"{i}: id not plain or repeated")
        seen.add(i)
        if f["modo"] not in MODES or f["gruppo"] not in GROUPS or f["kind"] not in KINDS or f["tech"] not in TECHS or f["pol"] not in (-1, 0, 1):
            bad.append(f"{i}: mode, group, kind, tech or polarity out of range")
        if f["modo"] == "site" and f["licenza"] not in FREE_LICENSES:
            bad.append(f"{i}: `site` needs a free licence, not {f['licenza']}")
        if f["modo"] == "external" and not str(f["download"] or "").startswith("https://"):
            bad.append(f"{i}: `external` needs an https download")
        if not re.fullmatch(r"\d{4}-\d\d-\d\d", str(f["verificato"])):
            bad.append(f"{i}: `verificato` is not a date")
        if f["sha256"] is not None and not re.fullmatch(r"[0-9a-f]{64}", f["sha256"]):
            bad.append(f"{i}: sha256 malformed")
    return bad


def external_entries(fonti: list[dict]) -> list[dict]:
    """The `external` sources in the shape the readers of `librerie/1` know (the desktop app downloads these)."""
    keys = ("id", "nome", "url", "formato", "dimensione", "sha256", "licenza")
    return [{**{k: f[k] for k in keys}, "url": f["download"], "sorgente": f["nome"]} for f in fonti if f["modo"] == "external"]


def sources_with_packages(fonti: list[dict], libs: list[dict]) -> list[dict]:
    """`site` sources take size and SHA-256 from the package built here; one whose package was not built is left out."""
    by = {p["id"]: p for p in libs}
    out = []
    for f in fonti:
        if f["modo"] != "site":
            out.append(f)
        elif f["id"] in by:
            out.append({**f, "dimensione": by[f["id"]]["dimensione"], "sha256": by[f["id"]]["sha256"]})
    return out


def default_builder() -> list[str]:
    exe = ROOT / "target" / "release" / "examples" / ("build.exe" if os.name == "nt" else "build")
    if not exe.is_file():
        subprocess.run(["cargo", "build", "--locked", "--release", "-p", "mzlab-lib", "--example", "build"], cwd=ROOT, check=True)
    return [str(exe)]


def unito_records(dest: Path) -> tuple[int, dict]:
    """The UniTO standards as one MassBank text; returns the count and the licences."""
    files = sorted(UNITO_DIR.glob("MSBNK-UniTO-*.txt"))
    lics: dict = {}
    with open(dest, "w", encoding="utf-8", newline="\n") as o:
        for f in files:
            t = f.read_text(encoding="utf-8")
            lic = _field(t, "LICENSE:") or "?"
            lics[lic] = lics.get(lic, 0) + 1
            o.write(t if t.endswith("\n") else t + "\n")
    return len(files), lics


def run(out_dir: Path, zip_path: Path | None, builder: list[str] | None, version: str = VERSION) -> dict | None:
    with tempfile.TemporaryDirectory(prefix="mzlab-lib-") as tmp:
        work = Path(tmp)
        if zip_path is None:
            zip_path = work / "massbank.zip"
            print(f"downloading MassBank-data {VERSION} (Zenodo {ZENODO_RECORD})")
            try:
                download(ZIP_URL, zip_path, ZIP_SHA256)
            except ConnectionError as e:
                print(f"::warning::spectral-library packages not built: the download failed ({e})")
                return None
        stats = filter_zip(zip_path, work)
        builder = builder or default_builder()
        if out_dir.exists():
            shutil.rmtree(out_dir)
        out_dir.mkdir(parents=True)
        libs = []
        for pol, sign, label in (("pos", 1, "ESI+"), ("neg", -1, "ESI−")):
            s = stats[pol]
            if not s["n"]:
                continue
            print(f"MassBank {label}: {s['n']} records")
            mz = work / f"massbank-{pol}.mzlib"
            build_mzlib(builder, work / f"{pol}.txt", mz)
            libs.append(entry(f"massbank-{pol}", f"MassBank Europe · {label} · MS2", sign, version, "MassBank Europe", "https://massbank.eu",
                              "Per record: CC0, CC BY o CC BY-SA 4.0; il pacchetto nel suo insieme: CC BY-SA 4.0", s["licenze"], s["contributori"], mz, out_dir, ATTRIBUTION))
        n_unito, lic_unito = unito_records(work / "unito.txt")
        if n_unito:
            print(f"UniTO standards: {n_unito} records")
            mz = work / "unito.mzlib"
            build_mzlib(builder, work / "unito.txt", mz)
            libs.append(entry("unito-standard", "Standard dell'Università di Torino (esempio HR)", 0, "1", "Università di Torino", "https://github.com/Freeesta/mzlab",
                              "CC BY 4.0", lic_unito, {"UniTO": n_unito}, mz, out_dir, UNITO_ATTRIBUTION))
        fonti = load_sources()
        bad = check_sources(fonti)
        if bad:
            raise SystemExit("tools/librerie_catalogo.json: " + "; ".join(bad))
        cat = {"formato": CATALOG_FORMAT, "pezzo": PIECE, "librerie": libs, "esterne": external_entries(fonti), "fonti": sources_with_packages(fonti, libs)}
        (out_dir / "catalogo.json").write_text(json.dumps(cat, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
        total = sum(sum(p["dimensione"] for p in lib["pezzi"]) for lib in libs)
        print(f"catalogo.json: {len(libs)} packages, {total / 1e6:.1f} MB in {sum(len(lib['pezzi']) for lib in libs)} pieces")
        return cat


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--check-catalog", action="store_true", help="check tools/librerie_catalogo.json and exit")
    ap.add_argument("--out", type=Path, help="folder that becomes static/librerie/ of the site")
    ap.add_argument("--zip", type=Path, help="an already downloaded MassBank-data zip (not checked against the pinned SHA-256)")
    ap.add_argument("--builder", nargs="+", help="command that builds IN.txt OUT.mzlib (default: the `build` example of crates/mzlab-lib)")
    a = ap.parse_args()
    if a.check_catalog:
        bad = check_sources(load_sources())
        print("\n".join(bad) if bad else "catalogo ok")
        sys.exit(1 if bad else 0)
    if not a.out:
        ap.error("--out is required")
    run(a.out, a.zip, a.builder)


if __name__ == "__main__":
    sys.exit(main())
