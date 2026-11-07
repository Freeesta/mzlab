"""The pools of mzFinder (TP_Mine/files: the index builders of finder-idx.js and the feature workers of tpmine-worker.js) take the size of the device into
account like the pool of the engine (rust-worker.js): at most 2 workers and a 32 MB builder budget on a tablet or a phone."""
import json
import re
import shutil
import subprocess
from pathlib import Path

import pytest

FILES = Path(__file__).resolve().parent.parent / "TP_Mine" / "files"


@pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")
def test_a_tablet_or_a_phone_gets_two_index_builders_of_32_mb():
    body = f"""
import {{ indexPool, smallDevice }} from {(FILES / 'finder-idx.js').as_uri()!r};
import {{ poolSize }} from {(FILES.parent.parent / 'mzlab' / 'web' / 'idxpool.js').as_uri()!r};
const ipad = {{ userAgent: "Mozilla/5.0 (iPad; CPU OS 17_0 like Mac OS X)", hardwareConcurrency: 8 }};
const phone = {{ userAgent: "Mozilla/5.0 (Linux; Android 14)", hardwareConcurrency: 8 }};
const mac = {{ userAgent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0)", platform: "MacIntel", maxTouchPoints: 0, hardwareConcurrency: 8 }};
const ipadDesktop = {{ ...mac, maxTouchPoints: 5 }};
console.log(JSON.stringify([indexPool(ipad, poolSize), indexPool(phone, poolSize), indexPool(mac, poolSize), smallDevice(ipadDesktop), smallDevice({{}})]));"""
    r = subprocess.run(["node", "--input-type=module", "-e", body], capture_output=True, encoding="utf-8")
    assert r.returncode == 0, r.stderr
    assert json.loads(r.stdout) == [{"size": 2, "budget": 32}, {"size": 2, "budget": 32}, {"size": 4, "budget": 64}, True, False]


def test_the_feature_workers_and_the_index_builders_use_the_size_of_the_device():
    worker = (FILES / "tpmine-worker.js").read_text(encoding="utf-8")
    ingress = (FILES / "finder-idx.js").read_text(encoding="utf-8")
    assert "poolSize(false" not in worker and "poolSize(false" not in ingress
    assert re.search(r"createFeatPool\(\{[^}]*size: mod\.poolSize\(smallDevice\(navigator\)", worker)
    assert re.search(r"createPool\(\{[^\n]*\.\.\.indexPool\(navigator, pools\.poolSize\)", ingress)
