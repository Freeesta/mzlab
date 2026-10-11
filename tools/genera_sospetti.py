"""Builds the offline suspect index `.mzsus` for mzFinder (never committed): the CI of Pages runs this after build_site.py.

    python tools/genera_sospetti.py --out sospetti.mzsus                         # downloads the pinned sources (Zenodo)
    python tools/genera_sospetti.py --fonte DIR --out sospetti.mzsus             # already downloaded copies (a missing file is downloaded into DIR)
    python tools/genera_sospetti.py --fonte DIR --pacchetto site/static/mzfinder/sospetti    # also the zlib-compressed pieces + catalogo.json

Sources (all CC BY 4.0): NORMAN SusDat, PubChemLite for Exposomics, Transformations in PubChem. Versions and SHA-256 are pinned below: a different file stops
the build. The index is a list of suspects by monoisotopic mass (never a name for a feature); the format is specified in docs/agenti/sospetti.md.
`leggi_sospetti()` is the reference reader (the Rust reader is crates/mzlab-feat/src/suspects.rs).
"""
from __future__ import annotations

import argparse
import array
import bisect
import csv
import hashlib
import json
import struct
import sys
import tempfile
import zlib
from pathlib import Path
from typing import NamedTuple

sys.path.insert(0, str(Path(__file__).resolve().parent))
from librerie_pacchetti import PIECE, download, sha256_file  # noqa: E402

FORMAT = "mzsus/1"
CATALOG_FORMAT = "sospetti/1"
VERSION = 1
MAGIC = b"MZSU"
HEAD = struct.Struct("<4sHHIIIIII")          # magic, version, reader_min, n_rows, n_formulas, n_reactions, manifest_len, flags, reserved
SOURCE_BITS = {"susdat": 1, "pubchemlite": 2, "transpubchem": 4}
NAMES = ("susdat", "pubchemlite", "transpubchem")


def _zen(record: str, name: str) -> str:
    return f"https://zenodo.org/api/records/{record}/files/{name}/content"


# Pinned sources. `files` = [(name, sha256)]; the first source of a key wins for mass and formula (SusDat, PubChemLite, Transformations).
SOURCES = [
    {"id": "susdat", "nome": "NORMAN SusDat (Merged NORMAN Suspect List)", "versione": "NORMAN-SLE-S0.0.6.1 (03/06/2025)", "licenza": "CC-BY-4.0",
     "doi": "10.5281/zenodo.15741861", "url": "https://doi.org/10.5281/zenodo.15741861", "istituzione": "NORMAN Network",
     "files": [("susdat_2025-06-03-092022.csv", "8b8c4b43e900b752c2e7ba0aa7adb3454617205034f9da5baffdf41250decda9", _zen("15741861", "susdat_2025-06-03-092022.csv"))]},
    {"id": "pubchemlite", "nome": "PubChemLite for Exposomics", "versione": "3.3.0 (25/09/2026)", "licenza": "CC-BY-4.0",
     "doi": "10.5281/zenodo.22953038", "url": "https://doi.org/10.5281/zenodo.22953038", "istituzione": "Università del Lussemburgo (LCSB)",
     "files": [("PubChemLite_exposomics_20260925.csv", "6a258459e1ccf29a5d08e82fcf5240a2fc8afbabe5ba9c678e39aa0801dd89cd", _zen("22953038", "PubChemLite_exposomics_20260925.csv"))]},
    {"id": "transpubchem", "nome": "Transformations in PubChem", "versione": "PubChemTrans-0.2.3 (21/03/2026)", "licenza": "CC-BY-4.0",
     "doi": "10.5281/zenodo.19150892", "url": "https://doi.org/10.5281/zenodo.19150892", "istituzione": "Università del Lussemburgo (LCSB), NCBI/NLM/NIH",
     "files": [("PubChem_all_transformations.csv", "42817e82e095890ba39287c2548b63a2d11c98e4947efc51f97b8b4928b99f3d", _zen("19150892", "PubChem_all_transformations.csv")),
               ("Transformations_CID_Info_all.csv", "9b49ee33eb01bf59317faf3c7182ff1aa68aaa03b12baee15d3683194d721431", _zen("19150892", "Transformations_CID_Info_all.csv"))]},
]

