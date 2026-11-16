"""Side panel on the empty landing and with one file: a tool button SHOWS its panel when another one is open (File included, in its
empty state); only the active tool's button hides it. First action on a fresh landing, Ctrl/Cmd+B and #sb-toggle, pressed state,
no transient hide/show after any action (sampled every frame for 500 ms). No JS errors."""
# covers: mzlab/web/barra.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; traceback.print_exc(); steps.append((name, "FAIL: " + str(e).split("\n")[0][:240]))
STATE = """() => { const a = document.querySelector('#dfiles'), t = document.querySelector('#sb-toggle');
  const on = [...document.querySelectorAll('#stbar [data-cmd]')].filter(b => b.getAttribute('aria-pressed') === 'true').map(b => b.dataset.cmd);
  return {tab: BARRA.curTab, open: !!a && a.getBoundingClientRect().width > 0 && a.getAttribute('aria-hidden') !== 'true', pressed: on, tg: t && t.getAttribute('aria-pressed')}; }"""
# one sample per frame, started by the next click/keydown (capture, once), so only post-action frames count
SAMPLER = """(ms) => { window.__smp = new Promise(res => { const out = [];
  const go = () => { const t0 = performance.now(); const tick = () => { const a = document.querySelector('#dfiles');
    out.push(!!a && a.getBoundingClientRect().width > 0 && a.getAttribute('aria-hidden') !== 'true'); if (performance.now() - t0 < ms) requestAnimationFrame(tick); else res(out); }; requestAnimationFrame(tick); };
  document.addEventListener('click', go, {capture: true, once: true}); document.addEventListener('keydown', go, {capture: true, once: true}); }); }"""
def state(pg): return pg.evaluate(STATE)
def stable(pg, act, visible, ms=500):
    """Do `act` and require the panel to be `visible` (or hidden) in EVERY frame for `ms` ms: no transient toggle."""
    pg.evaluate(SAMPLER, ms); act(); smp = pg.evaluate("window.__smp")
    assert smp and all(v == visible for v in smp), (visible, smp)
def click(pg, cmd, visible):
    stable(pg, lambda: pg.click(f'#stbar [data-cmd="{cmd}"]'), visible)
def consistent(pg, s, tab=None):
    """The pressed buttons say what is on screen: the open panel's button, or none when it is hidden."""
    assert s["pressed"] == ([tab] if s["open"] else []) and s["tg"] == ("true" if s["open"] else "false"), s
def cold(p, with_file, vw=1440):
    port = free_port(); r = Run(port=port, wd=f"/tmp/wd_pannello_vuoto_{port}"); r.b = p.chromium.launch()
    ctx = r.b.new_context(viewport={"width": vw, "height": 900}, locale="it-IT", reduced_motion="reduce")
    pg = ctx.new_page(); errs = []; pg.on("pageerror", lambda e: errs.append(str(e)))
    pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
    pg.goto(f"http://127.0.0.1:{r.port}/"); wait_for_js(pg, "typeof E !== 'undefined' && window.qqReady === true")      # BARRA.init (key handlers) runs right after qqReady
    if with_file:
        pg.set_input_files("#pick", [{"name": "a.mzML", "mimeType": "application/octet-stream", "buffer": open(mz("B_FullMass-t15"), "rb").read()}]); wait_for_js(pg, "document.querySelectorAll('#flist input[data-k=use]').length >= 1")
        pg.click("text=Carica dati"); ready(pg)
    frames(pg, 6)
    return r, ctx, pg, errs
def scenario(p, fn, with_file=False, vw=1440):
    r, ctx, pg, errs = cold(p, with_file, vw)
    try: fn(pg)
    finally: ctx.close(); r.close()
    assert not errs, errs
try:
    with sync_playwright() as p:
        def first_click(pg):
            s = state(pg); assert s["open"] and s["tab"] == "file", ("a fresh landing above 900 px starts with the File panel open", s); consistent(pg, s, "file")
            click(pg, "file", False); s = state(pg); assert not s["open"]; consistent(pg, s)      # File is the active tool: its button hides the panel, once, no flicker
            click(pg, "file", True); s = state(pg); assert s["open"] and s["tab"] == "file"; consistent(pg, s, "file")
        step("empty landing, File as the very first action: hides the open panel (active tool), then shows it, never flickers", lambda: scenario(p, first_click))

        def narrow_first(pg):
            s = state(pg); assert not s["open"], ("below 900 px the landing starts closed", s)
            click(pg, "file", True); s = state(pg); assert s["open"] and s["tab"] == "file"; consistent(pg, s, "file")
        step("empty landing below 900 px: File as the first action opens the panel", lambda: scenario(p, narrow_first, vw=800))

        def keyboard(pg):
            for how in ("ctrl+b", "toggle", "ctrl+b", "toggle"):      # Ctrl/Cmd+B and the header button are the same action on the empty landing
                want = not state(pg)["open"]
                act = (lambda: pg.keyboard.press("Control+b")) if how == "ctrl+b" else (lambda: pg.click("#sb-toggle"))
                stable(pg, act, want); s = state(pg); assert s["open"] == want, (how, s); consistent(pg, s, "file")
            click(pg, "addloss", True); consistent(pg, state(pg), "addloss")
            stable(pg, lambda: pg.keyboard.press("Control+b"), False); consistent(pg, state(pg))
            stable(pg, lambda: pg.click("#sb-toggle"), True); s = state(pg); assert s["tab"] == "addloss", ("the header button brings back the last tool", s); consistent(pg, s, "addloss")
        step("empty landing: Ctrl/Cmd+B and #sb-toggle show/hide, pressed follows, last tool kept", lambda: scenario(p, keyboard))

        def sequence(pg, landing):
            s = state(pg)
            if not s["open"]: click(pg, "file", True)
            for other in ("addloss", "lists"):
                click(pg, other, True); s = state(pg)
                assert s["open"] and s["tab"] == other, (other, s); consistent(pg, s, other)
                click(pg, "file", True); s = state(pg)      # another tool is shown: File SELECTS its panel, the column never blinks
                assert s["open"] and s["tab"] == "file", ("file after " + other, s); consistent(pg, s, "file")
                assert pg.is_visible("#sb-panel-file") and not pg.is_visible("#sb-panel-" + other), other
            if landing:
                click(pg, "file", False); s = state(pg); assert not s["open"]; consistent(pg, s)      # its own panel: hides
                click(pg, "addloss", True); click(pg, "file", True); s = state(pg)
                assert s["open"] and s["tab"] == "file", ("hidden, then another tool, then File", s)
                pg.click('#stbar [data-cmd="addloss"]'); frames(pg, 3)
                pg.click('#stbar [data-cmd="ptable"]'); frames(pg, 3)      # the periodic table is a window: the side column keeps its panel
                s = state(pg); assert s["open"] and s["tab"] == "addloss", ("Tavola leaves the side column alone", s)
                pg.keyboard.press("Escape"); frames(pg, 3)
                click(pg, "file", True); s = state(pg); assert s["open"] and s["tab"] == "file", s
            else:
                click(pg, "file", True); s = state(pg)      # with a file a second click goes back to the previous tool (docs: dati-barra.md)
                assert s["open"] and s["tab"] == "lists", s
        step("no files: File after Addotti e perdite / Liste shows the File panel; active button follows", lambda: scenario(p, lambda pg: sequence(pg, True)))
        step("one file: same sequence; a second click on the active tool goes back to the previous one", lambda: scenario(p, lambda pg: sequence(pg, False), with_file=True))
finally: pass
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n, "" if s == "ok" else s)
sys.exit(1 if bad else 0)
