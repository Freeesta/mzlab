"""Lab variant of the site (tools/lab/, MZLAB_LAB=1 in tools/build_site.py): the build, the guard script run in node with a fake clock, and the
normal build left alone. The browser side is tests_e2e/e2e_lab.py (CI only)."""
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
BUILD = ROOT / "tools" / "build_site.py"
LAB_JS = ROOT / "tools" / "lab" / "lab.js"
needs_node = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def build(out: Path, lab: str | None):
    env = {k: v for k, v in os.environ.items() if not k.startswith("MZLAB_LAB")}
    if lab:
        env.update(MZLAB_LAB="1", MZLAB_LAB_EXPIRES=lab)
    return subprocess.run([sys.executable, str(BUILD), "--out", str(out)], env=env, capture_output=True, text=True)


@pytest.fixture(scope="module")
def sites(tmp_path_factory):
    d = tmp_path_factory.mktemp("lab")
    assert build(d / "lab", "2026-12-29").returncode == 0
    assert build(d / "normal", None).returncode == 0
    return d / "lab", d / "normal"


def test_normal_build_has_no_trace_of_the_lab(sites):
    _, normal = sites
    assert not (normal / "static" / "lab.js").exists() and not (normal / "robots.txt").exists()
    html = (normal / "index.html").read_text(encoding="utf-8")
    assert "lab.js" not in html and "noindex" not in html and "og:title" in html and (normal / "theory" / "index.html").is_file()


def test_lab_build_head(sites):
    lab, _ = sites
    html = (lab / "index.html").read_text(encoding="utf-8")
    # the guard is the very first script of the page and the page is not indexed or previewed
    assert html.index('<script src="static/lab.js"></script>') < html.index("<script", html.index("</script>"))
    assert '<meta name="robots" content="noindex,nofollow">' in html and "og:" not in html and "canonical" not in html
    assert (lab / "robots.txt").read_text() == "User-agent: *\nDisallow: /\n"
    assert not (lab / "theory").exists()
    js = (lab / "static" / "lab.js").read_text(encoding="utf-8")
    assert 'EXPIRES = "2026-12-29"' in js and "__LAB_EXPIRES__" not in js
    assert "static/lab.js" in json.loads((lab / "static" / "manifest.json").read_text())["files"]


@pytest.mark.parametrize("env", [{}, {"MZLAB_LAB_EXPIRES": "2026-13-40"}, {"MZLAB_LAB_EXPIRES": "29/12/2026"}])
def test_lab_build_needs_a_valid_date(tmp_path, env):
    e = {k: v for k, v in os.environ.items() if not k.startswith("MZLAB_LAB")}
    r = subprocess.run([sys.executable, str(BUILD), "--out", str(tmp_path / "x")], env={**e, "MZLAB_LAB": "1", **env}, capture_output=True, text=True)
    assert r.returncode != 0 and "MZLAB_LAB_EXPIRES" in r.stderr


# --- the guard in node: a fake window/document, the clock is injected ---
HARNESS = r"""
const vm = require("vm"), fs = require("fs");
const [file, expires, nowIso, hash, stored] = process.argv.slice(1);
const store = JSON.parse(stored), calls = [];
class Storage { getItem(k) { return k in store ? store[k] : null; } setItem(k, v) { store[k] = String(v); } }
const el = () => ({ style: {}, setAttribute() {}, set textContent(v) { this.text = v; } });
const body = { firstChild: null, insertBefore(e) { this.inserted = e; } };
const doc = { readyState: "complete", body, documentElement: { set innerHTML(v) { calls.push(["page", v]); } }, addEventListener() {}, createElement: el };
const win = { stop() { calls.push(["stop"]); } };
const loc = { hash, pathname: "/qqq/", search: "" };
const RealDate = Date, now = nowIso === "none" ? NaN : RealDate.parse(nowIso);
class FakeDate extends RealDate { static now() { return now; } }
const I18N = { lang: "en", cat: {}, add(l, d) { this.cat[l] = d; }, t(k, p) { return this.cat[this.lang][k].replace("{date}", p.date); } };
const ctx = { window: win, document: doc, location: loc, Storage, Date: FakeDate, history: { replaceState(_, __, u) { loc.hash = u.slice(u.indexOf("#")); } }, addEventListener() {}, console };
win.I18N = I18N; Object.assign(win, ctx);
vm.createContext(ctx);
vm.runInContext(fs.readFileSync(file, "utf8").replace("__LAB_EXPIRES__", expires), ctx);
const s = new Storage();
console.log(JSON.stringify({ calls, hash: loc.hash, lab: !!win.MZLAB_LAB, sito: win.SITO_PUBBLICO, admin: ctx.Storage.prototype.getItem.call({ getItem: Storage.prototype.getItem }, "mzlab.admin"),
  adminViaStorage: s.getItem("mzlab.admin"), legacy: s.getItem("qqq.teoria.esteso"), prefs: s.getItem("mzlab.prefs"), written: (s.setItem("mzlab.admin", "1"), s.getItem("mzlab.admin")),
  banner: body.inserted && body.inserted.text }));
"""


