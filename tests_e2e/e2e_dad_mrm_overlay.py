"""The DAD on the MRM panel, on the built site with the Rust engine and the synthetic MRM `.wiff` of tools/wiff_dad_mrm.py (no laboratory data): the right-click
menu of the MRM panel has «UV spectrum at this RT», «TWC» and «Allinea»; a double click opens the UV spectrum at that time linked to the panel (its band at
274 nm at compound A), moved back by the DAD → MS delay when «Allinea» is on; the «TWC» check draws the TWC on a right axis, with the hover and cursor
readings and its column in the data table, survives a reload, and a file with no DAD offers none of it. axe on the menu and the table. WebKit-safe
(mouse and keyboard only). Run from the repo root: python3 tests_e2e/e2e_dad_mrm_overlay.py"""
# covers: tools/wiff_dad_mrm.py, mzlab/web/dad-mrm.js
import json, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

D = Path(tempfile.mkdtemp(prefix="dadmrmov_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_dad_mrm.py"), str(D)], check=True, capture_output=True)
TRUE = json.loads((D / "dadmrm.json").read_text())
AXE = axe_path()
MRM = "E.panels.find(q => q.type === 'mrm')"
UV = "E.panels.find(q => q.type === 'spec' && q.uv)"
RUN = """(sel) => axe.run(document.querySelector(sel) || document, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}})
  .then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.slice(0, 3).map(n => n.target.join(' ')).join(' | ')]))"""
def axe_ok(pg, sel):
    if not AXE: raise AssertionError("axe-core not available (pip install axe-playwright-python)")
    if not pg.evaluate("!!window.axe"): pg.add_script_tag(path=AXE)
    v = pg.evaluate(RUN, sel); assert not v, v
def at(pg, rt):
    """the point of the MRM plot at time rt, half way up"""
    return pg.evaluate(f"(() => {{ const p = {MRM}, b = p.cv.getBoundingClientRect(), a = p._a; return [b.x + a.X({rt}), b.y + M.t + (a.H - M.t - M.b) / 2]; }})()")
def menu(pg, rt=1.0):
    pg.evaluate(f"{MRM}.el.scrollIntoView({{block: 'center'}})"); frames(pg, 3)
    x, y = at(pg, rt); pg.mouse.click(x, y, button="right"); pg.wait_for_selector("#ctx:not([hidden])")
    return pg.evaluate("[...document.querySelectorAll('#ctx [role^=menuitem]')].map(e => [e.getAttribute('role'), e.textContent.replace(/^✓\s*/, '').trim(), e.getAttribute('aria-checked')])")
def item(pg, label):
    pg.click(f"#ctx [role^=menuitem]:has-text('{label}')"); ready(pg)
