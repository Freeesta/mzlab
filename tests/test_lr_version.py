"""The version that mzlab/web/lr.js answers in api/state is the one of the package (the flow without Pyodide cannot import it)."""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def test_lr_version_is_the_package_version():
    import mzlab
    js = (ROOT / "mzlab" / "web" / "lr.js").read_text(encoding="utf-8")
    assert re.search(r'const VERSION = "([^"]+)"', js).group(1) == mzlab.__version__
