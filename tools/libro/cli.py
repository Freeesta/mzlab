"""Command line: python3 tools/libro/cli.py --src <folder with .md> --out <folder for .html> [--pubblico]. Reads and writes only the two folders given."""
import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent))
from tools.libro.errori import LibroError  # noqa: E402
from tools.libro.pagina import converti  # noqa: E402


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--src", required=True, type=Path)
    ap.add_argument("--out", required=True, type=Path)
    ap.add_argument("--pubblico", action="store_true", help="drop <!-- privato --> blocks and refuse unresolved [I]")
    a = ap.parse_args(argv)
    files = sorted(f for f in a.src.rglob("*.md") if not f.name.endswith((".appunti.md", ".qa.md")) and f.name != "LEGGIMI.md")
    a.out.mkdir(parents=True, exist_ok=True)
    try:
        for f in files:
            (a.out / (f.stem + ".html")).write_text(converti(f.read_text(encoding="utf-8"), f.name, a.pubblico), encoding="utf-8", newline="\n")
    except LibroError as e:
        print(e, file=sys.stderr)
        return 1
    print(f"{len(files)} section(s) -> {a.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
