"""The SIM wording and controls of the Data view on the built site with the Rust engine (synthetic SIM .wiff pair of tools/wiff_sim.py, no laboratory data): the engine types every file «lc-sim», the
experiment cell says «SIM · N ions» (not «no data»), the side list group, the panel title, the toolbar button and the channel picker speak of SIM ions, overlaid traces take
the colour of their sample and a dash per ion, the chromatogram menu holds the «Extract ion ›» submenu (check mark, «All», click / Ctrl / Shift rules) in place
of XIC, and the «Area» toggle fills the traces (on by default). Needs the site and the .wasm (python3 tools/prepara_sito.py). Run from the repo root: python3 tests_e2e/e2e_sim_ui.py"""
import json, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

D = Path(tempfile.mkdtemp(prefix="sim_ui_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_sim.py"), str(D)], check=True, capture_output=True)
TRUE = json.loads((D / "sim.json").read_text())
N = len(TRUE["ioni"])
r = Run(port=8875, wd="/tmp/wd_sim_ui")
def need(c, m): c or (_ for _ in ()).throw(AssertionError(m))
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [D / "sim.wiff", D / "sim.wiff.scan"]); pg.click("text=Carica dati"); ready(pg); wait_for_js(pg, "E.files.length >= 2 && E.files.every(f => f.sim > 0)", timeout=120000)
        step("every file counts its SIM ions", lambda: need(pg.evaluate("E.files.every(f => f.sim === %d)" % N), pg.evaluate("E.files.map(f => f.sim)")))
        step("every file is typed SIM by the engine (a SIM file is neither «empty» nor «UV only»)", lambda: need(pg.evaluate("Promise.all(E.files.map(f => CANALI.canali(f.k).then(j => j.tipo.proposto)))") == ["lc-sim"] * pg.evaluate("E.files.length"), pg.evaluate("Promise.all(E.files.map(f => CANALI.canali(f.k).then(j => j.tipo.proposto)))")))
        def cell():
            t = pg.evaluate("ST.files.map(f => expCell(f)).join('|')")
            need("no data" not in t and "nessun dato" not in t and ("SIM" in t) and (" %d " % N in t), t)
        step("the experiment cell says SIM and the number of ions", cell)
        open_first(pg)
        step("the side list group is «SIM», not «MRM»", lambda: need(pg.evaluate("[...document.querySelectorAll('#flst .fgh')].map(e => e.textContent).join('|')").find("SIM") >= 0 and "MRM" not in pg.evaluate("[...document.querySelectorAll('#flst .fgh')].map(e => e.textContent).join('|')"), "group"))
        step("the toolbar button speaks of SIM ions", lambda: need("SIM" in pg.inner_text("#np-mrm"), pg.inner_text("#np-mrm")))
        pg.click("#np-mrm"); wait_for_js(pg, "E.panels.some(p => p.type === 'mrm' && p._a)", timeout=60000)
        step("the panel title says SIM ions", lambda: need(pg.evaluate("E.panels.find(p => p.type === 'mrm').title") == "@sim", pg.evaluate("E.panels.find(p => p.type === 'mrm').title")))
        def colours():
            # both samples in view, two ions plotted (the panel opens on the first file and one ion)
            pg.evaluate("(() => { E.files.forEach(f => { f.vis = true; }); const p = E.panels.find(p => p.type === 'mrm'); p.tr = SIMUI.ions(p).slice(0, 2).map(x => x.key); renderFileList(); draw(p); })()")
            wait_for_js(pg, "(() => { const a = E.panels.find(p => p.type === 'mrm')._a; return a && new Set(a.sr.map(s => s.k)).size === 2 && new Set(a.sr.map(s => s.ion)).size === 2; })()", timeout=30000)
            s = pg.evaluate("E.panels.find(p => p.type === 'mrm')._a.sr.map(s => [s.ion, s.color, s.k, JSON.stringify(s.dash)])")
            square = pg.evaluate("Object.fromEntries(E.files.map(f => [f.k, getComputedStyle(document.querySelector(`#flst .fl .nm[data-k=\"${f.k}\"]`).closest('.fl').querySelector(':scope > i')).backgroundColor]))")
            css = lambda c: pg.evaluate("(c => { const e = document.createElement('i'); e.style.background = c; document.body.appendChild(e); const v = getComputedStyle(e).backgroundColor; e.remove(); return v; })(%s)" % json.dumps(c))
            by_ion, by_k = {}, {}
            for ion, col, k, dash in s: by_ion.setdefault(ion, set()).add(col); by_k.setdefault(k, []).append((col, dash))
            need(len(by_k) >= 2, by_k)
            need(all(len(v) == len(by_k) for v in by_ion.values()), "two samples on one ion share a colour: %s" % by_ion)
            for k, v in by_k.items():
                need(len({c for c, _ in v}) == 1, "one sample, several colours: %s" % v)
                need(css(v[0][0]) == square[str(k)], "trace %s vs file square %s" % (css(v[0][0]), square[str(k)]))
                need(len({d for _, d in v}) == len(v), "two ions of one sample share a dash: %s" % v)
            leg = pg.evaluate("[...E.panels.find(p => p.type === 'mrm').leg.querySelectorAll('.tg')].map(e => e.textContent)")
            names = pg.evaluate("E.files.map(f => f.label)")
            need(leg and all(any(x.startswith(n) for n in names) for x in leg), "legend does not lead with the sample: %s" % leg)
            tip = pg.evaluate("(() => { const p = E.panels.find(p => p.type === 'mrm'); const d = document.createElement('div'); d.innerHTML = p._a.hov(p._a.X ? p._a.X((p._a.x0 + p._a.x1) / 2) : 300, 50).html; return [...d.querySelectorAll('i')].map(i => i.style.background); })()")
            need(tip and set(map(css, tip)) <= {css(c) for _, c, _, _ in s}, "tooltip colours %s" % tip)
            pg.evaluate("(() => { E.files.forEach((f, i) => { f.vis = i === 0; }); const p = E.panels.find(p => p.type === 'mrm'); p.tr = ''; renderFileList(); draw(p); })()")
        step("SIM overlay: colour = the sample (its file-list square), dash = the ion; legend and tooltip agree", colours)
        def zoombar():
            pg.evaluate("(() => { const p = E.panels.find(p => p.type === 'mrm'), [f0, f1] = p._a.full; zoomTo(p, f0 + (f1 - f0) / 4, f1 - (f1 - f0) / 4); })()")
            wait_for_js(pg, "(() => { const z = E.panels.find(p => p.type === 'mrm').el.querySelector('.zbar'); return !!z && !z.hidden; })()", timeout=20000)
            z = pg.evaluate("(() => { const z = E.panels.find(p => p.type === 'mrm').el.querySelector('.zbar'); return [z.getAttribute('role'), z.getAttribute('aria-label'), z.title]; })()")
            need(z[0] == "img" and z[1] and z[1] == z[2] and "RT" in z[1] and "min" in z[1], z)
            pg.evaluate("(() => { const p = E.panels.find(p => p.type === 'mrm'); p.zoom = null; draw(p); })()")
            wait_for_js(pg, "E.panels.find(p => p.type === 'mrm').el.querySelector('.zbar').hidden", timeout=20000)
        step("zoomed: the range is in the zoom bar's title and aria-label (no text on the chart), gone at full view", zoombar)
        step("«Area» is on by default and the traces are filled", lambda: need(pg.evaluate("E.panels.find(p => p.type === 'mrm').area !== false") and pg.evaluate("document.querySelector('[data-a=area]').getAttribute('aria-pressed')") == "true", "area"))
        def toggle():
            pg.click("[data-a=area]"); need(pg.evaluate("E.panels.find(p => p.type === 'mrm').area") is False, "off")
            pg.click("[data-a=area]"); need(pg.evaluate("E.panels.find(p => p.type === 'mrm').area") is True, "on")
        step("the Area button turns the fill off and on", toggle)
        def picker():
            pg.click(".can-b, .canb"); pg.wait_for_selector(".can-row", state="visible")
            rows = pg.evaluate("[...document.querySelectorAll('.can-pop')].map(e => e.querySelectorAll('.can-row').length)")
            need(rows and all(r == N for r in rows), rows)
        step("the channel picker lists the SIM ions", picker)
        PP = "E.panels.find(p => p.type === 'mrm' && p.el && p.el.isConnected)"
        Q = PP + ".el"
        tr = lambda: pg.evaluate("JSON.stringify(%s.tr)" % PP)
        def short_labels():
            t = pg.evaluate("[...%s.querySelectorAll('.can-row .can-et')].map(e => e.textContent)" % Q)
            need(len(t) == N and all(not any(w in x for w in ("ms", "SIM", "«")) for x in t), t)
            mixed = len({i["polarita"] for i in TRUE["ioni"]}) > 1                  # the polarity is printed only when the file holds both
            need(all(x.endswith(("(+)", "(−)")) for x in t) if mixed else not any("(" in x for x in t), t)
            need(pg.evaluate("[...%s.querySelectorAll('.can-hd span')].length" % Q) == 1, "extra columns")
            need(pg.evaluate("[...%s.querySelectorAll('.can-row')].every(e => /ms/.test(e.title) && /\\d+ (punti|points)/.test(e.title))" % Q), "details not in the hover text")
        step("the picker names each ion by its m/z alone; dwell, mode and points go to the hover text", short_labels)
        def single():
            pg.evaluate("%s.querySelectorAll('.can-row .can-et')[2].click()" % Q); frames(pg, 2)
            need(len(json.loads(tr())) == 1, tr())
            need(pg.evaluate("[...%s.querySelectorAll('.can-row')].filter(e => e.getAttribute('aria-selected') === 'true').length" % Q) == 1, "ticks")
            wait_for_js(pg, "%s._a && new Set(%s._a.sr.map(s => s.ion)).size === 1" % (PP, PP), timeout=30000)
            need(":" in pg.evaluate("%s.querySelector('.can-bt').textContent" % Q), "button does not name the ion")
        step("a click on an ion plots that ion alone", single)
        def multi():
            pg.evaluate("%s.querySelectorAll('.can-row .can-hit')[0].click()" % Q); frames(pg, 2)
            need(len(json.loads(tr())) == 2, tr())
            pg.evaluate("%s.querySelectorAll('.can-row .can-hit')[0].click()" % Q); frames(pg, 2)
            need(len(json.loads(tr())) == 1, tr())
            pg.evaluate("%s.querySelectorAll('.can-row .can-hit')[2].click()" % Q); frames(pg, 2)
            need(len(json.loads(tr())) == 1, "the last ion was unticked: " + tr())
            sz = pg.evaluate("(() => { const r = %s.querySelector('.can-hit').getBoundingClientRect(); return [r.width, r.height]; })()" % Q)
            need(sz[0] >= 24 and sz[1] >= 24, sz)
        step("the box adds or removes an ion (24 px target) and the last one stays", multi)
        def keys():
            pg.evaluate("(() => { const r = %s.querySelectorAll('.can-row')[0]; r.setAttribute('tabindex', '0'); r.focus(); })()" % Q)
            pg.keyboard.press(" "); frames(pg, 2); need(len(json.loads(tr())) == 2, tr())
            pg.keyboard.press("ArrowDown"); pg.keyboard.press("Enter"); frames(pg, 2); need(len(json.loads(tr())) == 1, tr())
            need(pg.evaluate("%s.querySelector('[role=tree]').getAttribute('aria-describedby')" % Q), "no hint")
            pg.evaluate("%s.querySelector('.can-all').click()" % Q); frames(pg, 2)
            need(tr() == '""', tr())
            need(pg.evaluate("document.activeElement.classList.contains('can-all') && !!document.activeElement.getAttribute('aria-label')"), "focus")
        step("Space adds an ion, Enter plots it alone, «All» plots every ion again", keys)
        pg.keyboard.press("Escape")
        def submenu():
            box = pg.locator("canvas").first.bounding_box()
            pg.mouse.click(box["x"] + box["width"] / 2, box["y"] + box["height"] / 2, button="right")
            pg.wait_for_selector("#ctx:not([hidden])")
            need(pg.evaluate("[...document.querySelectorAll('#ctx div[role=menuitem]')].some(e => /Extract ion|Estrai ione/.test(e.textContent))"), "entry")
            need(not pg.evaluate("[...document.querySelectorAll('#ctx div')].some(e => e.dataset.azione === 'chrom.xic')"), "XIC still there")
            pg.hover("#ctx .hsub"); need(pg.evaluate("document.querySelectorAll('#ctx .ctxsub:not([hidden]) [role=menuitemcheckbox]').length") == N + 1, "rows")
            need(pg.evaluate("document.querySelectorAll('#ctx .ctxsub .sw').length") == 0, "an ion swatch: colour belongs to the sample")
            pg.locator("#ctx .ctxsub [role=menuitemcheckbox]").nth(2).click()
            need(pg.evaluate("E.panels.find(p => p.type === 'mrm').tr.length") == 1, pg.evaluate("E.panels.find(p => p.type === 'mrm').tr"))
        step("the chromatogram menu extracts one ion with a click", submenu)
        def ctrl_shift():
            for mod in (None,):
                box = pg.locator("canvas").first.bounding_box()
                pg.mouse.click(box["x"] + box["width"] / 2, box["y"] + box["height"] / 2, button="right"); pg.hover("#ctx .hsub")
            pg.locator("#ctx .ctxsub [role=menuitemcheckbox]").nth(3).click(modifiers=["Control"])
            need(len(pg.evaluate("E.panels.find(p => p.type === 'mrm').tr")) == 2, "ctrl")
            pg.locator("#ctx .ctxsub [role=menuitemcheckbox]").nth(1).click(modifiers=["Shift"])
            need(len(pg.evaluate("E.panels.find(p => p.type === 'mrm').tr")) >= 2, "shift")
            pg.locator("#ctx .ctxsub [role=menuitemcheckbox]").nth(0).click()
            need(pg.evaluate("E.panels.find(p => p.type === 'mrm').tr") == "", "all")
        step("Ctrl adds an ion, Shift takes a range, All clears the selection", ctrl_shift)
        def helpline():
            pg.click("#np-closeall"); wait_for_js(pg, "!E.panels.some(p => p.el)", timeout=20000)
            wait_for_js(pg, "(() => { const h = document.querySelector('#simhelp'); return !!h && !h.hidden; })()", timeout=20000)
            need(pg.evaluate("document.querySelector('#simhelp').textContent === I18N.t('canali.sim.help') && !document.querySelector('#simhelp button')"), pg.inner_text("#simhelp"))
        step("with no graph the help line says how to add one and integrate", helpline)
        step("no page error", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
