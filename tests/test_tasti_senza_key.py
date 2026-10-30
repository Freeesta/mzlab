"""Key events from autofill or synthetic dispatch can have `key` undefined: handlers must read it as (e.key || "")."""
import re
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
UNSAFE = re.compile(r"\b(?:e|ev|evt|event)\.key\.(?:toLowerCase|toUpperCase|length|startsWith|endsWith)\b")


def test_no_unguarded_key_reads():
    bad = []
    for f in WEB.rglob("*.js"):
        if "vendor" in f.parts or "teoria" in f.parts and f.name.endswith("-dati.js"):
            continue
        for n, line in enumerate(f.read_text(encoding="utf-8", errors="ignore").splitlines(), 1):
            if UNSAFE.search(line):
                bad.append(f"{f.relative_to(WEB)}:{n}")
    assert not bad, "use (e.key || \"\") before string methods: " + ", ".join(bad)
