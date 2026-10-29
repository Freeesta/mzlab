"""«Load data» draws the work area first (shell.js #opbtn handler, apertura-area.js, openStaged), run by node with fake globals: the skeleton is up before the first file
is read, the full-screen veil stays down in the happy path, goes up only when nothing is usable yet, and a failure gives the table and the red message back."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def cut(name, start, end):
    s = (WEB / name).read_text(encoding="utf-8")
    a = s.index(start)
    return s[a:s.index(end, a)]


HARNESS = """
const log = [], els = {};
const fake = id => els[id] || (els[id] = new Proxy({ id, hidden: false, innerHTML: "", textContent: "", disabled: false, inert: false, style: {}, attrs: {}, onclick: null,
  setAttribute(k, v) { this.attrs[k] = v; }, focus() {}, appendChild() {}, get firstElementChild() { return fake(id + ">i"); } }, {}));
const $ = id => fake(id);
const document = { createElement: () => ({ style: {} }) };
const window = {};
const h = (s, ...v) => s.reduce((a, x, i) => a + x + (i < v.length ? v[i] : ""), "");
const I18N = { t: (k, p) => k + (p ? JSON.stringify(p) : "") };
const S = { view: "data", sess: null, adding: false, opening: false };
const STATO = { apertura(on) { S.opening = !!on; log.push("apertura:" + on); }, aggiunta(on) { S.adding = !!on; } };
let UPBUSY = %(upbusy)s, WAITING = false, WAIT0 = 0, idleRes = null;
const ST = { unit: "u", files: %(files)s }, usable = f => f.use, checkGo = () => {}, RUOLI = { roleOf: () => "x", commit() {} }, darkTime = t => t;
const applyView = () => { log.push("view:" + (S.opening ? "opening" : "data")); openingView(!!S.opening); };
const loading = v => log.push("veil:" + v), ldAction = (l) => log.push("ldAction:" + !!l), loadingStato = () => {}, waitStato = () => {}, convElapsed = () => "0s", PROGRESSO = { subscribe() {} };
const idle = () => UPBUSY > 0 ? new Promise(ok => { idleRes = ok; }) : Promise.resolve();
const cancelLoads = () => {};
const E = { files: [], panels: [], pending: [] }, CU = { u: ["u"] }, isCal = () => false;
let gate = null, explored = 0, booted = 0;
const exploreList = async () => { log.push("explore:start"); await gate; explored++; log.push("explore:end"); return null; };
const bootSession = async () => { log.push("boot:start"); booted++; S.sess = true; };
const uiSave = () => {};
%(openstaged)s
"""

SRC = lambda: (WEB / "apertura-area.js").read_text(encoding="utf-8")
HANDLER = lambda: cut("shell.js", '$("#opbtn").onclick=', "// both exits of the loading screen")
OPENSTAGED = lambda: cut("shell.js", "async function openStaged(", "// Files dropped on the side panel")


def run(tmp_path, body, files, upbusy=0, openstaged=None):
    code = HARNESS % {"files": json.dumps(files), "upbusy": upbusy, "openstaged": openstaged or OPENSTAGED()}
    code = code.replace("%%", "%") + SRC() + HANDLER() + "\n(async () => {\n" + body + "\n})();\n"
    f = tmp_path / "t.cjs"
    f.write_text("globalThis.window = {};\n" + code, encoding="utf-8")
    r = subprocess.run(["node", str(f)], capture_output=True, text=True, encoding="utf-8")
    assert r.returncode == 0, (r.stdout + r.stderr)[-3000:]
    return json.loads(r.stdout.strip().splitlines()[-1])


USABLE = [{"name": "a.mzML", "use": True}, {"name": "b.mzML", "use": True}]


def test_work_area_is_up_before_the_first_file_is_read_and_the_veil_never_is(tmp_path):
    r = run(tmp_path, """
  let go; gate = new Promise(ok => { go = ok; });
  const p = $("#opbtn").onclick();
  await new Promise(r => setTimeout(r, 20));
  const mid = { log: [...log], opening: S.opening, explored, title: $("#opn-title").textContent, list: $("#opn-list").innerHTML, tools: $("#tools").inert, panels: $("#dpanels").hidden };
  go(); await p;
  console.log(JSON.stringify({ mid, log, opening: S.opening, enabled: !$("#opbtn").disabled, marks: Object.keys(window.MZLAB_OPEN.t), tools: $("#tools").inert, panels: $("#dpanels").hidden }));
