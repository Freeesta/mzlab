"""Rust identifiers are English; the Italian words renamed so far must not come back in any tracked .rs file or in the agent docs.
Each rename slice adds its words to OLD_WORDS (whole snake_case / CamelCase segments)."""
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OLD_WORDS = ["apri", "scrivi", "chiave", "lunghezza", "percorso", "nodo", "nome", "dati", "membri"]
# not identifiers: the file-name family (tools/dati_*.rs), repository paths (`tests/dati/`, `dati/HRMS/`), the private repo name and JSON keys named in comments
KEEP = re.compile(r"mzlab-dati|scansione_dati|(?<=[a-z\]])\.nome\b|dati_percorsi|dati_sintetici|[A-Za-z0-9_.-]*/dati(?=/)|(?<![A-Za-z0-9_])dati/")
OLD = re.compile(r"(?<![A-Za-z0-9])(" + "|".join(OLD_WORDS) + r")(?![a-z0-9])")
SKIP = {"tests/test_no_old_rust_names.py"}


STRING = re.compile(r'r(#*)"(?s:.*?)"\1|"(?:\\.|[^"\\])*"')
DOC_API = re.compile(r"::(" + "|".join(OLD_WORDS) + r")(?![a-z0-9])|(" + "|".join(OLD_WORDS) + r")_(solo|indicizzato)")


def test_old_rust_names_are_gone():
    files = subprocess.run(["git", "ls-files", "*.rs", "docs/agenti/*.md"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.split("\n")
    hits = []
    for f in files:
        if not f or f in SKIP:
            continue
        for n, line in enumerate((ROOT / f).read_text(encoding="utf8").splitlines(), 1):
            if f.endswith(".rs"):
                # string contents (JSON keys, labels) follow their own families
                if OLD.search(KEEP.sub("", STRING.sub('""', line))):
                    hits.append(f"{f}:{n}")
            elif DOC_API.search(line):
                hits.append(f"{f}:{n}")
    assert not hits, hits[:20]
