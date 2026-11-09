"""Every control of the Settings panel (and of its bug-report form) has a short tooltip of its own, in Italian and English, with no file, with a low-resolution
file and with a high-resolution one: the mouse shows it after the rest delay, a long touch press shows it too, and Tab reaches every control with a visible focus ring."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_tip_impostazioni.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
# the visible controls of a panel and the tooltip the program would show for each (the nearest title / data-tip inside the panel, as eventi.js looks it up)
CTRLS = """root => { const R = document.querySelector(root), own = e => e.matches('input[type=radio],input[type=checkbox]') && e.closest('label') ? e.closest('label') : e;
  return [...R.querySelectorAll('button, input, select, textarea, summary, a[href]')].filter(e => { const t = own(e); return t.getClientRects().length > 0 && getComputedStyle(t).visibility !== 'hidden'; })
    .map(e => { let t = e.closest('[title],[data-tip],[data-tiph]'); if (t && !R.contains(t)) t = null;
      return { id: e.id || e.name || e.tagName, text: own(e).textContent.trim(), tip: t ? (t.getAttribute('title') || t.dataset.tip || t.dataset.tiph || '').trim() : '' }; }); }"""
def check_tips(pg, root, where):
    rows = pg.evaluate(CTRLS, root); bad = []
    for c in rows:
        if not c["tip"]: bad.append(f"{c['id']}: no tooltip")
        elif len(c["tip"]) > 60: bad.append(f"{c['id']}: {len(c['tip'])} characters")
        elif c["tip"] == c["text"]: bad.append(f"{c['id']}: the tooltip repeats the label")
    assert len(rows) >= 8 and not bad, f"{where}: {len(rows)} controls; {bad}"
def open_set(pg):
    if not pg.is_visible("#uipset"): pg.click("#np-set")
    pg.wait_for_selector("#uipset")
def bug_form(pg):
    pg.click("#uip-bugbtn"); pg.wait_for_selector("#bug-copy[title]")
D = synth("tipimp")
r, r2 = Run(port=free_port(), wd="/tmp/wd_tipimp"), Run(port=free_port(), wd="/tmp/wd_tipimp2")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        def hr():
            open_set(pg); assert pg.locator("#uip-ppm").count() == 1, "high-resolution rows are there"
            check_tips(pg, "#uipset", "HR file"); bug_form(pg); check_tips(pg, "#uipset", "HR file, bug form"); pg.keyboard.press("Escape")
        step("HR file: every control of the settings and of the bug form has its own short tooltip", hr)
        def mouse():
            open_set(pg); pg.hover("#uip-reset")
            wait_for_js(pg, "(() => { const q = document.getElementById('qtip'); return !q.hidden && q.textContent === I18N.t('settings.reset.title'); })()", 8000)
            pg.mouse.move(5, 5)
        step("mouse: the tooltip of a control shows after the rest delay", mouse)
        def keyboard():
            open_set(pg); pg.focus("#uip-x"); seen, bare = set(), []
            for _ in range(40):
                pg.keyboard.press("Tab")
                o = pg.evaluate("""(() => { const e = document.activeElement; if (!e || !e.closest('#uipset')) return null; const l = e.closest('label'), st = x => x && getComputedStyle(x).outlineStyle;
                  return { id: e.id || e.name || e.tagName, ring: [st(e), st(l)].some(v => v && v !== 'none') }; })()""")
                if not o: break
                seen.add(o["id"])
                if not o["ring"]: bare.append(o["id"])
            assert {"uip-th", "uip-font", "uip-hc", "uip-ppm", "uip-bugbtn", "uip-reset"} <= seen and not bare, f"reached {sorted(seen)}; no ring: {bare}"
            pg.keyboard.press("Escape")
        step("keyboard: Tab reaches the controls, each with a visible focus ring", keyboard)
        def touch():
            open_set(pg); b = pg.locator("#uip-hc").locator("xpath=ancestor::label").bounding_box(); cdp = pg.context.new_cdp_session(pg)
            pt = [{"x": b["x"] + 30, "y": b["y"] + b["height"] / 2}]
            cdp.send("Input.dispatchTouchEvent", {"type": "touchStart", "touchPoints": pt})
            wait_for_js(pg, "(() => { const q = document.getElementById('qtip'); return !q.hidden && q.textContent === I18N.t('settings.hc.tip'); })()", 8000)
            cdp.send("Input.dispatchTouchEvent", {"type": "touchEnd", "touchPoints": []})
            assert pg.evaluate("document.getElementById('uip-hc').checked") is False, "the lift after a long press is not a click"
        step("touch: a long press shows the tooltip and does not toggle the switch", touch)
        pg = r2.page(p)
        def lr_it_en():
            stage(pg, [D / "B_FullMass-t0.mzML"]); pg.click("text=Carica dati"); ready(pg)
            open_set(pg); assert pg.is_visible("#uip-tour"); check_tips(pg, "#uipset", "LR file, IT"); bug_form(pg); check_tips(pg, "#uipset", "LR file, IT, bug form")
            with pg.expect_navigation(timeout=60000): pg.click("#uipset label.lg:has(input[value=en])")
            ready(pg)
            open_set(pg); assert pg.evaluate("I18N.lang") == "en"; check_tips(pg, "#uipset", "LR file, EN"); bug_form(pg); check_tips(pg, "#uipset", "LR file, EN, bug form")
        step("LR file, Italian then English: every control has its own short tooltip", lr_it_en)
        def admin():
            pg.evaluate("localStorage.setItem(NOMI.KEY('admin'), '1')"); pg.reload(); ready(pg); open_set(pg)
            pg.evaluate("document.querySelectorAll('#uipset details.imp-g').forEach(g => { g.open = true; })"); pg.wait_for_selector("#llm-on", state="attached")
            assert pg.locator("#uip-g-llm summary").count() == 1 and pg.locator("#uip-g-diag summary").count() == 1; check_tips(pg, "#uipset", "administrator mode, EN")
        step("administrator mode: the assistant and diagnostics groups have a tooltip too", admin)
finally:
    r.close(); r2.close()
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n, "" if s == "ok" else s)
sys.exit(1 if any(s != "ok" for _, s in steps) or len(steps) < 6 else 0)
