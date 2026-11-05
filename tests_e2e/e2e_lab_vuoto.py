"""Lab build (MZLAB_LAB=1) with NO file loaded: every control of Data and Draw, every command of the side bar and what it opens, the shortcuts,
the search box and the Draw canvas menu are used once; any console error, uncaught exception, error box or engine miss fails the step.
Also: the hint of the search box gives results with no file, Ctrl/⌘+E opens no dialog, «Nothing to export» goes away once something is drawn.
Builds the lab site and serves it. Run from the repo root: python3 tests_e2e/e2e_lab_vuoto.py"""
import os, socket, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402  (patches wait_for_function for the CSP)
from lib import free_port, attendi; from lib_sito import add_wasm  # noqa: E402
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_lab_vuoto.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e)[:900]))
tmp = Path(tempfile.mkdtemp())
env = {k: v for k, v in os.environ.items() if not k.startswith("MZLAB_LAB")}; env.update(MZLAB_LAB="1", MZLAB_LAB_EXPIRES="2026-12-29")
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(tmp / "lab")], check=True, env=env, stdout=subprocess.DEVNULL)
add_wasm(tmp / "lab")
port = free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(port), str(tmp / "lab")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
while True:          # the server listens (state, not a pause)
    try: socket.create_connection(("127.0.0.1", port), timeout=1).close(); break
    except OSError: pass
URL = f"http://127.0.0.1:{port}/"
INIT = ("(()=>{const R=Date,t=R.parse('2026-10-29T12:00:00Z');class D extends R{constructor(...a){a.length?super(...a):super(t)}static now(){return t}};window.Date=D})();"      # before the expiry
        "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}))}catch(_){}")
# what can be clicked or typed in, outside the header tabs and the Ketcher frame; every element gets a stable mark
CONTROLS = """(root) => [...document.querySelectorAll(root + ' :is(button, [role=button], [role=tab], [role=menuitem], [role=option], summary, input, select, textarea)')]
  .filter(e => e.getClientRects().length && getComputedStyle(e).visibility !== 'hidden' && !e.disabled && e.type !== 'file' && !e.closest('#nav, #kframe, #tour-dialog'))
  .map(e => { if (!e.dataset.vuoto) e.dataset.vuoto = String(window.__nv = (window.__nv || 0) + 1);
    return { k: e.dataset.vuoto, id: e.id, tag: e.tagName, type: e.type || '', name: (e.getAttribute('aria-label') || e.title || e.textContent || e.value || '').trim().slice(0, 40), grp: e.closest('dialog, [role=dialog], [role=region], details, section, aside')?.id || '' }; })"""
