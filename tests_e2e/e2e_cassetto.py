"""The toolbar drawer (#dfiles) is really visible from a cold start: with and without #/dati, at 1280, 1440 and on an iPad,
also with saved sidebar states (side right, pinned) and a stale width. Visible = non-empty box inside the viewport, on top."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:240]))
CHECK = """() => { const e = document.getElementById('dfiles'), b = e.getBoundingClientRect(), t = document.getElementById('flst');
  const x = Math.min(innerWidth - 2, Math.max(1, b.x + b.width / 2)), y = Math.min(innerHeight - 2, Math.max(1, b.y + 120)), top = document.elementFromPoint(x, y);
  return {w: b.width, h: b.height, x: b.x, r: b.right, vw: innerWidth, vh: innerHeight, cls: document.getElementById('v-data').className, shown: !!(top && e.contains(top)), vis: getComputedStyle(e).visibility}; }"""
def cold(p, vw, vh, hash_="", saved=None, touch=False):
    r = Run(port=8871, wd="/tmp/wd_cassetto"); r.b = p.chromium.launch()      # a server of its own: it remembers the session
    ctx = r.b.new_context(viewport={"width": vw, "height": vh}, locale="it-IT", has_touch=touch, is_mobile=False, reduced_motion="reduce")
    pg = ctx.new_page()
    errs = []; pg.on("pageerror", lambda e: errs.append(str(e)))
    init = "try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');"
    for k, v in (saved or {}).items(): init += f"localStorage.setItem('{k}',{v!r});"
    pg.add_init_script(init + "}catch(_){}")
    pg.goto(f"http://127.0.0.1:{r.port}/{hash_}"); pg.wait_for_timeout(800)
    pg.set_input_files("#pick", [{"name": "a.mzML", "mimeType": "application/octet-stream", "buffer": open(mz("B_FullMass-t15"), "rb").read()}]); pg.wait_for_timeout(800)
    pg.click("text=Carica dati"); ready(pg)
    pg.click("#stbar [data-cmd=file]"); pg.wait_for_timeout(600)
    v = pg.evaluate(CHECK); ctx.close(); r.close()
    assert not errs, errs
    assert v["w"] > 100 and v["h"] > 200 and v["x"] >= -1 and v["r"] <= v["vw"] + 1 and v["shown"] and v["vis"] == "visible" and "sb-open" in v["cls"], v
try:
    with sync_playwright() as p:
        for vw, vh, tag, touch in ((1280, 800, "1280", False), (1440, 900, "1440", False), (1024, 1366, "iPad", True)):
            for h in ("", "#/dati"):
                step(f"{tag} {h or 'no hash'}: the drawer opens and is visible", lambda vw=vw, vh=vh, h=h, touch=touch: cold(p, vw, vh, h, touch=touch))
        step("side right saved", lambda: cold(p, 1440, 900, saved={"qqq.barra.lato": "right"}))
        step("pinned saved", lambda: cold(p, 1440, 900, saved={"qqq.barra.lr": '{"tab":"file","pinned":true,"open":true}'}))
        step("stale width saved", lambda: cold(p, 1440, 900, saved={"qqq.barra.lr": '{"tab":"calc","w":0}'}))
finally: pass
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n, "" if s == "ok" else s)
sys.exit(1 if bad else 0)
