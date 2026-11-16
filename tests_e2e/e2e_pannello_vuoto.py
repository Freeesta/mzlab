"""Side panel with no files: a tool button shows its panel when ANOTHER one is open (File included, in its empty state);
only the button of the active panel hides it. Same sequence with one synthetic file loaded. No JS errors."""
# covers: mzlab/web/barra.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:240]))
def state(pg):
    return pg.evaluate("() => ({tab: BARRA.curTab, open: !!document.querySelector('#dfiles') && getComputedStyle(document.querySelector('#dfiles')).display !== 'none' && document.querySelector('#dfiles').getBoundingClientRect().width > 0, "
                       "pressed: [...document.querySelectorAll('#stbar [data-cmd]')].filter(b => b.getAttribute('aria-pressed') === 'true').map(b => b.dataset.cmd)})")
def click(pg, cmd):
    pg.click(f'#stbar [data-cmd="{cmd}"]'); frames(pg, 3)
def sequence(pg):
    s = state(pg)
    if not s["open"]: click(pg, "file"); s = state(pg)
    assert s["tab"] == "file" and s["open"], s
    for other in ("addloss", "lists"):
        click(pg, other); s = state(pg)
        assert s["open"] and s["tab"] == other and s["pressed"] == [other], (other, s)
        click(pg, "file"); s = state(pg)
        assert s["open"] and s["tab"] == "file" and s["pressed"] == ["file"], ("file after " + other, s)
        assert pg.is_visible("#sb-panel-file") and not pg.is_visible("#sb-panel-addloss"), other
    click(pg, "file"); s = state(pg)
    assert not s["open"] and s["pressed"] == [], ("File on its own panel hides it", s)
    click(pg, "addloss"); click(pg, "file"); s = state(pg)
    assert s["open"] and s["tab"] == "file", ("closed, then another tool, then File", s)
def cold(p, with_file):
    r = Run(port=8873, wd="/tmp/wd_pannello_vuoto"); r.b = p.chromium.launch()
    ctx = r.b.new_context(viewport={"width": 1440, "height": 900}, locale="it-IT", reduced_motion="reduce")
    pg = ctx.new_page(); errs = []; pg.on("pageerror", lambda e: errs.append(str(e)))
    pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
    pg.goto(f"http://127.0.0.1:{r.port}/"); wait_for_js(pg, "typeof E !== 'undefined'")
    if with_file:
        pg.set_input_files("#pick", [{"name": "a.mzML", "mimeType": "application/octet-stream", "buffer": open(mz("B_FullMass-t15"), "rb").read()}]); wait_for_js(pg, "document.querySelectorAll('#flist input[data-k=use]').length >= 1")
        pg.click("text=Carica dati"); ready(pg)
    frames(pg, 4)
    return r, ctx, pg, errs
try:
    with sync_playwright() as p:
        for with_file, label in ((False, "no files"), (True, "one file")):
            def run(with_file=with_file):
                r, ctx, pg, errs = cold(p, with_file)
                try:
                    sequence(pg)
                    if not with_file:
                        pg.click('#stbar [data-cmd="addloss"]'); frames(pg, 3)
                        pg.click('#stbar [data-cmd="ptable"]'); frames(pg, 3)      # the periodic table is a window: the side column keeps its panel
                        click(pg, "file"); s = state(pg); assert s["open"] and s["tab"] == "file", s
                finally: ctx.close(); r.close()
                assert not errs, errs
            step(f"{label}: File after Addotti e perdite / Liste shows the File panel; active button follows", run)
finally: pass
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n, "" if s == "ok" else s)
sys.exit(1 if bad else 0)
