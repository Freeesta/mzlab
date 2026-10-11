"""Pratica «Δm ed ESI±» (pratica-tp.html, games lc-dm and lc-esi): one exercise per level played to the end, the three aids (no answer in them),
keyboard only, dark theme (readable feedback, no fixed white background), phone note, axe-core (skipped when not available)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))

def axe_path():
    p = os.environ.get("AXE_JS")
    if p and os.path.exists(p): return p
    try:
        import axe_playwright_python
        q = os.path.join(os.path.dirname(axe_playwright_python.__file__), "axe.min.js")
        return q if os.path.exists(q) else None
    except ImportError:
        return None

r = Run(port=8969, wd="/tmp/wdprtp")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        url = f"http://127.0.0.1:{r.port}/static/teoria/pratica-tp.html"
        pg.goto(url); pg.wait_for_selector("#g1 .opts button"); pg.wait_for_selector("#g2 .st"); pg.wait_for_selector("#g3 .cin-svg"); pg.wait_for_selector("#g4 .st")
        def dm_level(lv):
            def f():
                pg.select_option("#g1 .lv", str(lv)); pg.wait_for_timeout(100)
                e = pg.evaluate("GIOCHI_LC.last.dm"); assert e["level"] == lv
                idx = [o["id"] for o in e["options"]].index(e["truthId"])
                pg.click(f"#g1 .opts >> nth=0 >> button >> nth={idx}")
                assert "Giusto" in pg.inner_text("#g1 .st >> nth=0 >> .fb"), "step 1"
                pg.click("#g1 [data-n=" + ("odd" if e["product"]["nParityOdd"] else "even") + "]")
                pg.fill("#g1 .no", str(e["product"]["nO"])); pg.click("#g1 .ck")
                t = pg.inner_text("#g1 .wk"); assert t.count("Giusto") == 3, t[-400:]
                assert f"{PLAYED[0]} problemi · media 100" in pg.inner_text("#g1 .pts"), pg.inner_text("#g1 .pts")
                PLAYED[0] += 1
            return f
        PLAYED = [1]
        for lv in (1, 2, 3): step(f"H1 level {lv}: one exercise played to the end (right answers score 100)", dm_level(lv))
        def aids():
            pg.click("#g1 .nw"); e = pg.evaluate("GIOCHI_LC.last.dm")
            for i in range(3): pg.click("#g1 .hp")
            t = pg.inner_text("#g1 .hpt"); assert "Aiuto 3" in t and pg.is_disabled("#g1 .hp")
            plain = lambda s: __import__("re").sub(r"<[^>]+>", "", s)
            assert plain(e["truthLabel"]) not in t and e["product"]["f"] not in t, "an aid gives the answer"
            assert pg.get_attribute("#g1 .hpt", "class").find("fb") >= 0 and pg.evaluate("document.querySelector('#g1 .hpt').getAttribute('aria-live')") == "polite"
        step("H1 aids: three levels, polite live region, no answer", aids)
        def esi_level(lv):
            def f():
                pg.select_option("#g2 .lv", str(lv)); pg.wait_for_timeout(100)
                e = pg.evaluate("GIOCHI_LC.last.esi"); assert e["level"] == lv
                par = lambda c: "odd" if c["nOdd"] else "even"
                if e["task"] == "M":
                    c = e["compounds"][0]; pg.fill("#g2 .vm", f"{c['M']:.3f}"); pg.select_option("#g2 .vn", par(c))
                elif e["task"] == "pair":
                    for s in e["pos"]:
                        c = [x for x in e["compounds"] if x["id"] == s["comp"]][0]; n = [x for x in e["neg"] if x["comp"] == c["id"]][0]
                        pg.select_option(f"#g2 .pr[data-p={s['id']}]", n["id"]); pg.select_option(f"#g2 .pn[data-p={s['id']}]", par(c))
                else:
                    c = e["compounds"][0]
                    for s in e["pos"] + e["neg"]: pg.select_option(f"#g2 .ad[data-p={s['id']}]", s["adduct"])
                    pg.fill("#g2 .vm", f"{c['M']:.3f}"); pg.select_option("#g2 .vn", par(c))
                pg.click("#g2 .ck"); assert "Tutto giusto" in pg.inner_text("#g2 .wk"), pg.inner_text("#g2 .fb")
                assert "problemi · media 100" in pg.inner_text("#g2 .pts")
            return f
        for lv in (1, 2, 3): step(f"H2 level {lv}: one exercise played to the end", esi_level(lv))
        def cin_level(lv):
            def f():
                pg.select_option("#g3 .lv", str(lv)); pg.wait_for_timeout(100)
                e = pg.evaluate("GIOCHI_LC.last.cin"); assert e["level"] == lv
                assert pg.locator("#g3 .cin-svg").count() == 1 and pg.locator("#g3 table.cin-tab tr").count() == len(e["features"]) + 2, "chart or data table missing"
                for x in e["features"]: pg.select_option(f"#g3 .c1s[data-id={x['id']}]", x["truth"])
                pg.click("#g3 .ck1"); assert "Tutto giusto" in pg.inner_text("#g3 .st >> nth=0 >> .fb")
                for x in e["features"]:
                    if x["role"]: pg.select_option(f"#g3 .c2s[data-id={x['id']}]", x["role"])
                pg.click("#g3 .ck2")
                k = [x for x in e["features"] if x["ms2Times"]][0]
                pg.select_option("#g3 .c3p", k["id"]); pg.select_option("#g3 .c3t", str(k["ms2Times"][0])); pg.click("#g3 .ck3")
                t = pg.inner_text("#g3 .wk"); assert t.count("Tutto giusto") == 2 and "Giusto." in t, t[-500:]
                assert "problemi · media 100" in pg.inner_text("#g3 .pts")
            return f
        for lv in (1, 2, 3): step(f"H3 level {lv}: lc-cin played to the end (chart plus data table)", cin_level(lv))
        def cin_aids():
            pg.click("#g3 .nw"); e = pg.evaluate("GIOCHI_LC.last.cin")
            for i in range(3): pg.click("#g3 .hp")
            t = pg.inner_text("#g3 .hpt"); assert "Aiuto 3" in t and pg.is_disabled("#g3 .hp")
            assert pg.evaluate("document.querySelector('#g3 .hpt').getAttribute('aria-live')") == "polite"
        step("H3 aids: three levels, polite live region", cin_aids)
        def cin_targets():
            n = pg.evaluate("[...document.querySelectorAll('#g3 select, #g3 button, #g3 input')].filter(e=>{const r=e.getBoundingClientRect();return r.width>0&&(r.height<24||r.width<24)}).length"); assert n == 0, n
        step("H3 controls are at least 24x24 px", cin_targets)
        def ms2_level(lv):
            def f():
                pg.select_option("#g4 .lv", str(lv)); pg.wait_for_timeout(100)
                e = pg.evaluate("GIOCHI_LC.last.ms2"); assert e["level"] == lv
                assert pg.locator("#g4 table.cin-tab tr").count() == len(e["ions"]) + 1, "MS2 table missing"
                pg.click("#g4 .hp"); assert "Aiuto 1" in pg.inner_text("#g4 .hpt")
                pg.click(f"#g4 .opts button[data-id={e['bestIds'][0]}]"); assert "Coerente" in pg.inner_text("#g4 .st >> nth=0 >> .fb")
                n = pg.evaluate("(()=>{const e=GIOCHI_LC.last.ms2;return LCMSN.infoOf(e,e.ions.find(x=>x.id===e.probeId))})()")
                pg.fill("#g4 .nn", str(n)); pg.click("#g4 .ck2")
                assert "Giusto." in pg.inner_text("#g4 .st >> nth=1 >> .fb") and "problemi · media 100" in pg.inner_text("#g4 .pts")
            return f
        for lv in (1, 2, 3): step(f"H7 level {lv}: lc-ms2 played to the end", ms2_level(lv))
        def ms2_targets():
            n = pg.evaluate("[...document.querySelectorAll('#g4 select, #g4 button, #g4 input')].filter(e=>{const r=e.getBoundingClientRect();return r.width>0&&(r.height<24||r.width<24)}).length"); assert n == 0, n
        step("H7 controls are at least 24x24 px", ms2_targets)
        def keyboard():
            pg.click("#g1 .nw"); pg.focus("#g1 .opts button >> nth=0"); pg.keyboard.press("Enter")
            assert pg.evaluate("!document.querySelectorAll('#g1 .st')[1].hasAttribute('inert')"), "step 2 not open"
            assert pg.evaluate("document.activeElement.dataset.n") is not None, "focus does not move to step 2"
            pg.keyboard.press("Space"); assert pg.evaluate("document.activeElement.className") == "no", "focus does not move to step 3"
            pg.keyboard.type("3"); pg.keyboard.press("Enter")
            n = pg.evaluate("document.querySelectorAll('#g1 .fb.ok, #g1 .fb.no').length"); assert n == 3, n
        step("keyboard only: options, parity, Enter in the O field", keyboard)
        def live():
            n = pg.evaluate("[...document.querySelectorAll('.wk .fb')].filter(e=>e.getAttribute('aria-live')!=='polite').length"); assert n == 0, n
        step("every .fb is a polite live region", live)
        def dark():
            pg.evaluate("localStorage.setItem('qqq.prefs', JSON.stringify({theme:'dark'}))"); pg.goto(url); pg.wait_for_selector("#g1 .opts button")
            pg.click("#g1 .opts button >> nth=0")
            lum = """(c)=>{const m=c.match(/[\\d.]+/g).map(Number);const f=v=>{v/=255;return v<=.03928?v/12.92:Math.pow((v+.055)/1.055,2.4)};return .2126*f(m[0])+.7152*f(m[1])+.0722*f(m[2])}"""
            bgs = pg.evaluate("(" + lum + ")(getComputedStyle(document.body).backgroundColor)"); assert bgs < 0.1, bgs
            ratio = pg.evaluate("""(()=>{const L=%s;const e=document.querySelector('#g1 .st .fb:not(:empty)');const cs=getComputedStyle(e);const a=L(cs.color),b=L(cs.backgroundColor);return (Math.max(a,b)+.05)/(Math.min(a,b)+.05)})()""" % lum)
            assert ratio >= 4.5, ratio
            white = pg.evaluate("[...document.querySelectorAll('.wk, .gbar, .wk table, .wk select, .wk input')].filter(e=>getComputedStyle(e).backgroundColor==='rgb(255, 255, 255)').map(e=>e.tagName+'.'+e.className)"); assert not white, white
        step("dark theme: dark page, feedback contrast >= 4.5, no fixed white background", dark)
        def phone():
            ctx = r.b.new_context(viewport={"width": 390, "height": 800}, has_touch=True, is_mobile=True, locale="it-IT",
                                  user_agent="Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 Mobile/15E148")
            q = ctx.new_page(); q.goto(url); q.wait_for_timeout(800)
            assert q.locator(".phex").count() == 1, "phone note missing"; ctx.close()
        step("phone: the exercises are not offered, same note as the other pages", phone)
        AXE = axe_path()
        if AXE:
            def axe():
                pg.evaluate("localStorage.removeItem('qqq.prefs')"); pg.goto(url); pg.wait_for_selector("#g1 .opts button"); pg.click("#g1 .opts button >> nth=0"); pg.add_script_tag(path=AXE)
                v = pg.evaluate("axe.run('main', {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => v.id + ': ' + v.nodes.length))")
                assert not v, v
            step("axe-core: no serious or critical violation", axe)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
