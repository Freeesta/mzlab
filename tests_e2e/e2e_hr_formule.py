"""HR_PANNELLO 2: «Composizione elementare» as a compact panel (Xcalibur style): ready defaults, the table copies a row, «Applica» writes the formula under the m/z of the peak,
right click on a peak → «Genera formula», the expanded form (FreeStyle style) with the periodic table; the panel texts are not clipped at 1280/1440/1680 px."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_hr_formule.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
D = synth("hrformule")
r = Run(port=8994, wd="/tmp/wd_hrformule")
SHOTS = os.path.join(os.path.dirname(os.path.abspath(__file__)), "shots"); os.makedirs(SHOTS, exist_ok=True)
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1400})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2); ready(pg)
        pg.evaluate("localStorage.removeItem('mzlab.comp')")
        def sp_id():
            return pg.evaluate("(()=>{const s=E.panels.find(q=>q.type==='spec'&&q._a&&q._a.hrp);return s?s.id:null})()")
        def peak_xy():
            pg.evaluate(f"E.panels.find(q=>q.id=={sp_id()}).el.scrollIntoView({{block:'center'}})"); pg.wait_for_timeout(300)
            return pg.evaluate(f"(()=>{{const s=E.panels.find(q=>q.id=={sp_id()}),a=s._a,d=a.data[0].d;let b=0;d.y.forEach((v,i)=>{{if(v>d.y[b])b=i}});const r=s.cv.getBoundingClientRect();return {{x:r.left+a.X(d.mz[b]),y:r.top+a.Y(d.y[b])+4,m:d.mz[b]}}}})()")
        def defaults():
            pg.evaluate("setActive(E.panels.find(p=>p.type==='spec'&&p._a&&p._a.hrp))")
            drawer(pg, "cmp"); pg.wait_for_selector("#cx-out table, #cx-out .muted", timeout=20000); print("top peak", pg.input_value("#cx-mz"), pg.inner_text("#cx-out")[:80].replace("\n", " | "))
            pg.fill("#cx-mz", "317.1639"); pg.wait_for_selector("#cx-out tr[data-i]", timeout=20000)
            head = pg.inner_text("#cx-out thead").lower(); assert "idx" in head and "formula" in head and "rdb" in head and "ppm" in head, head
            els = pg.inner_text(".cx-els"); assert "Isotopo" in els and "Eq. DB" in els and "Massa" in els, els
            got = pg.evaluate("[...document.querySelectorAll('.cx-els tbody tr td:first-child')].map(r=>r.innerText.replace(/\\s/g,'')).join(',')"); assert got == "12C,1H,14N,16O,32S", got
            assert pg.evaluate("document.querySelector('#cx-els').value") == "C:0-13,H:0-25,N:0-4,O:0-3,S:0-1"
            assert not pg.is_visible("#hri-ttl")
        step("the panel calculates at once with the default elements; table Idx/Formula/RDB/Δ ppm, elements in use with isotope, min, max, DB eq., mass", defaults)
        def candidates():
            pg.fill("#cx-mz", "317.1639"); pg.wait_for_function("document.querySelector('#cx-out').innerText.replace(/\\s/g,'').includes('C13H25N4O3S')", timeout=15000)
            row = pg.inner_text("#cx-out tr[data-i='0']").replace("\t", " "); assert "C13H25N4O3S" in row.replace(" ", ""), row
        step("317.1639 gives C13H25N4O3S with the default limits", candidates)
        def copy():
            pg.context.grant_permissions(["clipboard-read", "clipboard-write"], origin=f"http://127.0.0.1:{r.port}")
            pg.click("#cx-out tr[data-i='0']"); pg.wait_for_timeout(300)
            t = pg.evaluate("navigator.clipboard.readText()"); assert t.startswith("C13H25N4O3S\t"), repr(t)
            assert pg.get_attribute("#cx-out tr[data-i='0']", "aria-selected") == "true"
        step("a click on a row selects and copies it", copy)
        def edit():
            pg.fill(".cx-els input[data-f=max][data-i='0']", "5"); pg.wait_for_timeout(800)
            assert pg.input_value("#cx-els") if False else pg.evaluate("document.querySelector('#cx-els').value").startswith("C:0-5,"), "the table edits the text"
            pg.wait_for_function("document.querySelector('#cx-out').innerText.includes('Nessuna formula')", timeout=15000)
            pg.fill(".cx-els input[data-f=max][data-i='0']", "13"); pg.wait_for_function("document.querySelector('#cx-out').innerText.replace(/\\s/g,'').includes('C13H25N4O3S')", timeout=15000)
        step("editing max in the table recalculates (no candidate, then back)", edit)
        def generate():
            pg.evaluate("BARRA.setTab('hdr',{noFocus:true,instant:true})"); pg.wait_for_timeout(300)
            xy = peak_xy(); pg.mouse.click(xy["x"], xy["y"], button="right"); pg.wait_for_timeout(400)
            assert "Genera formula (m/z" in pg.inner_text("#ctx")
            pg.click("#ctx >> text=Genera formula"); pg.wait_for_selector("#cx-out table, #cx-out .muted", timeout=20000)
            assert abs(float(pg.input_value("#cx-mz")) - xy["m"]) < 1e-3, (pg.input_value("#cx-mz"), xy["m"])
            assert pg.evaluate("BARRA.curTab") == "cmp"
        step("right click on a peak → «Genera formula» opens the panel with that m/z", generate)
        def apply():
            sid = sp_id(); xy = peak_xy()
            pg.evaluate("document.querySelector('#cx-els').value='C:0-30,H:0-60,N:0-6,O:0-8,S:0-2'; document.querySelector('#cx-els').dispatchEvent(new Event('input')); document.querySelector('#cx-mz').dispatchEvent(new Event('input'))")
            pg.wait_for_selector("#cx-out tr[data-i]", timeout=20000)
            top = float(pg.input_value("#cx-mz")); calc = pg.evaluate("document.querySelector('#cx-out tr[data-i=\"0\"]').innerText"); print("first candidate:", calc.replace("\n", " ").replace("\t", " "))
            pg.evaluate(f"setActive(E.panels.find(q=>q.id=={sid}))"); pg.click("#cx-apply"); pg.wait_for_timeout(500)
            fm = pg.evaluate(f"E.panels.find(q=>q.id=={sid}).fm"); assert fm and abs(fm[0]["m"] - xy["m"]) < 1e-4 and fm[0]["text"], fm
            xy = peak_xy(); pg.mouse.click(xy["x"], xy["y"], button="right"); pg.wait_for_timeout(300)
            assert "Togli le formule scritte" in pg.inner_text("#ctx"); pg.keyboard.press("Escape")
        step("«Applica» writes the formula (candidate) on the spectrum under the peak", apply)
        def expand():
            pg.click("#cx-exp"); pg.wait_for_selector("#cmp-els", timeout=5000)
            t = pg.inner_text("#bigbody")
            for w in ("Impostazioni di predizione", "Confronto del profilo isotopico", "Confronto dei frammenti", "Tavola periodica", "Soglia di intensità", "Tolleranza di intensità", "Adattamento minimo", "Algoritmo di centroide", "Normalizzazione", "elementi rappresentativi"):
                assert w in t, w
            assert pg.input_value("#cmp-ithr") == "0.1" and pg.input_value("#cmp-sint") == "30"
            pg.click("#bigbody summary:has-text('Tavola periodica')"); pg.click("#cmp-ptb button[data-s='P']"); pg.wait_for_timeout(200)
            assert "P:" in pg.input_value("#cmp-els")
            pg.click("#cmp-go"); pg.wait_for_selector("#cmp-out table", timeout=20000)
            pg.evaluate("document.querySelector('#bigdlg').close()")
        step("expanded form: prediction settings, pattern and fragments matching, periodic table", expand)
        def clip():
            for w in (1280, 1440, 1680):
                pg.set_viewport_size({"width": w, "height": 1000}); pg.wait_for_timeout(300)
                drawer(pg, "cmp"); pg.fill("#cx-mz", "317.1639"); pg.wait_for_selector("#cx-out tr[data-i]", timeout=20000); ready(pg)
                bad = pg.evaluate("""() => { const R = document.querySelector('#dfiles').getBoundingClientRect(), out = [];
                  document.querySelectorAll('#dfiles *').forEach(e => { if (!e.offsetParent || e.closest('#cx-out,.cx-els')) return; if (![...e.childNodes].some(n => n.nodeType === 3 && n.textContent.trim())) return;
                    const cs = getComputedStyle(e), b = e.getBoundingClientRect(); if ((e.scrollWidth > e.clientWidth + 1 && cs.overflowX !== 'visible') || b.right > R.right + 1) out.push([e.id || e.className || e.tagName, e.textContent.trim().slice(0, 30)]); }); return out; }""")
                assert not bad, f"at {w}: {bad[:3]}"
                pg.locator("#dfiles").screenshot(path=os.path.join(SHOTS, f"hr_formule_{w}.png"))
        step("no clipped text in the panel at 1280, 1440 and 1680 px", clip)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
