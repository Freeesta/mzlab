"""HR_PANELS 3a: «Opzioni di visualizzazione» (right click on a chart → panel of the drawer, extendable per kind of chart; palette command) and the isotope simulation
with several adducts (concentration) and a charge distribution. The decimals field and the ▾ button of the spectrum are gone from the bars in HR."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_hr_display.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
D = synth("hrdisplay")
r = Run(port=8996, wd="/tmp/wd_hrdisplay")
SHOTS = os.path.join(os.path.dirname(os.path.abspath(__file__)), "shots"); os.makedirs(SHOTS, exist_ok=True)
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1400})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2); ready(pg)
        def spec_id(): return pg.evaluate("(()=>{const s=E.panels.find(q=>q.type==='spec'&&q._a&&q._a.hrp);return s?s.id:null})()")
        def at(kind):
            sid = spec_id() if kind == "spec" else pg.evaluate("E.panels.find(q=>q.type==='chrom').id")
            pg.evaluate(f"E.panels.find(q=>q.id=={sid}).el.scrollIntoView({{block:'center'}})"); pg.wait_for_timeout(300)
            return pg.evaluate(f"(()=>{{const s=E.panels.find(q=>q.id=={sid}),r=s.cv.getBoundingClientRect();return {{x:r.left+r.width*0.5,y:r.top+r.height*0.5}}}})()")
        def menu_spec():
            assert pg.evaluate("getComputedStyle(document.querySelector('.pnl.spec [data-a=par]')).display") == "none", "the ▾ button is gone in HR"
            xy = at("spec"); pg.mouse.click(xy["x"], xy["y"], button="right"); pg.wait_for_timeout(300)
            assert "Opzioni di visualizzazione" in pg.inner_text("#ctx")
            pg.click("#ctx >> text=Opzioni di visualizzazione"); pg.wait_for_selector(".dsp", timeout=5000)
            t = pg.inner_text(".dsp"); assert "Decimali" in t and "Etichette" in t and "al massimo" in t, t
            assert pg.evaluate("BARRA.curTab") == "disp" and pg.evaluate("document.querySelector('#sb-title').textContent") == "Opzioni di visualizzazione"
        step("right click on a spectrum → Opzioni di visualizzazione: decimals, labels (threshold, maximum)", menu_spec)
        def change():
            pg.fill("#dsp-2", "3"); pg.press("#dsp-2", "Tab"); pg.wait_for_timeout(500)
            assert pg.evaluate("E.panels.find(q=>q.type==='spec'&&q._a&&q._a.hrp).nlab") == 3
            pg.select_option("#dsp-0", "5"); pg.wait_for_timeout(600); assert pg.evaluate("UIP.hrDec") == 5; pg.select_option("#dsp-0", "4")
        step("changing an option redraws the chart", change)
        def menu_chrom():
            xy = at("chrom"); pg.mouse.click(xy["x"], xy["y"], button="right"); pg.wait_for_timeout(300)
            pg.click("#ctx >> text=Opzioni di visualizzazione"); pg.wait_for_selector(".dsp", timeout=5000)
            assert "cromatogramma" in pg.inner_text(".dsp") and "Normalizza" in pg.inner_text(".dsp")
        step("right click on a chromatogram → its own list", menu_chrom)
        def extend():
            pg.evaluate("DISP.register('chrom', {id:'zz', label:'disp.norm100', kind:'check', get:p=>!!p.zz, set:(p,v)=>{p.zz=v}})")
            pg.evaluate("BANCO.renderInfo()"); pg.wait_for_function("document.querySelectorAll('.dsp input[type=checkbox]').length==2", timeout=5000)
        step("extendable: a package registers an option for a kind of chart", extend)
        def palette():
            pg.keyboard.press("Control+k"); pg.wait_for_selector("#paletta", timeout=5000); pg.keyboard.type("opzioni di vis"); pg.wait_for_timeout(300)
            assert "Opzioni di visualizzazione" in pg.inner_text("#pal-l")
            pg.keyboard.press("Enter"); pg.wait_for_timeout(500); assert pg.evaluate("BARRA.curTab") == "disp"
        step("«Display options» is a command of the palette", palette)
        def iso():
            pg.evaluate("setActive(E.panels.find(q=>q.type==='spec'&&q._a&&q._a.hrp))")
            drawer(pg, "iso"); pg.wait_for_selector("#hri-f", timeout=5000); ready(pg)
            t = pg.inner_text("#hri-body"); assert "Distribuzione di carica" in t and "Conc. (%)" in t
            pg.fill("#hri-f", "C13H24N4O3S"); pg.click("#hri-addad"); pg.wait_for_timeout(200)
            assert pg.locator(".hri-ad").count() == 2
            pg.click("#hri-go"); pg.wait_for_timeout(1500)
            o = pg.evaluate("E.panels.find(q=>q.type==='spec'&&q._a&&q._a.hrp).iso"); assert o and o["adds"] and len(o["adds"]) == 2 and o["adds"][1]["c"] == 10, o
            n = pg.evaluate("HR.isoPat(E.panels.find(q=>q.type==='spec'&&q._a&&q._a.hrp).iso, 0).filter(r=>r.tag).length"); assert n > 3
        step("isotope simulation with two adducts (concentration 100 % and 10 %)", iso)
        def charge():
            pg.fill("#hri-z0", "3"); pg.fill("#hri-hw", "2"); pg.click("#hri-go"); pg.wait_for_timeout(1500)
            o = pg.evaluate("E.panels.find(q=>q.type==='spec'&&q._a&&q._a.hrp).iso"); assert o["cd"] == {"z0": 3, "hw": 2}, o
            tags = pg.evaluate("[...new Set(HR.isoPat(E.panels.find(q=>q.type==='spec'&&q._a&&q._a.hrp).iso, 0).map(r=>r.tag))]"); assert "3+" in tags and "2+" in tags, tags
            pg.locator("#dfiles").screenshot(path=os.path.join(SHOTS, "hr_isotopi.png"))
        step("charge distribution: several charge states weighted around the most abundant", charge)
        def clip():
            for w in (1280, 1440, 1680):
                pg.set_viewport_size({"width": w, "height": 1000}); pg.wait_for_timeout(300)
                for tb in ("disp", "iso"):
                    drawer(pg, tb); ready(pg)
                    bad = pg.evaluate("""() => { const R = document.querySelector('#dfiles').getBoundingClientRect(), out = [];
                      document.querySelectorAll('#dfiles *').forEach(e => { if (!e.offsetParent || e.closest('[hidden]')) return; if (![...e.childNodes].some(n => n.nodeType === 3 && n.textContent.trim())) return;
                        const cs = getComputedStyle(e), b = e.getBoundingClientRect(); if ((e.scrollWidth > e.clientWidth + 1 && cs.overflowX !== 'visible') || b.right > R.right + 1) out.push([e.id || e.className || e.tagName, e.textContent.trim().slice(0, 30)]); }); return out; }""")
                    assert not bad, f"{tb} at {w}: {bad[:3]}"
                    pg.locator("#dfiles").screenshot(path=os.path.join(SHOTS, f"hr_{tb}_{w}.png"))
        step("no clipped text in the two panels at 1280, 1440 and 1680 px", clip)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