def load(pg, name):
    pg.set_input_files("#pick", [str(D / name), str(D / (name + ".scan"))])
    wait_for_js(pg, "document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=90000); pg.click("text=Carica dati"); ready(pg)
    wait_for_js(pg, "E.files.length >= 1", timeout=120000); open_kind(pg, "mrm")
    wait_for_js(pg, f"!!{MRM} && !!{MRM}._a", timeout=60000)

r = Run(port=8884, wd="/tmp/wd_dadmrmov")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, "dadmrm.wiff")
        T = pg.evaluate("({ uv: I18N.t('ctx.uv.atRt'), twc: I18N.t('ctx.mrm.twc'), al: I18N.t('disp.pdaAlign'), col: I18N.t('tdati.col.twc') })")
        def entries():
            wait_for_js(pg, "!!AZIONI.voce('mrm.twc')", timeout=30000)
            labs = menu(pg); pg.keyboard.press("Escape")
            assert any(l[1] == T["uv"] for l in labs), labs
            assert ["menuitemcheckbox", T["twc"], "false"] in labs and any(l[0] == "menuitemcheckbox" and l[1] == T["al"] for l in labs), labs
        step("the MRM panel of a file with a DAD: «UV spectrum at this RT», «TWC» and «Allinea» in its menu", entries)
        def menu_axe():
            menu(pg); axe_ok(pg, "#ctx"); pg.keyboard.press("Escape")
        step("axe: the menu of the MRM panel", menu_axe)
        def dbl():
            x, y = at(pg, TRUE["a"]["rt"]); pg.mouse.dblclick(x, y)
            wait_for_js(pg, f"!!{UV} && !!{UV}._a && !!{UV}._a.uvd", timeout=60000)
            q = pg.evaluate(f"(() => {{ const s = {UV}, d = s._a.uvd; let b = 0; d.y.forEach((v, i) => {{ if (v > d.y[b]) b = i; }}); return {{ link: s.link === {MRM}.id, nm: d.nm[b], rt: d.rt, k: s.k }}; }})()")
            assert q["link"] and abs(q["nm"] - 274) <= 2 and abs(q["rt"] - TRUE["a"]["rt"]) < 0.05 and q["k"] == 0, q
        step("a double click at compound A opens the UV spectrum there, linked to the MRM panel (band at 274 nm)", dbl)
        def align():
            pg.evaluate(f"{MRM}.pdaDelay = 0.5"); menu(pg); item(pg, T["al"])
            assert pg.evaluate(f"{MRM}.pdaAlign") is True and pg.evaluate(f"DISP.pdaShift({MRM})") == 0.5
            x, y = at(pg, TRUE["a"]["rt"] + 0.5); pg.mouse.dblclick(x, y)
            wait_for_js(pg, f"Math.abs({UV}.r0 - {TRUE['a']['rt']}) < 0.05", timeout=30000)
            menu(pg); item(pg, T["al"]); assert not pg.evaluate(f"{MRM}.pdaAlign")
            pg.evaluate(f"{MRM}.pdaDelay = null")
        step("«Allinea»: the spectrum is read at the MRM time minus the DAD → MS delay", align)
        def overlay():
            menu(pg); item(pg, T["twc"])
            wait_for_js(pg, f"{MRM}.overlay === 'twc' && {MRM}._a && {MRM}._a.twc === true", timeout=60000)
            assert pg.evaluate(f"{MRM}.cv._needR") > 0
            labs = menu(pg); pg.keyboard.press("Escape"); assert ["menuitemcheckbox", T["twc"], "true"] in labs, labs
        step("the «TWC» check draws the TWC on a right axis of the MRM panel", overlay)
        def readings():
            d = pg.evaluate("(async () => { const j = await J('api/chrom?k=0&kind=pda&level=1'); let b = 0; j.y.forEach((v, i) => { if (v > j.y[b]) b = i; }); return { rt: j.rt[b], y: j.y[b] }; })()")
            assert abs(d["rt"] - TRUE["a"]["rt"]) < 0.05, d
            v = pg.evaluate(f"(() => {{ const p = {MRM}; p.cur = {d['rt']}; cursorLine(p); return p.ch.getAttribute('aria-valuetext'); }})()")
            assert "TWC" in v and f"{d['y']:.1f}" in v, (v, d)
            h = pg.evaluate(f"(() => {{ const p = {MRM}, a = p._a, top = {d['y']} * 1.08, py = a.H - M.b - {d['y']} / top * (a.H - M.t - M.b); return GRADIENTE.hover(p, {d['rt']}, py); }})()")
            assert "TWC" in h and "mAU" in h, h
        step("the cursor and the hover read the TWC of the engine at its apex", readings)
        def table():
            pg.evaluate(f"{MRM}.cv.focus()"); pg.keyboard.press("t"); pg.wait_for_selector("#tdati[open]")
            head = pg.evaluate("[...document.querySelectorAll('#tdati thead th')].map(e => e.textContent)")
            rows = pg.evaluate("[...document.querySelectorAll('#tdati tbody tr')].map(r => [...r.cells].map(c => c.textContent))")
            assert head[-1] == T["col"] and len(rows) > 10 and sum(1 for x in rows if x[-1] != "") > 10, (head, rows[:3])
            axe_ok(pg, "#tdati")
            pg.keyboard.press("Escape")
        step("the data table of the MRM panel has the TWC column (and passes axe)", table)
        def reload():
            pg.evaluate("uiSave()"); pg.reload(); ready(pg)
            wait_for_js(pg, f"!!{MRM} && {MRM}.overlay === 'twc' && !!{MRM}._a && {MRM}._a.twc === true", timeout=90000)
        step("the TWC overlay comes back after a reload", reload)
        step("the engine answered every request", lambda: (lambda m: m and (_ for _ in ()).throw(AssertionError(m)))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        step("no page error", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()
    r2 = Run(port=8885, wd="/tmp/wd_dadmrmov2")
    with sync_playwright() as p:
        pg = r2.page(p)
        load(pg, "nodad.wiff")
        def none():
            labs = menu(pg); pg.keyboard.press("Escape")
            T2 = pg.evaluate("[I18N.t('ctx.uv.atRt'), I18N.t('ctx.mrm.twc')]")
            assert not any(l[1] in T2 for l in labs), labs
        step("an MRM file with no DAD: no UV spectrum and no TWC in the menu", none)
    r2.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
