"""Standalone lazy scripts are loaded without numeri.js, so they must not call its globals (maxOf, minOf, ...)."""
import re
from pathlib import Path

WEB = Path(__file__).resolve().parents[1] / "mzlab" / "web"
LAZY_ALONE = ["rapporto-flow.js"]
NUMERI = re.compile(r"\b(maxOf|minOf|maxAbs|lowerIdx|spanOf)\(")


def test_lazy_alone_scripts_do_not_use_numeri():
    for name in LAZY_ALONE:
        assert not NUMERI.search((WEB / name).read_text(encoding="utf-8")), name
