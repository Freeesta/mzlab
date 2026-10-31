"""Table of bytes for tools/wasm_nightly.sh: reads lines `<module> <variant> <bytes>` on stdin, prints a Markdown table."""
import sys

VARIANTI = ["stable", "nightly", "loc", "locimm"]

dati: dict[str, dict[str, int]] = {}
for riga in sys.stdin:
    nome, variante, byte = riga.split()
    dati.setdefault(nome, {})[variante] = int(byte)
print("| module | " + " | ".join(VARIANTI) + " | locimm vs stable |")
print("|---|" + "---|" * (len(VARIANTI) + 1))
for nome, r in dati.items():
    print(f"| {nome} | " + " | ".join(str(r[v]) for v in VARIANTI) + f" | {r['locimm'] - r['stable']:+d} |")