csv.field_size_limit(1 << 30)


def _rows(path: Path):
    with open(path, newline="", encoding="utf-8", errors="replace") as f:
        yield from csv.DictReader(f)


def _num(s: str | None) -> float | None:
    try:
        v = float(s)                                     # type: ignore[arg-type]
    except (TypeError, ValueError):
        return None
    return v if 0 < v < 1e5 else None


def _int(s: str | None) -> int:
    try:
        return max(0, int(float(s)))                     # type: ignore[arg-type]
    except (TypeError, ValueError):
        return 0


def read_susdat(p: Path):
    for r in _rows(p):
        yield (r.get("StdInChIKey") or "")[:14], _num(r.get("Monoiso_Mass")), (r.get("Molecular_Formula") or "").strip(), 0, 0, 0


def read_pubchemlite(p: Path):
    for r in _rows(p):
        yield ((r.get("FirstBlock") or "")[:14], _num(r.get("MonoisotopicMass")), (r.get("MolecularFormula") or "").strip(),
               _int(r.get("PubMed_Count")), _int(r.get("Patent_Count")), 0)


def read_transpubchem(react: Path, info: Path):
    """One entry per compound that takes part in a reaction; the reaction id is the 1-based row of its first reaction in the reactions file."""
    cid = {r["CID"]: ((r.get("IKFB") or r.get("InChIKey") or "")[:14], _num(r.get("ExactMass")), (r.get("MolecularFormula") or "").strip()) for r in _rows(info)}
    first: dict[str, int] = {}
    for i, r in enumerate(_rows(react), 1):
        for k in ("predecessorcid", "successorcid"):
            first.setdefault((r.get(k) or "").strip(), i)
    for c, i in first.items():
        if c in cid:
            yield (*cid[c], 0, 0, i)


def merge(streams: list[tuple[str, object]]) -> tuple[list[tuple], dict]:
    """Rows keyed by InChIKey-14 (A-Z only), the sources united; sorted by (mass, key). Rows without a valid key or mass are dropped and counted."""
    acc: dict[str, list] = {}
    stats = {n: {"letti": 0, "usati": 0} for n, _ in streams}
    for name, it in streams:
        bit = SOURCE_BITS[name]
        for key, mass, formula, lit, pat, react in it:
            stats[name]["letti"] += 1
            if len(key) != 14 or not key.isalpha() or not key.isupper() or mass is None or not formula:
                continue
            stats[name]["usati"] += 1
            a = acc.get(key)
            if a is None:
                acc[key] = [mass, formula, bit, min(lit, 65535), min(pat, 65535), react]
                continue
            a[2] |= bit
            a[3], a[4] = max(a[3], min(lit, 65535)), max(a[4], min(pat, 65535))
            if react and (not a[5] or react < a[5]):
                a[5] = react
    rows = sorted(((v[0], k, v[1], v[2], v[3], v[4], v[5]) for k, v in acc.items()), key=lambda t: (t[0], t[1]))
    return rows, stats


def _pad8(b: bytes) -> bytes:
    return b + b"\0" * (-len(b) % 8)


