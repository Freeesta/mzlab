"""Default XIC window at unit resolution: [n - 0.2, n + 0.3] around the nearest integer n of the m/z, clicked or typed alike.
The rule is `xicWin` of mzlab/web/pannelli.js (with its rounding `rh`); this runs those two lines in node, unchanged."""
import json
import re
import shutil
import subprocess
from pathlib import Path

import pytest

SRC = (Path(__file__).resolve().parent.parent / "mzlab" / "web" / "pannelli.js").read_text(encoding="utf-8")


def _windows(values):
    lines = [re.search(rf"^const {n} = .*$", SRC, re.M).group(0) for n in ("rh", "XIC_BELOW", "xicWin")]
    js = "\n".join(lines) + f"\nconsole.log(JSON.stringify({json.dumps(values)}.map(([v, o]) => xicWin(v, o))));"
    return json.loads(subprocess.run(["node", "-e", js], capture_output=True, text=True, check=True).stdout)


@pytest.mark.skipif(not shutil.which("node"), reason="node not installed")
def test_unit_window_is_centred_on_the_nearest_integer():
    cases = {(264, False): [263.8, 264.3], (364.3, True): [363.8, 364.3], (364.37, True): [363.8, 364.3], (364.6, False): [364.8, 365.3],
             (364.5, False): [364.8, 365.3], (364.49, True): [363.8, 364.3], (100.26, False): [99.8, 100.3], (46.07, False): [45.8, 46.3]}
    got = _windows([list(k) for k in cases])
    assert got == list(cases.values()), got


@pytest.mark.skipif(not shutil.which("node"), reason="node not installed")
def test_clicked_and_typed_give_the_same_window():
    vals = [194.04, 299.9, 363.7, 385.3, 1000.5]
    assert _windows([[v, True] for v in vals]) == _windows([[v, False] for v in vals])
