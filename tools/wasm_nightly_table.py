"""Table of bytes for tools/wasm_nightly.sh: reads lines `<module> <variant> <bytes>` on stdin, prints a Markdown table."""
import sys

VARIANTS = ["stable", "nightly", "loc", "locimm"]

data: dict[str, dict[str, int]] = {}
for riga in sys.stdin:
    nome, variante, byte = riga.split()
    data.setdefault(nome, {})[variante] = int(byte)
print("| module | " + " | ".join(VARIANTS) + " | locimm vs stable |")
print("|---|" + "---|" * (len(VARIANTS) + 1))
for nome, r in data.items():
    print(f"| {nome} | " + " | ".join(str(r[v]) for v in VARIANTS) + f" | {r['locimm'] - r['stable']:+d} |")