def guard(expires="2026-12-29", now="2026-10-29T12:00:00Z", hash="", stored=None):
    r = subprocess.run(["node", "-e", HARNESS, str(LAB_JS), expires, now, hash, json.dumps(stored or {})], capture_output=True, text=True, check=True)
    return json.loads(r.stdout)


@needs_node
def test_guard_before_the_date_hides_the_admin_mode_and_adds_no_banner():
    r = guard(stored={"mzlab.admin": "1", "qqq.teoria.esteso": "1", "mzlab.prefs": "{}"})
    assert r["calls"] == [] and r["lab"] and r["sito"] == {"scaricaApp": False, "teoria": False}
    assert r["adminViaStorage"] is None and r["legacy"] is None and r["prefs"] == "{}" and r["written"] is None
    assert r.get("banner") is None


@needs_node
@pytest.mark.parametrize("h,want", [("#/flow", "#/data"), ("#/tpmine", "#/data"), ("#/attributio", "#/data"), ("#/theory", "#/data"), ("#/practice/3", "#/data"), ("#/data", "#/data"), ("#/draw", "#/draw"), ("", "")])
def test_guard_addresses(h, want):
    assert guard(hash=h)["hash"] == want


@needs_node
@pytest.mark.parametrize("now,expired", [("2026-12-29T23:59:59Z", False), ("2026-12-30T00:00:00Z", True), ("2027-06-01T00:00:00Z", True), ("none", True)])
def test_guard_expiry_with_a_fake_clock(now, expired):
    r = guard(now=now)
    assert (r["calls"] != []) == expired
    if expired:
        page = r["calls"][1][1]
        assert r["calls"][0] == ["stop"] and "expired" in page and "scaduta" in page and not r["lab"] and r.get("banner") is None


@needs_node
@pytest.mark.parametrize("bad", ["", "soon", "2026-13-40", "29/12/2026"])
def test_guard_never_fails_open(bad):
    assert guard(expires=bad)["calls"] != []


def test_qqq_workflow_follows_the_pages_workflow():
    """tools/lab/qqq-pages.yml has the same jobs as .github/workflows/pages.yml (same pinned actions, same build steps); only the triggers, the lab
    variables and the checkout of Freeesta/mzlab main differ."""
    cut = lambda t: t[t.index("# No run is ever cancelled"):]
    main = cut((ROOT / ".github" / "workflows" / "pages.yml").read_text(encoding="utf-8"))
    qqq = cut((ROOT / "tools" / "lab" / "qqq-pages.yml").read_text(encoding="utf-8"))
    co = "          repository: Freeesta/mzlab\n          ref: main\n"
    assert co in qqq and qqq.replace("        with:\n" + co, "", 1) == main
    head = (ROOT / "tools" / "lab" / "qqq-pages.yml").read_text(encoding="utf-8").split("# No run is ever cancelled")[0]
    assert "schedule:" in head and "workflow_dispatch:" in head and 'MZLAB_LAB: "1"' in head and 'MZLAB_LAB_EXPIRES: "2026-12-29"' in head
    assert "secrets." not in qqq and "PAT" not in qqq
    assert (ROOT / "tools" / "lab" / "README.md").stat().st_size <= 2048
