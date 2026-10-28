"""Precursor tree panel of an MSn infusion: every MSn with a superscript (copied as plain text it reads «MS3»), the highlight of a clicked row in the same
task, the arrows and Enter, the table redrawn at once when Level / Tolerance / Energies / Spectrum change, and the four controls in labelled groups that wrap
cleanly. Synthetic file (lib_albero), no real data."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
from lib_albero import big_nodes
from TP_Mine.tests.msn_synth import write_msn
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_infusione_albero.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("infalb")
NODES = len(big_nodes()) - 1
F = write_msn(D / "MSn_synth_t02.mzML", big_nodes(), n_scans=int(os.environ.get("ALBERO_SCANS", "800")))
r = Run(port=8979, wd="/tmp/wd_infalb")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1200})
        load(pg, [F], 6000, 1)
        attendi(pg, "E.panels.some(p=>p.type==='spec' && p._a && p._a.data && p.filt && p.filt.startsWith('msn:'))", 120000)
        drawer(pg, "alb"); pg.wait_for_selector("#inf-tree tbody tr", timeout=30000)
        def sup():
            t = pg.evaluate("""[...document.querySelectorAll('#inf-tree tbody tr')].map(tr => { const c = tr.cells[1]; return [c.textContent, !!c.querySelector('sup'), c.querySelector('[aria-label]')?.getAttribute('aria-label')] })""")
            assert len(t) == NODES + 1, len(t)
            assert all(x[1] for x in t), t[:3]
            assert sorted(set(x[0] for x in t)) == ["MS1", "MS2", "MS3", "MS4"], t[:4]
            assert {x[2] for x in t} == {"MS 1", "MS 2", "MS 3", "MS 4"}, t
            assert pg.evaluate("document.querySelector('#inf-tree thead th:nth-child(2) sup')?.textContent") == "n"
            assert "⁴" in pg.evaluate("[...document.querySelectorAll('#inf-lv option')].map(o => o.textContent).join()")
        step("every MSn of the tree has its superscript and reads «MS3» as text", sup)
        def selection():
            r0 = pg.evaluate("""() => { const out = []; const trs = [...document.querySelectorAll('#inf-tree tbody tr[data-id]')];
              for (const tr of trs.slice(1, 6)) { const t0 = performance.now(); tr.click();
                const on = document.querySelector('#inf-tree tbody tr[aria-selected=true]'); out.push([on === tr, document.querySelectorAll('#inf-tree tbody tr[aria-selected=true]').length, performance.now() - t0]); }
              return out; }""")
            assert all(a and n == 1 and t < 16 for a, n, t in r0), r0
        step("a click highlights the row in the same task (under one frame)", selection)
        def keys():
            pg.focus("#inf-tree tbody tr[tabindex='0']")
            before = pg.evaluate("document.activeElement.dataset.id")
            pg.keyboard.press("ArrowDown"); now = pg.evaluate("document.activeElement.dataset.id"); assert now != before, (before, now)
            assert pg.evaluate("getComputedStyle(document.activeElement).outlineStyle") != "none"
            pg.keyboard.press("Enter")
            assert pg.evaluate("document.querySelector('#inf-tree tbody tr[aria-selected=true]').dataset.id") == now
            attendi(pg, f"E.panels.every(p=>!p.filt || p.filt==='msn:{now}')", 20000)
        step("arrows move the focus, Enter selects the row (visible focus)", keys)
        def controls():
            m = pg.evaluate("""async () => { const ms = async (id, v) => { const s = document.getElementById(id), n0 = document.querySelectorAll('#inf-tree tbody tr').length, t0 = performance.now();
                s.value = v; s.dispatchEvent(new Event('change')); await new Promise(r => requestAnimationFrame(r));
                return [n0, document.querySelectorAll('#inf-tree tbody tr').length, performance.now() - t0, document.getElementById(id) === s]; };
              return [await ms('inf-lv', '3'), await ms('inf-lv', '0'), await ms('inf-sum', '1'), await ms('inf-sum', '0')]; }""")
            assert all(x[3] for x in m), m              # the controls stay in place (focus and open lists survive)
            assert m[0][1] < m[0][0] and m[1][1] == m[1][0] + (m[0][0] - m[0][1]), m
            assert all(x[2] < 50 for x in m), m
            for sid, v in (("inf-ce", "0"), ("inf-ce", "1"), ("inf-tol", "0.01"), ("inf-tol", "nom")):
                pg.select_option("#" + sid, v); attendi(pg, "document.querySelectorAll('#inf-tree tbody tr').length > 1 && !document.querySelector('#hri-body > .muted')", 10000)
                assert 1 < pg.locator("#inf-tree tbody tr").count() <= NODES + 1, sid
        step("Level, Tolerance, Energies and Spectrum redraw the table at once", controls)
        def copy():
            pg.evaluate("window.__clip = null; navigator.clipboard.writeText = async t => { window.__clip = t; }")
            pg.click("#inf-copy"); attendi(pg, "window.__clip !== null", 10000)
            lines = [l for l in pg.evaluate("window.__clip").split("\n") if not l.startswith("# ")]
            n = pg.locator("#inf-tree tbody tr").count()
            assert len(lines) == n + 1 and all(len(l.split("\t")) == 6 for l in lines), (n, lines[:3])
            assert lines[1].split("\t")[1] == "MS1" and any(l.split("\t")[1] == "MS3" for l in lines), lines[:4]
        step("«Copy table» puts the shown rows on the clipboard as tab-separated text (MS3, not a superscript)", copy)
        def sort():
            col = lambda: pg.evaluate("[...document.querySelectorAll('#inf-tree tbody tr')].slice(1).map(t => parseFloat(t.cells[4].textContent))")
            ids = lambda: pg.evaluate("[...document.querySelectorAll('#inf-tree tbody tr')].map(t => t.dataset.id)")
            tree = ids(); pg.click("#inf-sort"); attendi(pg, "document.querySelector('#inf-tree th[aria-sort=descending]')", 10000)
            d = col(); assert d == sorted(d, reverse=True) and sorted(ids()) == sorted(tree) and ids()[0] == tree[0], d
            assert pg.evaluate("document.querySelector('#inf-tree tbody tr[data-id]:not([data-id=\"\"])').getAttribute('aria-level')") == "2"
            pg.click("#inf-sort"); attendi(pg, "document.querySelector('#inf-tree th[aria-sort=ascending]')", 10000)
            a = col(); assert a == sorted(a), a
            pg.click("#inf-sort"); attendi(pg, "document.querySelector('#inf-tree th[aria-sort=none]')", 10000)
            assert ids() == tree, "the third click gives back the tree"
        step("the Intensity column sorts the rows (highest, lowest, back to the tree)", sort)
        def graph():
            pg.click("#inf-graph"); attendi(pg, "BARRA.curTab === 'gra'", 10000)
            drawer(pg, "alb"); pg.wait_for_selector("#inf-tree tbody tr", timeout=30000)
        def late_graph():
            # the graph module arrives AFTER the student went back to the tree: it must not repaint the body
            pg.evaluate("""() => { window.__lateGraph = false; const o = window.carica; window.carica = async n => { const r = await o(n); if (/infusione-grafo/.test(n)) { await new Promise(f => setTimeout(f, 1200)); window.__lateGraph = true; } return r; }; }""")
            pg.evaluate("BARRA.setTab('gra', {noFocus: true, instant: true})"); attendi(pg, "BARRA.curTab === 'gra'", 10000)
            pg.evaluate("BARRA.setTab('alb', {noFocus: true, instant: true})"); attendi(pg, "BARRA.curTab === 'alb' && !!document.querySelector('#inf-tree tbody tr')", 10000)
            attendi(pg, "window.__lateGraph === true", 10000); frames(pg, 5)
            assert pg.evaluate("!!document.querySelector('#inf-tree tbody tr') && !document.querySelector('#ig-open')"), "the late graph repainted the tree tab"
        step("«Fragmentation graph» in the tree controls opens the graph tab", graph)
        step("a graph that arrives late does not repaint the tree tab", late_graph)
        def labels():
            t = pg.evaluate("""[...document.querySelectorAll('.inf-f')].map(l => [l.querySelector('span').textContent, l.title, l.querySelector('select').offsetHeight])""")
            assert [x[0] for x in t] == ["Level", "Mass tolerance", "Collision energies", "Spectrum"] or pg.evaluate("I18N.lang") != "en", t
            assert all(len(x[1]) > 20 for x in t) and len({x[2] for x in t}) == 1, t
        step("four labelled controls of the same height, each with its tooltip", labels)
        for w, theme in ((900, "light"), (900, "dark"), (480, "light"), (480, "dark")):
            def fit(w=w, theme=theme):
                pg.set_viewport_size({"width": w, "height": 1000}); pg.evaluate(f"document.getElementById('dfiles').style.width='{620 if w >= 900 else 300}px'"); pg.evaluate(f"document.documentElement.setAttribute('data-theme','{theme}')"); pg.evaluate("new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))")
                drawer(pg, "alb"); pg.wait_for_selector(".inf-ctl", timeout=10000); attendi(pg, "document.getElementById('hri-body').clientWidth > 200", 10000); pg.evaluate("new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))")
                g = pg.evaluate("""() => { const c = document.querySelector('.inf-ctl'), b = document.getElementById('hri-body'); const tops = [...c.querySelectorAll('.inf-f')].map(x => Math.round(x.getBoundingClientRect().top));
                  return {tops, over: b.scrollWidth > b.clientWidth + 1, cw: c.getBoundingClientRect().width} }""")
                if os.environ.get("ALBERO_SHOTS"): pg.locator("#hrinfo").screenshot(path=f"{os.environ['ALBERO_SHOTS']}/albero_{w}_{theme}.png")
                assert not g["over"], g
                if w >= 900: assert len(set(g["tops"])) <= 2, g
            step(f"controls fit at {w} px, {theme} theme", fit)
        errs = [e for e in r.errs if e[0] in ("pageerror",)]
        step("no page errors", lambda: (_ for _ in ()).throw(AssertionError(errs[:3])) if errs else None)
        step("no request left unanswered by the engine", lambda: (_ for _ in ()).throw(AssertionError(pg.evaluate("window.MZLAB_LR_MISS"))) if pg.evaluate("(window.MZLAB_LR_MISS||[]).length") else None)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
