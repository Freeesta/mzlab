"""Checks `INDICE.tsv` of the data repository (docs/studi/ROSINA.md D) and writes `INDICE.md` from it. No data inside.
    python3 tools/rosina/indice.py INDICE.tsv [--md INDICE.md] [--controlla]
Rules: the columns of D, the closed vocabulary of `esperimenti` (separated by `,` or `;`), `ridistribuibile` in si/no/da-verificare, `verita` in si/no, a licence for every
row that is `si`, a `produttore` of the list, no empty `percorso`, no repeated `percorso`. Exit 1 with one line per problem."""
import argparse
import csv
import sys
from pathlib import Path

COLONNE = ["percorso", "produttore", "modello_nel_file", "versione_formato", "esperimenti", "fonte", "accession", "doi", "licenza", "ridistribuibile", "byte", "sha256", "url", "riferimento", "verita", "note"]
PRODUTTORI = {"thermo", "sciex", "agilent", "waters", "bruker", "shimadzu", "leco", "aperti"}
ESPERIMENTI = {
    "fullscan", "sim", "mrm", "smrm", "ms2", "dda", "dia", "mse", "prm", "pis", "nl", "epi", "ms3", "ims", "uv", "analogico",
    "polarita-mista", "lockmass", "profilo", "centroidi", "gc", "imaging",
}
RIDISTRIBUIBILE = {"si", "no", "da-verificare"}
SI_NO = {"si", "no"}
NESSUNA_LICENZA = {"", "-", "nessuna", "non dichiarata"}


def leggi(p: Path) -> list:
    with p.open(newline="", encoding="utf-8") as f:
        return list(csv.DictReader(f, delimiter="\t"))


def controlla(righe: list, colonne: list) -> list:
    problemi = [f"missing column: {c}" for c in COLONNE if c not in colonne]
    visti = set()
    for i, r in enumerate(righe, start=2):
        dove = f"row {i} ({r.get('percorso', '')})"
        p = (r.get("percorso") or "").strip()
        if not p:
            problemi.append(f"{dove}: empty percorso")
        elif p in visti:
            problemi.append(f"{dove}: repeated percorso")
        visti.add(p)
        if (r.get("produttore") or "") not in PRODUTTORI:
            problemi.append(f"{dove}: produttore not in the list")
        for e in (x.strip() for x in (r.get("esperimenti") or "").replace(";", ",").split(",") if x.strip()):
            if e not in ESPERIMENTI:
                problemi.append(f"{dove}: esperimenti outside the vocabulary: {e}")
        rid = r.get("ridistribuibile") or ""
        if rid not in RIDISTRIBUIBILE:
            problemi.append(f"{dove}: ridistribuibile must be si, no or da-verificare")
        if rid == "si" and (r.get("licenza") or "").strip().lower() in NESSUNA_LICENZA:
            problemi.append(f"{dove}: ridistribuibile=si without a licence")
        if (r.get("verita") or "") not in SI_NO:
            problemi.append(f"{dove}: verita must be si or no")
        b = (r.get("byte") or "").strip()
        if b and not b.isdigit():
            problemi.append(f"{dove}: byte is not a number")
        s = (r.get("sha256") or "").strip()
        if s and len(s) != 64:
            problemi.append(f"{dove}: sha256 is not 64 characters")
    return problemi


def markdown(righe: list) -> str:
    out = ["# Indice dei formati (generato da INDICE.tsv: non modificare a mano)", ""]
    for prod in sorted({r["produttore"] for r in righe}):
        out += [f"## {prod}", "", "| percorso | modello nel file | esperimenti | accession | licenza | ridistribuibile | riferimento | verità |", "|---|---|---|---|---|---|---|---|"]
        for r in sorted((r for r in righe if r["produttore"] == prod), key=lambda r: r["percorso"]):
            cel = [r.get(k, "").replace("|", "/") for k in ("percorso", "modello_nel_file", "esperimenti", "accession", "licenza", "ridistribuibile", "riferimento", "verita")]
            out.append("| " + " | ".join(cel) + " |")
        out.append("")
    return "\n".join(out)


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("tsv", type=Path)
    ap.add_argument("--md", type=Path, help="where to write INDICE.md")
    ap.add_argument("--controlla", action="store_true", help="only check (do not write)")
    a = ap.parse_args(argv)
    with a.tsv.open(newline="", encoding="utf-8") as f:
        lettore = csv.DictReader(f, delimiter="\t")
        righe, colonne = list(lettore), lettore.fieldnames or []
    problemi = controlla(righe, colonne)
    for p in problemi:
        print(p, file=sys.stderr)
    if problemi:
        return 1
    if a.md and not a.controlla:
        a.md.write_text(markdown(righe), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
