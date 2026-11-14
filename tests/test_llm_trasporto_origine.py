"""The transport iframe (mzlab/web/llm/trasporto.js, the only holder of the site's API key) attaches the key only to requests for the origin of its
provider, taken from the registry (`fornitori-1.json`) and `proxy-host.js`, never from the page's request. Run by node with a stub DOM and fetch."""
import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def run(provider, proxy, *urls, fail_registry=0):
    out = subprocess.run(["node", str(ROOT / "tests/trasporto_node.js"), str(ROOT / "mzlab/web/llm"), provider, proxy, *urls],
                         check=True, capture_output=True, encoding="utf-8", env={**os.environ, "FAIL_REGISTRY": str(fail_registry)}).stdout
    return json.loads(out)


def test_a_send_to_another_origin_is_refused_and_the_key_is_not_attached():
    bad = ["https://evil.org/v1/messages", "https://api.anthropic.com.evil.org/v1/messages", "https://api.anthropic.com@evil.org/", "http://api.anthropic.com/x", "file:///etc/passwd"]
    for r in run("anthropic", "", *bad, "https://api.anthropic.com/v1/messages"):
        if "evil" in r["url"] or r["url"].startswith(("http://", "file")):
            assert r["fetched"] == [], r
            assert r["msgs"] == ['fail:err.llm.network:{"reason":"url"}'], r
        else:
            (f,) = r["fetched"]
            assert ["x-api-key", "sk-SECRET"] in f["headers"], f


def test_the_demo_provider_reaches_only_the_proxy_host():
    host = "https://assist.example.org"
    got = {r["url"]: r for r in run("mzlab-demo", host, host + "/v1/messages", "https://evil.org/v1/messages", "https://proxy.invalid/v1/messages")}
    assert len(got[host + "/v1/messages"]["fetched"]) == 1
    assert got["https://evil.org/v1/messages"]["fetched"] == []
    # the placeholder's origin is the provider's `base_url` (an invalid host, no key can reach anyone); Rust has already replaced it by the proxy host when a send arrives
    assert len(got["https://proxy.invalid/v1/messages"]["fetched"]) == 1
    # no host yet: nothing leaves
    assert run("mzlab-demo", "", host + "/v1/messages")[0]["fetched"] == []


def test_the_site_serves_the_registry_beside_the_transport():
    text = (ROOT / "tools/build_site.py").read_text(encoding="utf-8")
    assert 'static / "llm" / "fornitori-1.json"' in text


def test_a_redirect_is_never_followed_and_a_failed_registry_fetch_is_retried():
    (r,) = run("anthropic", "", "https://api.anthropic.com/v1/messages")
    assert r["fetched"][0]["redirect"] == "error", r
    # the first fetch of the registry fails: that send fails closed, the next one asks again and goes through
    a, b = run("anthropic", "", "https://api.anthropic.com/v1/messages", "https://api.anthropic.com/v1/messages", fail_registry=1)
    assert a["fetched"] == [] and a["msgs"] == ['fail:err.llm.network:{"reason":"url"}'], a
    assert len(b["fetched"]) == 1, b
