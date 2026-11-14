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


def run(provider, proxy, *urls, fail_registry=0, race=""):
    out = subprocess.run(["node", str(ROOT / "tests/trasporto_node.js"), str(ROOT / "mzlab/web/llm"), provider, proxy, *urls],
                         check=True, capture_output=True, encoding="utf-8", env={**os.environ, "FAIL_REGISTRY": str(fail_registry), "RACE": race}).stdout
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


def test_a_second_init_never_sends_the_previous_providers_key_to_the_new_origin():
    """init A, the key of A is set; init B arrives and its recall is still pending: a send to B's origin must not carry A's key."""
    a, b = run("anthropic", "", "https://api.openai.com/v1/chat/completions", "https://api.anthropic.com/v1/messages", race="openai")
    assert a["fetched"] == [] and a["msgs"] == ['fail:err.llm.auth:{"reason":"nokey"}'], a
    # A's origin is no longer this provider's: nothing leaves either
    assert b["fetched"] == [], b


def race(scenario):
    out = subprocess.run(["node", str(ROOT / "tests/trasporto_race_node.js"), str(ROOT / "mzlab/web/llm"), scenario], check=True, capture_output=True, encoding="utf-8").stdout
    return json.loads(out)


def test_the_recall_of_an_earlier_init_never_lands_on_a_later_one():
    """init A, init B (done), then the recall of A answers: B keeps its own key, and with no key stored B has none (A's key is not adopted)."""
    (f,) = race("init-order")["fetched"]
    assert f["key"] == "key-of-openai", f
    r = race("init-then-recall")
    assert r["fetched"] == [] and r["msgs"] == ['fail:err.llm.auth:{"reason":"nokey"}'], r


def test_a_send_whose_provider_changed_while_it_waited_sends_nothing():
    """a send of provider A waits for the proxy host; init B lands; B's key must not go to A's origin."""
    r = race("send-vs-init")
    assert r["fetched"] == [] and r["msgs"] == ['fail:err.llm.network:{"reason":"url"}'], r


def test_a_remembered_key_is_stored_under_the_provider_of_the_click():
    """init B lands while the key of A is being encrypted: it is stored for A, never for B."""
    stored = race("remember-vs-init")["stored"]
    assert stored.get("k:anthropic") == "typed-key" and "k:openai" not in stored, stored
