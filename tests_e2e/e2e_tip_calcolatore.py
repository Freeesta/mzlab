"""Every control of the Adducts & losses page (calculator, choices row, the three tabs) and of the floating Calculator window has a short tooltip of its own, in Italian
and English, with no data file: the mouse shows it after the rest delay, a long touch press shows it without pressing the button, and Tab reaches the controls with a focus ring."""
# covers: mzlab/web/tooltip.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_tip_calcolatore.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
# the visible controls of a panel and the tooltip the program would show for each (the nearest title / data-tip inside the panel, as eventi.js looks it up)
CTRLS = """root => { const R = document.querySelector(root);
  return [...R.querySelectorAll('button, input, select, textarea, summary, a[href]')].filter(e => e.getClientRects().length > 0 && getComputedStyle(e).visibility !== 'hidden')
    .map(e => { let t = e.closest('[title],[data-tip],[data-tiph]'); if (t && !R.contains(t)) t = null;
      return { id: e.id || e.className || e.tagName, text: e.textContent.trim(), tip: t ? (t.getAttribute('title') || t.dataset.tip || t.dataset.tiph || '').trim() : '' }; }); }"""
# what each query brings out: the adduct table with XIC buttons, the loss rows, the contaminant list
CASES = [("C8H9NO2", "adducts", ("cxic", "cadd-more", "al-t-losses")), ("214", "adducts", ("cadd-more", "altol")), ("44", "losses", ("nlrow", "al-calc-tg")), ("214", "cont", ("al-t-cont",))]
def check_tips(pg, root, where, need=()):
    pg.evaluate(f"document.querySelectorAll('{root} details').forEach(d => {{ d.open = true; }})"); frames(pg, 5)
    rows = pg.evaluate(CTRLS, root); bad = []
    for c in rows:
        if not c["tip"]: bad.append(f"{c['id']} «{c['text'][:20]}»: no tooltip")
        elif len(c["tip"]) > 60: bad.append(f"{c['id']}: {len(c['tip'])} characters")
        elif c["tip"] == c["text"]: bad.append(f"{c['id']}: the tooltip repeats the label")
    ids = " ".join(c["id"] for c in rows)
    assert len(rows) >= 10 and not bad and all(n in ids for n in need), f"{where}: {len(rows)} controls; {bad}; missing {[n for n in need if n not in ids]}"
def page_tab(pg, q, sub):
    pg.evaluate(f"BARRA.setTab('{sub}', {{q: '{q}'}})"); frames(pg, 40)
def all_cases(pg, lang):
    for q, sub, need in CASES:
        page_tab(pg, q, sub)
        check_tips(pg, "#sb-panel-addloss", f"{lang}, page, {q} on {sub}", need)
def calc_window(pg, lang):
    pg.evaluate("setView('draw')"); wait_for_js(pg, "!!document.querySelector('#np-calc2') && !!document.querySelector('#np-calc2').offsetParent")
    if pg.is_hidden("#calcdlg"): pg.click("#np-calc2")
    wait_for_js(pg, "!document.querySelector('#calcdlg').hidden")
    for q, need in (("44", ("cq-lb", "calcx")), ("C8H9NO2", ("cxic", "cadd-more"))):
        pg.fill("#calcin", q); pg.evaluate("document.querySelector('#calcin').dispatchEvent(new Event('input',{bubbles:true}))"); frames(pg, 40)
        check_tips(pg, "#calcdlg", f"{lang}, calculator window, {q}", need)
    pg.click("#calcx")
r = Run(port=free_port(), wd="/tmp/wd_tipcalc")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        wait_for_js(pg, "window.qqReady && !document.querySelector('#start').hidden && !!window.BARRA")
        step("Italian: every control of the Adducts & losses page has its own short tooltip", lambda: all_cases(pg, "IT"))
        step("Italian: every control of the Calculator window has its own short tooltip", lambda: calc_window(pg, "IT"))
        def mouse():
            page_tab(pg, "214", "adducts"); pg.hover("#al-t-losses")
            wait_for_js(pg, "(() => { const q = document.getElementById('qtip'); return !q.hidden && q.textContent === I18N.t('al.t.losses.tip'); })()", 8000)
            pg.mouse.move(5, 5)
        step("mouse: the tooltip of a tab shows after the rest delay", mouse)
        def touch():
            page_tab(pg, "214", "adducts"); da = pg.locator("#alunit [data-u=da]"); was = da.get_attribute("aria-pressed")
            b = da.bounding_box(); pt = [{"x": b["x"] + b["width"] / 2, "y": b["y"] + b["height"] / 2}]; cdp = pg.context.new_cdp_session(pg)
            cdp.send("Input.dispatchTouchEvent", {"type": "touchStart", "touchPoints": pt})
            wait_for_js(pg, "(() => { const q = document.getElementById('qtip'); return !q.hidden && q.textContent === I18N.t('cq.unit'); })()", 8000)
            cdp.send("Input.dispatchTouchEvent", {"type": "touchEnd", "touchPoints": []}); frames(pg, 5)
            assert pg.locator("#alunit [data-u=da]").get_attribute("aria-pressed") == was, "the lift after a long press is not a click"
        step("touch: a long press shows the tooltip and does not press the button", touch)
        def keyboard():
            page_tab(pg, "214", "adducts"); pg.focus("#calcin"); seen, bare = set(), []
            for _ in range(40):
                pg.keyboard.press("Tab")
                o = pg.evaluate("""(() => { const e = document.activeElement; if (!e || !e.closest('#sb-panel-addloss')) return null;
                  return { id: e.id || e.dataset.u || e.dataset.p || e.dataset.k || e.className, ring: getComputedStyle(e).outlineStyle !== 'none' }; })()""")
                if not o: break
                seen.add(o["id"])
                if not o["ring"]: bare.append(o["id"])
            assert {"altol", "da", "ppm", "1", "-1", "al-t-adducts", "cadd-more"} <= seen and not bare, f"reached {sorted(seen)}; no ring: {bare}"
        step("keyboard: Tab reaches the controls, each with a visible focus ring", keyboard)
        def english():
            pg.click("#np-set"); pg.wait_for_selector("#uipset")
            with pg.expect_navigation(timeout=60000): pg.click("#uipset label.lg:has(input[value=en])")
            wait_for_js(pg, "window.qqReady && !document.querySelector('#start').hidden && !!window.BARRA && I18N.lang === 'en'")
            all_cases(pg, "EN"); calc_window(pg, "EN")
        step("English: the page and the window again, every control with its own short tooltip", english)
finally:
    r.close()
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n, "" if s == "ok" else s)
sys.exit(1 if any(s != "ok" for _, s in steps) or len(steps) < 6 else 0)