# outside the empty state or outside the page: «add files» (the drop area, like #pick), new session (reload), language (reload), external sites, the bug report, the tours (e2e_lab walks them)
SKIP = {"addf", "newrun", "np-pc", "np-bt", "uip-bugbtn", "uip-reset", "uip-tour", "pick", "side-toggle"}
GONE = "(() => { const e = document.getElementById('errbox'); if (!e) return ''; const t = e.innerText; e.remove(); return t; })()"
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        ctx = b.new_context(viewport={"width": 1440, "height": 950}, accept_downloads=True); pg = ctx.new_page(); pg.add_init_script(INIT)
        bad, where, used = [], ["load"], []
        pg.on("pageerror", lambda e: bad.append(f"{where[0]}: uncaught {e}"))
        pg.on("console", lambda m: m.type == "error" and bad.append(f"{where[0]}: console {m.text[:300]}"))
        pg.on("popup", lambda q: q.close()); pg.on("filechooser", lambda fc: None); pg.on("dialog", lambda d: d.dismiss())
        pg.goto(URL, wait_until="commit")
        pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_function("crossOriginIsolated === true", timeout=60000); pg.wait_for_selector("#drop", timeout=120000)
        attendi(pg, "typeof setView === 'function' && typeof CMD !== 'undefined' && typeof PALETTE !== 'undefined'", 120000)
        def check(what):
            eb = pg.evaluate(GONE)
            if eb: bad.append(f"{what}: error box {eb[:200]}")
            miss = pg.evaluate("(window.MZLAB_LR_MISS || []).splice(0)")
            if miss: bad.append(f"{what}: engine miss {miss}")
            assert not bad, "\n".join(dict.fromkeys(bad))
        def back(view):
            pg.keyboard.press("Escape"); pg.evaluate("document.querySelectorAll('dialog[open]').forEach(d => d.close()); PALETTE.close(false); window.BARRA && BARRA.isOpen() && BARRA.close(false)")
            if pg.evaluate("S.view") != view: pg.evaluate(f"setView('{view}')"); attendi(pg, f"S.view === '{view}'")
        def use(c):
            used.append(where[0]); loc = pg.locator(f"[data-vuoto='{c['k']}']")
            if c["tag"] in ("INPUT", "TEXTAREA") and c["type"] not in ("checkbox", "radio", "button", "submit", "range", "color"):
                for v in ("abc", "18", "-1", ""): loc.fill(v, timeout=3000); loc.press("Enter", timeout=3000)
            elif c["tag"] == "SELECT":
                for j in range(pg.evaluate("k => document.querySelector(`[data-vuoto='${k}']`).options.length", c["k"])): loc.select_option(index=j, timeout=3000)
            else:
                try: loc.click(timeout=1500)
                except Exception: pg.evaluate("k => document.querySelector(`[data-vuoto='${k}']`).click()", c["k"])      # covered by an earlier popover: the handler still runs
            pg.evaluate("n => new Promise(ok => { const f = () => (--n > 0 ? requestAnimationFrame(f) : ok()); requestAnimationFrame(f) })", 3)
        plain = {}          # the controls of the bare page of each view: what an opener adds is the rest
        def walk(view, opener=None, root="body", cap=60):
            """Uses every control that `opener` shows (or, with none, every control of the page), reopening it when a control closed it."""
            back(view); base = plain.setdefault(view, {c["k"] for c in pg.evaluate(CONTROLS, root)}) if opener else set()
            if opener: opener()
            seen = set()
            todo = [c for c in pg.evaluate(CONTROLS, root) if c["k"] not in base and c["id"] not in SKIP]
            todo.sort(key=lambda c: c["id"] in ("uip-x", "refx", "listex", "calcx", "sb-x"))      # the close buttons last
            for c in todo[:cap]:
                key = (c["grp"], c["tag"], c["id"] or c["name"] if c["grp"] != "refdlg" else "element")      # one element of the periodic table stands for all
                if key in seen or c["name"] in ("English", "Italiano"): continue
                seen.add(key); where[0] = f"{view}: {opener.__name__ if opener else 'page'} > {c['tag']} {c['id'] or c['name']}"
                if not pg.evaluate("k => { const e = document.querySelector(`[data-vuoto='${k}']`); return !!(e && e.getClientRects().length) }", c["k"]):
                    back(view)
                    if opener: opener()
                    if not pg.evaluate("k => { const e = document.querySelector(`[data-vuoto='${k}']`); return !!(e && e.getClientRects().length) }", c["k"]): continue
                use(c); check(where[0])
            back(view); check(f"{view}: after {opener.__name__ if opener else 'page'}")
        def commands(view):
            back(view)
            ids = pg.evaluate("CMD.filter(c => c.run && !c.items && (!c.visible || c.visible())).map(c => c.id)")
            assert "calc" in ids and "ptable" in ids, ids
            for i in ids:
                if i in ("pubchem", "bio", "search"): continue
                def run(i=i): pg.evaluate("id => CMD.find(x => x.id === id).run()", i); pg.evaluate("new Promise(r => setTimeout(r, 150))")
                run.__name__ = "command " + i
                walk(view, run)
        KEYS = ["Control+z", "Control+Shift+z", "Control+y", "Control+e", "Control+b", "Control+k", "?", "Shift+H", "Alt+ArrowLeft", "Alt+ArrowRight",
                "Backspace", "Delete", "Escape", "f", "i", "r", "x", "Enter", "Shift+F10", "ContextMenu", "Control+a", "Control+c"]
        def keys(view):
            for k in KEYS:
                back(view); where[0] = f"{view}: key {k}"
                pg.evaluate("document.activeElement && document.activeElement.blur()"); pg.keyboard.press(k)
                pg.evaluate("new Promise(r => setTimeout(r, 120))"); check(where[0])
                if k == "Control+e": assert not pg.evaluate("!!document.querySelector('dialog[open]')"), "Ctrl/⌘+E with no file: no dialog " + pg.evaluate("[...document.querySelectorAll('dialog[open]')].map(d => d.innerText).join()")
            back(view); pg.keyboard.press("Alt+2" if view == "data" else "Alt+1"); attendi(pg, "S.view === " + repr("draw" if view == "data" else "data")); back(view); check(f"{view}: Alt+1/2")
        def data():
            pg.evaluate("setView('data')"); walk("data"); commands("data"); keys("data")
            assert len(used) >= 40, ("the walk reached too few controls", len(used), used)
        step("data with no file: every control, every side-bar command and what it opens, every shortcut, no error", data)
        def hint():
            for view in ("data", "draw"):
                back(view); pg.evaluate("PALETTE.open()"); attendi(pg, "!!document.querySelector('#paletta:not([hidden]) #pal-e')")
                live = pg.inner_text("#pal-n"); assert not live.strip(), ("nothing typed: no «nothing found» for the screen reader", live)
                tries = [t.strip() for t in pg.inner_text("#pal-e").split(":", 1)[1].split(",")]
                assert len(tries) == 3, tries
                for t in tries:
                    pg.fill("#pal-in", t); attendi(pg, "document.querySelectorAll('#pal-l [role=option]').length > 0 || document.getElementById('pal-n').textContent !== ''", 15000)
                    assert pg.locator("#pal-l [role=option]").count() > 0, (view, t, pg.inner_text("#pal-e"))
                where[0] = f"{view}: palette"; check(where[0])
            back("data")
        step("the hint of the search box gives results with no file (18 at unit resolution)", hint)
        def draw():
            pg.evaluate("setView('draw')"); attendi(pg, "!!(window.TPDraw && TPDraw.ready())", 120000)
            K = "document.querySelector('#kframe').contentWindow.ketcher"
            walk("draw", root="#v-draw")
            where[0] = "draw: export of the empty canvas"; pg.evaluate(f"{K}.setMolecule('')"); attendi(pg, f"{K}.editor.struct().isBlank()")
            pg.click("#ex-png"); attendi(pg, "!document.getElementById('ex-warn').hidden"); assert pg.get_attribute("#ex-warn", "role") == "status"
            pg.fill("#ex-smi", "CCO"); pg.click("#ex-load"); attendi(pg, f"!{K}.editor.struct().isBlank()")
            attendi(pg, "document.getElementById('ex-warn').hidden", 10000)      # «Nothing to export» is stale once something is drawn
            pg.evaluate(f"{K}.setMolecule('')"); attendi(pg, f"{K}.editor.struct().isBlank()")
            for v in ("C1CC", "zzzz qqqq", "InChI=1S/x"):
                where[0] = f"draw: quick draw {v!r}"; pg.fill("#ex-smi", v); pg.click("#ex-load")
                attendi(pg, "!document.getElementById('smi-warn').hidden", 15000); assert pg.get_attribute("#smi-warn", "role") == "status"; check(where[0])
            where[0] = "draw: canvas menu, empty canvas"
            pg.evaluate("(()=>{const d=document.querySelector('#kframe').contentDocument;d.body.dispatchEvent(new d.defaultView.MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:300,clientY:300}))})()")
            check(where[0])
            n = pg.evaluate("(() => { const d = document.querySelector('#kframe').contentDocument; return [...d.querySelectorAll('#tp-bar button, [data-testid^=zoom-][data-testid$=-tp]')].filter(e => e.getClientRects().length).map((e, i) => (e.dataset.vuoto = 'k' + i)).length })()")
            assert n >= 4, n
            for i in range(n):
                where[0] = f"draw: button {i} in the Ketcher frame"; pg.frame_locator("#kframe").locator(f"[data-vuoto='k{i}']").click(timeout=5000); pg.keyboard.press("Escape"); check(where[0])
            where[0] = "draw: panels toggle"; pg.click("#side-toggle"); pg.click("#side-toggle"); check(where[0])
            n0 = len(used); commands("draw"); keys("draw")
            assert len(used) - n0 >= 50, ("the walk reached too few controls in Draw", len(used) - n0)
        step("draw with no file: every control, quick draw with wrong text, exports and menu of the empty canvas, frame buttons, commands, shortcuts", draw)
        def shortcuts_panel():
            back("data"); pg.evaluate("document.activeElement && document.activeElement.blur()"); pg.keyboard.press("?"); attendi(pg, "!!document.querySelector('#scorc[open]')")
            heads = pg.evaluate("[...document.querySelectorAll('#scorc h3')].map(h => h.textContent)")
            assert heads == ["Ovunque", "Sui grafici (con il fuoco sul grafico)"], heads      # no chart: the chart keys are not listed as «everywhere»
            rows = pg.evaluate("[...document.querySelectorAll('#scorc li')].map(li => li.innerText)")
            assert len(rows) == len(set(rows)), rows
            back("data"); check("data: shortcuts panel")
        step("the shortcuts panel with no chart: chart keys under their own heading, no line twice", shortcuts_panel)
        print("   controls used:", len(used)); Path(os.environ.get("VUOTO_LOG", os.devnull)).write_text("\n".join(used))
        b.close()
finally:
    srv.terminate()
fails = [s for s in steps if s[1] != "ok"]
for s in steps: print("  ", s)
sys.exit(1 if fails else 0)