""", USABLE)
    mid = r["mid"]
    assert mid["opening"] is True and mid["explored"] == 0 and "view:opening" in mid["log"]
    assert mid["log"].index("view:opening") < mid["log"].index("explore:start")
    assert "a.mzML" in mid["title"] and "open.st.read" in mid["list"] and "open.st.wait" in mid["list"]
    assert mid["tools"] is True and mid["panels"] is True
    assert "veil:true" not in r["log"] and "veil:false" not in r["log"]
    assert r["opening"] is False and r["tools"] is False and r["panels"] is False
    assert r["log"].index("explore:end") < r["log"].index("boot:start") < len(r["log"]) - 1 and r["log"][-1] == "view:data"
    assert r["marks"] == ["click", "area", "first read", "first shown"]


def test_a_failure_ends_the_opening_and_gives_the_message_back(tmp_path):
    r = run(tmp_path, """
  await $("#opbtn").onclick();
  console.log(JSON.stringify({ log, opening: S.opening, msg: $("#gomsg").innerHTML, enabled: !$("#opbtn").disabled }));
""", USABLE, openstaged="async function openStaged() { log.push('openStaged'); throw Object.assign(new Error('boom'), { key: 'err.load.failed' }); }")
    assert r["opening"] is False and "boom" in r["msg"] and r["enabled"] is True
    assert r["log"] == ["apertura:true", "view:opening", "openStaged", "apertura:false", "view:data"]


def test_nothing_usable_yet_keeps_the_veil_until_the_copies_finish(tmp_path):
    files = [{"name": "a.mzML", "use": False}]
    r = run(tmp_path, """
  const p = $("#opbtn").onclick();
  await new Promise(r => setTimeout(r, 20));
  const mid = { log: [...log], opening: S.opening };
  ST.files[0].use = true; UPBUSY = 0; idleRes(); await p;
  console.log(JSON.stringify({ mid, log, opening: S.opening }));
""", files, upbusy=1)
    assert r["mid"]["opening"] is False and r["mid"]["log"] == ["veil:true", "ldAction:true"]
    assert r["log"].index("veil:true") < r["log"].index("apertura:true") < r["log"].index("veil:false")
    assert r["opening"] is False


def test_copies_under_way_do_not_hold_the_work_area_back(tmp_path):
    r = run(tmp_path, """
  const p = $("#opbtn").onclick();
  await new Promise(r => setTimeout(r, 20));
  const mid = { log: [...log], opening: S.opening, act: $("#opn-act").hidden };
  UPBUSY = 0; idleRes(); await p;
  console.log(JSON.stringify({ mid, log, opening: S.opening }));
""", USABLE, upbusy=1)
    assert r["mid"]["opening"] is True and "veil:true" not in r["log"] and r["mid"]["log"][:2] == ["apertura:true", "view:opening"]
    assert r["opening"] is False


def test_cancel_with_nothing_left_goes_back_to_the_table(tmp_path):
    r = run(tmp_path, """
  const p = $("#opbtn").onclick();
  await new Promise(r => setTimeout(r, 20));
  ST.files.forEach(f => { f.use = false; }); UPBUSY = 0; idleRes(); await p;
  console.log(JSON.stringify({ log, opening: S.opening }));
""", USABLE, upbusy=1)
    assert r["opening"] is False and r["log"][-1] == "view:data" and "explore:start" not in r["log"]
