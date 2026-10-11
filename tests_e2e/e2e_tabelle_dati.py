"""Data tables of the charts (tdati.js): the «Dati» button and the T key on a focused chart open a real <table> (headers with scope) whose values are those
of the API; D reads the description through the polite live region; every chart canvas has a role, a label and a description."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_tabelle_dati.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8866, wd="/tmp/wd_tabelle")
CH = "E.panels.find(p=>p.type==='chrom'&&p.tab==='full'&&p.kind==='tic')"
SP = "E.panels.find(p=>p.type==='spec'&&p.tab==='full')"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg); pg.wait_for_timeout(1500)
        def tbl():
            return pg.evaluate("(()=>{const t=document.querySelector('#tdati table');return t?{cols:[...t.querySelectorAll('thead th')].map(h=>[h.textContent,h.getAttribute('scope')]),rows:[...t.querySelectorAll('tbody tr')].map(r=>[...r.children].map(c=>c.textContent)),cap:t.querySelector('caption')?.textContent}:null})()")
        def close(): pg.keyboard.press("Escape"); pg.wait_for_timeout(200); assert pg.locator("#tdati").count() == 0
        def attrs():
            for sel in (CH, SP):
                v = pg.evaluate(f"(()=>{{const c={sel}.cv;return [c.getAttribute('role'),c.getAttribute('aria-label'),c.tabIndex,!!document.getElementById(c.getAttribute('aria-describedby')||'x')?.textContent]}})()")
                assert v[0] == "img" and v[1] and v[2] == 0 and v[3] is True, v
        step("each chart canvas has role, label, focus and a description", attrs)
        def chrom_table():
            pg.evaluate(f"{CH}.cv.focus()"); pg.keyboard.press("t"); pg.wait_for_selector("#tdati[open]")
            t = tbl(); assert t["cols"][0][1] == "col" and len(t["rows"]) > 20 and t["cap"], t["cols"]
            api = pg.evaluate("(async()=>await (await fetch('api/chrom?k=0&kind=tic&level=1')).json())()")
            first = next(r for r in t["rows"] if "TIC" in r[0] or True)
            for row in t["rows"][:40:7]:
                rt, y = float(row[1]), float(row[2])
                i = min(range(len(api["rt"])), key=lambda j: abs(api["rt"][j] - rt))
                assert abs(api["rt"][i] - rt) < 1e-3 and abs(api["y"][i] - y) <= max(1.0, abs(api["y"][i]) * 0.2), (row, api["rt"][i], api["y"][i])
            close()
        step("T on the chromatogram: a table whose RT and intensities are those of the API", chrom_table)
        def spec_table():
            pg.evaluate(f"{SP}.cv.focus()"); pg.keyboard.press("t"); pg.wait_for_selector("#tdati[open]")
            t = tbl(); assert [c[0] for c in t["cols"]][1:3] == ["m/z", "Intensità"] and len(t["rows"]) >= 1, t["cols"]
            p0 = pg.evaluate(f"({{r0:{SP}.r0,r1:{SP}.r1,k:{SP}.k}})")
            api = pg.evaluate("(async(p)=>await (await fetch(`api/spectrum?k=${p.k}&rt0=${p.r0}&rt1=${p.r1}&level=1&precursor=&merge=1`)).json())(%s)" % __import__("json").dumps(p0))
            top = max(range(len(api["y"])), key=lambda j: api["y"][j])
            best = max(t["rows"], key=lambda r: float(r[2]))
            assert abs(float(best[1]) - api["mz"][top]) < 0.51, (best, api["mz"][top])
            close()
        step("T on the spectrum: the most intense row is the highest peak of the API", spec_table)
        def button():
            pg.click(".pnl.chrom [data-a=tab]"); pg.wait_for_selector("#tdati[open]"); assert tbl()["rows"]; close()
            assert pg.evaluate("document.activeElement.dataset.a") == "tab", "focus returns to the button"
        step("the Dati button opens the same table and gives the focus back", button)
        def live():
            pg.evaluate(f"{CH}.cv.focus()"); pg.keyboard.press("d"); pg.wait_for_timeout(300)
            t = pg.evaluate("document.getElementById('tdati-live').textContent"); assert "Cromatogramma" in t and "tracce" in t, t
            assert pg.evaluate("document.getElementById('tdati-live').getAttribute('aria-live')") == "polite"
        step("D reads the description through the polite live region", live)
        def copy():
            pg.evaluate(f"{CH}.cv.focus()"); pg.keyboard.press("t"); pg.wait_for_selector("#tdati[open]")
            pg.evaluate("navigator.clipboard.writeText=async t=>{window.__clip=t}"); pg.click("#tdati [data-td=c]"); pg.wait_for_timeout(300)
            c = pg.evaluate("window.__clip"); assert c.split("\n")[0].split("\t")[1] == "RT (min)" and "\t" in c.split("\n")[1], c[:80]
            close()
        step("Copy puts the table in the clipboard as tab separated text", copy)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
