"""Block B: clean interface: one "?" only, every icon-only button has a label, short load screen, neutral palette names."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_pulito.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
UNLABELLED = """()=>[...document.querySelectorAll('button,[role=button]')].filter(b=>b.offsetParent&&!b.closest('dialog:not([open])')&&!(b.innerText||'').trim().replace(/[\\u200b]/g,'')&&!b.title&&!b.getAttribute('aria-label')).map(b=>b.id||b.className||b.outerHTML.slice(0,60))"""
r = Run(port=8877, wd="/tmp/wd77")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_MS2-t15"), mz("B_MRM-t0")]); pg.wait_for_timeout(1200)
        def start():
            assert pg.locator(".hq:visible").count() == 1, pg.locator(".hq:visible").count()
            u = pg.evaluate(UNLABELLED); assert not u, u
        step("loading screen: one ?, no unlabelled button", start)
        pg.click("text=Carica dati"); ready(pg)
        def data():
            for t in ("full", "ms2", "mrm"):
                pg.evaluate(f"setTab('{t}',true)"); pg.wait_for_timeout(1800)
                assert pg.locator(".hq:visible").count() == 1, (t, pg.locator(".hq:visible").count())
                u = pg.evaluate(UNLABELLED); assert not u, (t, u)
        step("Dati (3 tabs): one ?, every button without text has a label", data)
        def tabs():
            tt = pg.evaluate("[...document.querySelectorAll('#flst .fgh[data-gt]')].map(b=>b.dataset.tiph||b.title)"); assert all(len(x) > 20 and "Q1" in x for x in tt), tt
        step("the tabs say what the experiment is (hover)", tabs)
        def guide():
            pg.click("button.hq[data-help=header]"); pg.wait_for_timeout(300)
            t = pg.inner_text("#helppop"); assert "Guida" not in t and "Aprire i dati" not in t, t[:200]
            t = r.teoria(pg); assert all(w in t for w in ["Aprire i dati", "Lavorare con i grafici", "Full Scan", "MRM"]), t[:200]
            pg.keyboard.press("Escape")
        step("the general ? is the guide, by topic", guide)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
