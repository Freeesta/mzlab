"""Weight of the start-up scripts: the list `L` of mzlab/web/carica.js is fetched whole on every first visit, so no file in it
may grow past a cap and the total is capped too. A bigger tool that is used rarely goes behind `carica(name)` (lazy) instead."""
import re
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
MAX_FILE = 100_000      # bytes, uncompressed, one script of the list
MAX_TOTAL = 1_550_000   # bytes, uncompressed, the whole list


def startup_files():
    src = (WEB / "carica.js").read_text(encoding="utf-8")
    body = re.search(r"var L = \[(.*?)\];", src, re.S).group(1)
    return [n[2:] if n.startswith("m:") else n for n in re.findall(r'"([^"]+)"', body)]


def test_every_listed_script_exists():
    assert [n for n in startup_files() if not (WEB / n).is_file()] == []


def test_no_startup_script_is_too_big():
    big = {n: (WEB / n).stat().st_size for n in startup_files() if (WEB / n).stat().st_size > MAX_FILE}
    assert not big, f"start-up scripts over {MAX_FILE} bytes (make them lazy with carica()): {big}"


def test_startup_total_is_capped():
    total = sum((WEB / n).stat().st_size for n in startup_files())
    assert total <= MAX_TOTAL, f"start-up scripts weigh {total} bytes (cap {MAX_TOTAL})"
