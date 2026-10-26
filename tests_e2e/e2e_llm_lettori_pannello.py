"""Readers on the assistant panel in Flow (docs/agenti/llm-lettori.md): the card shows each reader queued, running, then done / discarded / error / skipped / cancelled with a short outcome; the state
is in words (not only colour); the next batch opens a new card; targets of 24 px (40 on touch); axe. Driven through the display (FLOW.llmReaders) that the host of the panel uses.
No request leaves the site. Run from the repo root: python3 tests_e2e/e2e_llm_lettori_pannello.py"""
import json, socket, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from lib import free_port, attendi, axe_path; PORT = free_port()
TAB = '#nav button[data-v="flow"]'
AXE = axe_path()
steps = []; errs = []; reqs = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:600]))

tmp = Path(tempfile.mkdtemp())
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(tmp / "site")], check=True)
from lib_sito import add_wasm; add_wasm(tmp / "site")
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(tmp / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(2000):
    try: socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError: pass
# a display wired to the log of the panel, like the host of flow-llm.js
DRIVE = "() => { const rd = FLOW.llmReaders.make(c => document.getElementById('flx-llm-log').append(c)); window.__rd = rd; }"
def states(pg, card=-1):
    return pg.evaluate("i => { const cs = [...document.querySelectorAll('#flx-llm .flx-llm-readers')]; const c = cs.at(i); return c ? [...c.querySelectorAll('li')].map(l => [l.querySelector('.name').textContent, l.dataset.state, l.querySelector('.state').textContent, l.querySelector('.out').hidden ? '' : l.querySelector('.out').textContent]) : null; }", card)
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        def fresh(reduced=False, coarse=False):
            c = b.new_context(viewport={"width": 1500, "height": 1000}, reduced_motion="reduce" if reduced else "no-preference", has_touch=coarse, is_mobile=coarse); pg = c.new_page()
            pg.on("pageerror", lambda e: errs.append(str(e))); pg.on("request", lambda r: reqs.append(r.url))
            settings = json.dumps({"on": True, "provider": "anthropic", "model": "claude-sonnet-5-5", "level": "L1", "l2max": 5})
            pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');localStorage.setItem('mzlab.lang','en');localStorage.setItem('mzlab.profilo','ricerca');localStorage.setItem('mzlab.llm',%s);localStorage.setItem('mzlab.llm.consenso',%s);}catch(_){}"
                               % (json.dumps(settings), json.dumps(json.dumps({"anthropic": {"classes": ["plan", "summary", "names"]}}))))
            pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
            pg.wait_for_selector(TAB, timeout=60000); pg.click(TAB); pg.wait_for_selector("#flx-root", timeout=30000)
            pg.wait_for_selector("#flx-assistant", timeout=30000); attendi(pg, "!!FLOW.llmReaders")
            pg.click("#flx-assistant"); pg.evaluate(DRIVE)
            return pg

        def lifecycle():
            pg = fresh()
            pg.evaluate("() => { for (const r of ['metadati', 'cromatografia', 'parametri', 'interprete']) __rd.queued(r); }")
            assert [s[1] for s in states(pg)] == ["queued"] * 4, states(pg)
            pg.evaluate("() => __rd.stage('metadati', 'running')")
            assert states(pg)[0][1:3] == ["running", "▶ Running"], states(pg)
            pg.evaluate("() => __rd.verdict({role: 'metadati', status: 'accepted', summary: {instrument: 'QTRAP'}})")
            r = states(pg)[0]; assert r[1:3] == ["done", "✓ Done"] and "QTRAP" in r[3], r
            pg.evaluate("() => { __rd.stage('cromatografia', 'running'); __rd.verdict({role: 'cromatografia', status: 'dropped', problems: [{key: 'err.llm.module', params: {}}]}); }")
            r = states(pg)[1]; assert r[1] == "discarded" and r[2].endswith("Discarded") and r[3], r
            pg.evaluate("() => { __rd.stage('parametri', 'running'); __rd.verdict({role: 'parametri', status: 'failed', problems: [{key: 'err.llm.network', params: {reason: 'offline'}}]}); }")
            assert states(pg)[2][1] == "error", states(pg)
            pg.evaluate("() => { __rd.stage('interprete', 'cancelled'); }")
            assert states(pg)[3][1] == "cancelled", states(pg)
            assert pg.evaluate("document.querySelectorAll('#flx-llm .flx-llm-readers').length") == 1
            pg.evaluate("() => { __rd.queued('grafici'); __rd.verdict({role: 'grafici', status: 'skipped'}); }")
            assert pg.evaluate("document.querySelectorAll('#flx-llm .flx-llm-readers').length") == 2, "a new batch opens a new card"
            assert states(pg)[0][1:3] == ["skipped", "– Skipped"], states(pg)
            live = pg.evaluate("(document.getElementById('flx-llm-log') || {}).getAttribute('aria-live')"); assert live == "polite", live
            pg.context.close()
        step("states of the readers: queued, running, done, discarded, error, cancelled, skipped; a new batch, a new card", lifecycle)

        def a11y():
            for coarse in (False, True):
                pg = fresh(reduced=True, coarse=coarse)
                pg.evaluate("() => { __rd.queued('metadati'); __rd.queued('grafici'); __rd.verdict({role: 'metadati', status: 'accepted', summary: 'x'}); __rd.stage('grafici', 'running'); }")
                assert pg.get_attribute("#flx-llm .flx-llm-readers", "aria-label") == "Readers"
                assert not pg.evaluate("[...document.querySelectorAll('#flx-llm .flx-llm-readers *')].some(e => /(^|[^-\\w])#[0-9a-f]{3,6}\\b|rgb\\(/i.test(e.getAttribute('style') || ''))"), "no colour outside the variables"
                small = pg.evaluate("n => [...document.querySelectorAll('#flx-llm button, #flx-llm summary')].filter(e => e.offsetParent).map(e => [e.getAttribute('aria-label') || e.textContent, e.getBoundingClientRect()]).filter(x => x[1].width < n || x[1].height < n).map(x => x[0])", 40 if coarse else 24)
                assert not small, (coarse, small)
                if AXE and not coarse:
                    pg.add_script_tag(path=AXE)
                    v = pg.evaluate("axe.run(document.querySelector('#flx-llm .flx-llm-readers'), {resultTypes: ['violations']}).then(r => r.violations.filter(x => ['serious', 'critical'].includes(x.impact)).map(x => x.id + ': ' + x.nodes.length))")
                    assert not v, v
                pg.context.close()
        step("targets of 24 px (40 on touch), axe, no colour outside the variables", a11y)

        def outside():
            ext = [u for u in reqs if not u.startswith(f"http://127.0.0.1:{PORT}/") and not u.startswith(("data:", "blob:"))]
            assert not ext, ext[:5]
            assert not errs, errs[:3]
        step("no request left the site and no script error", outside)
        b.close()
finally:
    srv.terminate()
bad = [s for s in steps if s[1] != "ok"]
for n, r in steps: print(("PASS " if r == "ok" else "FAIL ") + n + ("" if r == "ok" else " :: " + r))
sys.exit(1 if bad else 0)
