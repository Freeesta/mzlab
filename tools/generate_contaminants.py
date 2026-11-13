"""Write mzlab/web/contaminanti.json: the answer of /api/contaminants (built-in lists of known contaminants and pesticides expanded into ions), served as a static file
by the flow without Pyodide (web/lr.js). tests/test_contaminanti.py keeps it equal to mzlab.chem.contaminants.lists().

    python3 tools/generate_contaminants.py
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from mzlab.chem.contaminants import lists  # noqa: E402

OUT = ROOT / "mzlab" / "web" / "contaminanti.json"


def text() -> str:
    return json.dumps({"lists": lists()})


if __name__ == "__main__":
    OUT.write_text(text(), encoding="utf-8")
    print(f"{OUT} ({OUT.stat().st_size} bytes)")
