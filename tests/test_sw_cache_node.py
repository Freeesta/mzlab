"""mzlab/web/sw.js with a mocked scope: an app file revalidated with the same ETag is not rewritten into Cache Storage; the first-paint scripts are preloaded."""
import re
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent


@pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")
def test_sw_skips_put_for_unchanged_resource():
    r = subprocess.run(["node", str(ROOT / "tests" / "sw_cache_node.mjs")], capture_output=True, text=True, check=False)
    assert r.returncode == 0, (r.stdout + r.stderr)[-2000:]


def test_first_paint_scripts_are_preloaded():
    web = ROOT / "mzlab" / "web"
    names = re.search(r'\[((?:"[\w-]+",?\s*)+)\]\.forEach', (web / "first-paint.js").read_text()).group(1)
    html = (web / "index.html").read_text()
    for n in re.findall(r'"([\w-]+)"', names):
        assert f'<link rel="preload" as="script" href="static/{n}.js">' in html, n