def write_mzsus(rows: list[tuple], manifest: dict) -> bytes:
    """Layout (little-endian, every section starts at a multiple of 8): header, manifest JSON, then the columns; see docs/agenti/sospetti.md."""
    formulas = sorted({r[2] for r in rows})
    fidx = {f: i for i, f in enumerate(formulas)}
    react = [(i, r[6]) for i, r in enumerate(rows) if r[6]]
    man = json.dumps(manifest, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")
    blob = [f.encode("ascii", "replace") for f in formulas]
    offs = [0]
    for b in blob:
        offs.append(offs[-1] + len(b))
    n = len(rows)
    out = [_pad8(HEAD.pack(MAGIC, VERSION, 1, n, len(formulas), len(react), len(man), 0, 0)), _pad8(man),
           _pad8(struct.pack(f"<{n}d", *[r[0] for r in rows])),
           _pad8(b"".join(r[1].encode("ascii") for r in rows)),
           _pad8(struct.pack(f"<{n}I", *[fidx[r[2]] for r in rows])),
           _pad8(bytes(r[3] for r in rows)),
           _pad8(struct.pack(f"<{n}H", *[r[4] for r in rows])),
           _pad8(struct.pack(f"<{n}H", *[r[5] for r in rows])),
           _pad8(b"".join(struct.pack("<II", i, rid) for i, rid in react)),
           _pad8(struct.pack(f"<{len(offs)}I", *offs)), _pad8(b"".join(blob))]
    return b"".join(out)


class Hit(NamedTuple):
    mass: float
    formula: str
    inchikey14: str
    sources: tuple
    pubmed: int
    patents: int
    reaction: int          # 0 = none; else the 1-based row in the reactions file of Transformations in PubChem


class Suspects:
    """Reference reader of a `.mzsus` (the whole file in memory)."""

    def __init__(self, data: bytes):
        if len(data) < HEAD.size or data[:4] != MAGIC:
            raise ValueError("not a .mzsus file")
        _, ver, need, n, nf, nr, nman, _, _ = HEAD.unpack_from(data)
        if need > VERSION:
            raise ValueError(f".mzsus needs reader {need}, this one is {VERSION}")
        self.version, self.n = ver, n
        p = HEAD.size

        def take(size: int) -> bytes:
            nonlocal p
            b = data[p:p + size]
            if len(b) != size:
                raise ValueError(".mzsus truncated")
            p += size + (-size % 8)
            return b

        self.manifest = json.loads(take(nman).decode("utf-8"))
        self.masses = self._arr("d", take(8 * n))
        self.keys = take(14 * n)
        self.fidx = self._arr("I", take(4 * n))
        self.src = take(n)
        self.lit = self._arr("H", take(2 * n))
        self.pat = self._arr("H", take(2 * n))
        rb = take(8 * nr)
        self.react = {struct.unpack_from("<II", rb, 8 * i)[0]: struct.unpack_from("<II", rb, 8 * i)[1] for i in range(nr)}
        offs = self._arr("I", take(4 * (nf + 1)))
        blob = take(offs[-1])
        self.formulas = [blob[offs[i]:offs[i + 1]].decode("ascii") for i in range(nf)]

    @staticmethod
    def _arr(code: str, b: bytes):
        a = array.array(code)
        a.frombytes(b)
        if sys.byteorder == "big":
            a.byteswap()
        return a

    def hit(self, i: int) -> Hit:
        s = self.src[i]
        return Hit(self.masses[i], self.formulas[self.fidx[i]], self.keys[14 * i:14 * i + 14].decode("ascii"),
                   tuple(n for n in NAMES if s & SOURCE_BITS[n]), self.lit[i], self.pat[i], self.react.get(i, 0))

    def cerca(self, mass: float, ppm: float) -> list[Hit]:
        """Hits with |m - mass| <= mass * ppm * 1e-6, ordered by mass then InChIKey-14."""
        d = mass * ppm * 1e-6
        lo = bisect.bisect_left(self.masses, mass - d)
        hi = bisect.bisect_right(self.masses, mass + d)
        return [self.hit(i) for i in range(lo, hi)]


def leggi_sospetti(src) -> Suspects:
    """`src`: bytes or a path of a raw `.mzsus`."""
    return Suspects(src if isinstance(src, (bytes, bytearray)) else Path(src).read_bytes())


def prepara_fonti(fonti: list[dict], folder: Path, scarica: bool = True) -> dict[str, list[Path]]:
    """Every file of every source in `folder`, SHA-256 checked (a different file stops the build; a missing one is downloaded)."""
    folder.mkdir(parents=True, exist_ok=True)
    out: dict[str, list[Path]] = {}
    for s in fonti:
        out[s["id"]] = []
        for name, want, url in s["files"]:
            p = folder / name
            if p.is_file():
                got = sha256_file(p)
                if got != want:
                    raise SystemExit(f"{name}: SHA-256 {got} is not the expected {want}: build stopped")
            elif scarica:
                print(f"downloading {name}")
                download(url, p, want)
            else:
                raise SystemExit(f"{name}: missing in {folder}")
            out[s["id"]].append(p)
    return out


def genera(fonti: list[dict], folder: Path, scarica: bool = True) -> bytes:
    files = prepara_fonti(fonti, folder, scarica)
    streams = []
    for s in fonti:
        p = files[s["id"]]
        it = {"susdat": lambda: read_susdat(p[0]), "pubchemlite": lambda: read_pubchemlite(p[0]), "transpubchem": lambda: read_transpubchem(p[0], p[1])}[s["id"]]()
        streams.append((s["id"], it))
    rows, stats = merge(streams)
    man = {"formato": FORMAT, "righe": len(rows),
           "attribuzione": "Indice dei sospetti costruito da NORMAN SusDat (NORMAN Network), PubChemLite for Exposomics e Transformations in PubChem "
                           "(Università del Lussemburgo e NCBI/NLM/NIH), tutti CC BY 4.0; i DOI sono nelle fonti.",
           "fonti": [{"id": s["id"], "nome": s["nome"], "versione": s["versione"], "licenza": s["licenza"], "doi": s["doi"], "url": s["url"],
                      "istituzione": s["istituzione"], "bit": SOURCE_BITS[s["id"]], "sha256": {n: h for n, h, _ in s["files"]}, **stats[s["id"]]} for s in fonti]}
    return write_mzsus(rows, man)


def pacchetto(raw: bytes, out_dir: Path, fonti: list[dict] | None = None) -> dict:
    """zlib (level 9) of the raw file cut into pieces of PIECE bytes with a SHA-256 each + catalogo.json (the browser inflates with DecompressionStream)."""
    z = zlib.compress(raw, 9)
    out_dir.mkdir(parents=True, exist_ok=True)
    pieces = []
    for i in range(0, len(z), PIECE):
        b = z[i:i + PIECE]
        name = f"{i // PIECE:03d}.bin"
        (out_dir / name).write_bytes(b)
        pieces.append({"file": name, "dimensione": len(b), "sha256": hashlib.sha256(b).hexdigest()})
    man = leggi_sospetti(raw).manifest
    cat = {"formato": CATALOG_FORMAT, "pezzo": PIECE, "compressione": "zlib", "dimensione_compressa": len(z), "dimensione": len(raw),
           "sha256": hashlib.sha256(raw).hexdigest(), "righe": man["righe"], "fonti": man["fonti"], "attribuzione": man["attribuzione"], "pezzi": pieces}
    (out_dir / "catalogo.json").write_text(json.dumps(cat, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    return cat


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--fonte", type=Path, help="folder with the source files (downloaded there if missing); default: a temporary folder")
    ap.add_argument("--out", type=Path, help="raw .mzsus file")
    ap.add_argument("--pacchetto", type=Path, help="folder for the compressed pieces and catalogo.json")
    a = ap.parse_args()
    if not a.out and not a.pacchetto:
        ap.error("give --out and/or --pacchetto")
    with tempfile.TemporaryDirectory(prefix="mzlab-sus-") as tmp:
        try:
            raw = genera(SOURCES, a.fonte or Path(tmp))
        except ConnectionError as e:
            print(f"::warning::suspect index not built: the download failed ({e})")
            return
    if a.out:
        a.out.write_bytes(raw)
    print(f".mzsus: {len(raw) / 1e6:.1f} MB raw, {leggi_sospetti(raw).n} rows")
    if a.pacchetto:
        cat = pacchetto(raw, a.pacchetto)
        print(f"package: {cat['dimensione_compressa'] / 1e6:.1f} MB compressed in {len(cat['pezzi'])} pieces")


if __name__ == "__main__":
    main()
