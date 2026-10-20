"""Teoria editor page (mzlab/web/libro/teoria.html): its page CSP, and the pull-request flow of pr.js against a fake GitHub (tests/teoria_editor_node.mjs):
one branch, the source and its page, one pull request with auto-merge; nothing written when an editor pull request is open or main moved on the chapter."""
import json
import re
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
WEB = ROOT / "mzlab" / "web"
NODE = shutil.which("node")


def test_page_csp_is_strict_and_allows_only_the_converter_wasm():
    html = (WEB / "libro" / "teoria.html").read_text(encoding="utf-8")
    csp = re.search(r'Content-Security-Policy" content="([^"]+)"', html).group(1)
    d = dict(x.strip().split(" ", 1) for x in csp.split(";") if x.strip())
    assert d["default-src"] == "'none'"
    assert d["script-src"] == "'self' 'wasm-unsafe-eval'", "no inline script, no eval; wasm only for the converter"
    assert d["connect-src"] == "'self' https://api.github.com"
    assert "noindex" in html and not re.search(r"<script(?![^>]*\bsrc=)[^>]*>", html)
    frame = re.search(r"<iframe[^>]*>", html).group(0)
    assert 'sandbox="allow-same-origin"' in frame and "allow-scripts" not in html, "the page of the preview runs no script; the same origin only lets the editor reach its blocks"


@pytest.fixture(scope="module")
def flow():
    if not NODE:
        pytest.skip("node not available")
    out = subprocess.run([NODE, "tests/teoria_editor_node.mjs"], cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout
    return json.loads(out)


def test_chapters_are_the_numbered_sources(flow):
    assert flow["chapters"] == ["00-uso.md", "12-dati.md"]
    assert flow["load"]["rawOk"] is True


def test_submit_makes_one_branch_two_files_one_pull_request(flow):
    writes = [(c["m"], c["p"]) for c in flow["ok"]["calls"]]
    assert writes == [("POST", "/git/refs"), ("PUT", "/contents/mzlab/web/teoria/src/12-dati.md"), ("PUT", "/contents/mzlab/web/teoria/12-dati.html"), ("POST", "/pulls"), ("POST", "/graphql")]
    r = flow["ok"]
    assert r["r"] == {"number": 7, "url": "https://github.com/Freeesta/mzlab/pull/7", "branch": "libro/teoria-12-dati-202610201405", "autoMerge": True}
    puts = [c["b"] for c in r["calls"] if c["m"] == "PUT"]
    assert [p["sha"] for p in puts] == ["m1", "h1"] and all(p["branch"] == r["r"]["branch"] for p in puts), "blobs read from main, written on the new branch only"
    assert "enablePullRequestAutoMerge" in r["calls"][-1]["b"]["query"] and "SQUASH" in r["calls"][-1]["b"]["query"]


def test_nothing_is_written_when_a_pull_request_is_open_or_main_moved(flow):
    assert flow["openBlocks"].startswith("conflict: Pull request #5") and flow["openWrites"] == 0
    assert flow["stale"] == "conflict" and flow["staleWrites"] == 0


def test_a_refused_auto_merge_is_reported_not_hidden(flow):
    assert flow["noAuto"] is False


def test_headings_guard_sees_levels_one_to_three_only(flow):
    assert flow["headings"] == ["# A", "## B {#b}", "### C"]


@pytest.fixture(scope="module")
def storia():
    if not NODE:
        pytest.skip("node not available")
    return json.loads(subprocess.run([NODE, "tests/storia_node.mjs"], cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout)


def test_history_walks_back_and_forth_and_drops_redo_after_a_new_step(storia):
    assert storia["same"] is False
    assert storia["undo"] == ["b", "a", None] and storia["redo"] == ["b", "c", None]
    assert storia["branch"] == {"canRedo": False, "now": "x", "back": "b"}
    assert storia["limit"] == ["3", "2", None], "only the last states are kept"


def test_draft_is_kept_per_chapter_and_never_holds_the_token(storia):
    assert storia["saved"] is True and "github_pat_" not in storia["raw"] and "token" not in storia["raw"]
    assert storia["loaded"] == {"text": "testo", "base": "sha1", "summary": "s", "saved": "2026-10-20T10:00:00Z"}
    assert storia["other"] is None and storia["bad"] is None and storia["cleared"] is None


def test_a_blocked_storage_does_not_break_the_editor(storia):
    assert storia["blocked"] == [False, None]


def test_a_retitled_section_sends_the_rebuilt_search_index_as_a_third_file(flow):
    puts = flow["withIndex"]
    assert [p["p"] for p in puts] == ["mzlab/web/teoria/src/12-dati.md", "mzlab/web/teoria/12-dati.html", "mzlab/web/teoria/indice-ricerca.js"]
    assert puts[2]["content"] == "NEW INDEX" and puts[2]["sha"] == "x1" and puts[2]["message"].endswith("(search index)")


def test_nothing_is_written_when_the_search_index_moved(flow):
    assert flow["staleIndex"] == "conflict" and flow["staleIndexWrites"] == 0
