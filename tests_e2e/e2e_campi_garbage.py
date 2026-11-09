"""Garbage in the typed numeric fields (campi.js): letters, 1e999, empty, a number outside the bounds go into every visible field of the panel headers,
the spectrum parameters, the peak table and Settings; each is refused with one calm note (role=status), the old value comes back and nothing changes in the
state or in a request; a comma decimal («12,5») is taken. The parser itself is checked on the same cases."""
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_campi_garbage.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
D = synth("campi_garbage")
r = Run(port=8912, wd="/tmp/wd_campi_garbage")
SPEC = "E.panels.find(q => q.type === 'spec' && q.tab === E.tab)"
STATE = ("JSON.stringify([E.panels.map(q => [q.thr, q.nlab, q.gamma, q.gdelay, q.pdaDelay, q.bw0, q.bw1, q.mz0, q.mz1, q.fondo && q.fondo.w, q.fondo && q.fondo.zd]),"
         " localStorage.getItem('mzlab.prefs'), localStorage.getItem('mzlab.llm')])")
NOTE = """(x => { const box = x.closest('.cpop, .sp-pop') || x.closest('.pnl'); const m = box ? box.querySelector(':scope > .xr-msg')
  : ((x.closest('label') || x).nextElementSibling); return m && !m.hidden && m.getAttribute('role') === 'status' ? m.textContent : ''; })"""
FIELDS = """(scope => [...document.querySelectorAll(scope + ' input[data-num]')].filter(x => x.offsetParent && !x.disabled)
  .map(x => { x.dataset.garb = Math.random().toString(36).slice(2); return { id: x.dataset.garb, min: x.dataset.min ?? null, max: x.dataset.max ?? null,
  gt: x.dataset.gt ?? null, empty: x.hasAttribute('data-vuoto') }; }))"""
urls = []
def garbage(f):
    out = ["abc", "1e999", "12abc", "1,2.3"]
    if not f["empty"]: out.append("")
    if f["min"] is not None: out.append(str(float(f["min"]) - 1))
    if f["gt"] is not None: out.append(f["gt"])
    if f["max"] is not None: out.append(str(float(f["max"]) + 1))
    return out
def sweep(pg, scope):
    fs = pg.evaluate(FIELDS + f"({scope!r})")
    assert fs, f"no numeric field visible in {scope}"
    texts = pg.evaluate("['campo.num', 'campo.empty', 'campo.gt', 'campo.range', 'campo.min', 'campo.max'].map(k => I18N.t(k).split('{')[0])")
    for f in fs:
        sel = f"[data-garb='{f['id']}']"
        for g in garbage(f):
            x = pg.locator(sel)
            if not x.count(): break          # a refusal never rebuilds the header: a missing field is a bug of its own
            before, st0 = x.input_value(), pg.evaluate(STATE)
            x.focus(); x.press("ControlOrMeta+a"); x.press_sequentially(g) if g else x.press("Delete"); x.press("Tab")
            wait_for_js(pg, f"(() => {{ const x = document.querySelector({sel!r}); return x && {NOTE}(x) !== ''; }})()", 5000)
            msg = pg.evaluate(f"{NOTE}(document.querySelector({sel!r}))")
            assert any(msg.startswith(t) for t in texts), f"{scope} {sel} {g!r}: note {msg!r}"
            assert pg.locator(sel).input_value() == before, f"{scope} {sel} {g!r}: value {pg.locator(sel).input_value()!r}, was {before!r}"
            assert pg.get_attribute(sel, "aria-invalid") == "true", f"{scope} {sel} {g!r}: aria-invalid"
            assert pg.evaluate(STATE) == st0, f"{scope} {sel} {g!r}: the state changed"
    return len(fs)
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.on("request", lambda q: urls.append(q.url))
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 6000, 2); ready(pg)
        def parser():
            v = pg.evaluate("""[['250,8', '250.8', ' 7 ', '-3', '−3', '1e3', '.5', '', 'abc', '1e999', '12abc', '1,2,3', '1,2.3', '--1', 'Infinity', 'NaN'].map(t => CAMPI.num(t)),
              [numIt('1e999'), numIt('5 ppm'), numIt('0,5')]].map(a => a.map(x => Number.isNaN(x) ? 'NaN' : x))""")
            assert v[0][:8] == [250.8, 250.8, 7, -3, -3, 1000, 0.5, None], v
            assert v[0][8:] == ["NaN"] * 8, v
            assert v[1] == ["NaN", 5, 0.5], v
        step("the parser: comma or point, sign and exponent taken; letters, 1e999, two separators refused; numIt never gives Infinity", parser)
        def headers():
            pg.evaluate(f"(() => {{ const p = {SPEC}; p.bg = 'w'; ctl(p); }})()")   # «background of a stretch»: the two time boxes of the spectrum header
            n = sweep(pg, "#dpanels .pnl")
            print("header fields", n)
            x = pg.locator("#dpanels .pnl [data-o=bw0]").first; x.focus(); x.press("ControlOrMeta+a"); x.press_sequentially("1,5"); x.press("Tab")
            wait_for_js(pg, f"{SPEC}.bw0 === 1.5", 5000)
        step("panel headers: every visible numeric field refuses garbage with a note, old value back, state unchanged", headers)
        def spec_params():
            pg.evaluate(f"{SPEC}.el.querySelector('[data-a=par]').click()"); wait_for_js(pg, "!!document.querySelector('.sp-pop [data-s=thr]')", 5000)
            sweep(pg, ".sp-pop")
            x = pg.locator(".sp-pop [data-s=thr]"); x.focus(); x.press("ControlOrMeta+a"); x.press_sequentially("12,5"); x.press("Tab")
            wait_for_js(pg, f"{SPEC}.thr === 13", 5000)
        step("spectrum parameters: garbage refused; «12,5» is taken", spec_params)
        def peak_table():
            pg.keyboard.press("Escape")
            pg.evaluate(f"{SPEC}.el.querySelector('[data-a=tabp]').click()"); wait_for_js(pg, "!!document.querySelector('.tp-pop')", 5000)
            sweep(pg, ".tp-pop")
            pg.keyboard.press("Escape")
        step("peak table: garbage refused", peak_table)
        def settings():
            pg.click("#np-set"); wait_for_js(pg, "!!document.querySelector('#uip-ppm')", 5000)
            sweep(pg, "#uipset")
            x = pg.locator("#uip-ppm"); x.focus(); x.press("ControlOrMeta+a"); x.press_sequentially("7,0"); x.press("Tab")
            wait_for_js(pg, "JSON.parse(localStorage.getItem('mzlab.prefs') || '{}').hrPpm === 7", 5000)
            pg.keyboard.press("Escape")
        step("Settings: garbage refused; «7,0» ppm is taken", settings)
        def requests():
            bad = [u for u in urls if any(s in u for s in ("NaN", "Infinity", "undefined"))]
            assert not bad, bad[:3]
        step("no request carried NaN, Infinity or undefined", requests)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
