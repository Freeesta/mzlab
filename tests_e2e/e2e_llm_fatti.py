"""The facts of the open files without a model (flow/lazy/flow-llm-facts.js, docs/agenti/llm.md «Fatti senza modello»): in the profile «ricerca» with the assistant off, three
synthetic files open, the Flow shows the «Files» card and the proposals of the parameters; the inspection and the proposal are the same bytes as `mzlab-cli llm inspect|params`
(tests/data/llm/fatti/); «Apply» is one entry of Undo; the readers of the assistant get the inspection (accepted, not skipped); axe and target sizes; no request leaves the site.
Run from the repo root: python3 tests_e2e/e2e_llm_fatti.py"""
import json, socket, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from lib import free_port, attendi, axe_path, stage; PORT = free_port()
AXE = axe_path()
TAB = '#nav button[data-v="flow"]'
FATTI = ["B_FullMass-t0.mzML", "B_MRM-t0.mzML", "C_IDA-t0.mzML"]   # crates/mzlab-cli/src/llm_facts_tests.rs
GOLD = ROOT / "tests" / "data" / "llm" / "fatti"
steps = []; errs = []; reqs = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:700]))

tmp = Path(tempfile.mkdtemp())
subprocess.run([sys.executable, str(ROOT / "tools" / "dati_sintetici.py"), str(tmp / "dati")], check=True, stdout=subprocess.DEVNULL)
FILES = [tmp / "dati" / f for f in FATTI]
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(tmp / "site")], check=True)
from lib_sito import add_wasm; add_wasm(tmp / "site")
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(tmp / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(2000):
    try: socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError: pass
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        def fresh(on=False, coarse=False):
            c = b.new_context(viewport={"width": 1400, "height": 1000}, has_touch=coarse, is_mobile=coarse); pg = c.new_page()
            pg.on("pageerror", lambda e: errs.append(str(e))); pg.on("request", lambda r: reqs.append(r.url))
            settings = json.dumps({"on": on, "provider": "anthropic", "model": "claude-sonnet-5-5", "level": "L1"})
            consent = json.dumps({"anthropic": {"classes": ["plan", "summary", "names"]}}) if on else "{}"
            pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');localStorage.setItem('mzlab.lang','en');localStorage.setItem('mzlab.llm',%s);localStorage.setItem('mzlab.llm.consenso',%s);}catch(_){}"
                               % (json.dumps(settings), json.dumps(consent)))
            pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
            stage(pg, FILES)
            # a file with two parts (MS1, MS2) has two rows: wait for every file
            attendi(pg, "new Set(E.files.map(f => String(f.file).split('#')[0])).size >= %d" % len(FATTI), 90000)
            pg.wait_for_selector(TAB, timeout=60000); pg.click(TAB); pg.wait_for_selector("#flx-root", timeout=30000)
            pg.wait_for_selector("#flx-facts .flx-facts-file", timeout=60000)
            return pg

        pg = fresh()
        def card():
            names = pg.eval_on_selector_all("#flx-facts .flx-facts-file h4", "e => e.map(x => x.textContent)")
            assert names == [f"f{i + 1}: {n}" for i, n in enumerate(FATTI)], names
            text = pg.inner_text("#flx-facts")
            for want in ("Polarity", "positive", "Resolution", "unit (mass_decimals:1)", "Acquisition", "mrm", "dda", "Isolation", "36 windows", "Role", "t0 (proposed)", "Proposed parameters"):
                assert want in text, want
            assert pg.locator("#flx-assistant").count() == 0, "the assistant is off: no button, no panel"
            assert pg.evaluate("!window.MZLAB_LR_MISS || !window.MZLAB_LR_MISS.length"), pg.evaluate("window.MZLAB_LR_MISS")
        step("assistant off: the «Files» card and the proposals come from Rust alone", card)

        def same_bytes():
            r = pg.evaluate("""async () => { const S = (() => { let n = document.querySelector('#flx-root'); while (n && !n.__flow) n = n.parentElement; return n.__flow; })();
              const f = await FLOW.llmFacts.facts(S, false); return [f.inspection, f.proposal]; }""")
            assert r[0] + "\n" == (GOLD / "ispezione.json").read_text(), r[0][:600]
            assert r[1] + "\n" == (GOLD / "proposta.json").read_text(), r[1][:600]
        step("the inspection and the proposal are the bytes of `mzlab-cli llm inspect|params`", same_bytes)

        def apply_undo():
            S = "(() => { let n = document.querySelector('#flx-root'); while (n && !n.__flow) n = n.parentElement; return n.__flow; })()"
            if pg.locator("#flx-facts-props button[aria-label^='Apply']").count() == 0:
                # an empty plan: a step the proposals name, added as the user would
                pg.evaluate(f"() => {{ const S = {S}; FLOW.addStep(S.plan, S.reg, 'features.detect'); S.refresh(null); }}")
                pg.click("#flx-facts button[aria-label='Read the open files again']")
                pg.wait_for_selector("#flx-facts-props button[aria-label^='Apply']", timeout=30000)
            # a proposal equal to the value of the plan changes nothing («Already set.»): the first one that changes it is one entry of Undo
            tried = []
            while pg.locator("#flx-facts-props button[aria-label^='Apply']").count():
                before = pg.evaluate(f"() => {{ const S = {S}; return [S.history.length, JSON.stringify(S.plan)]; }}")
                btn = pg.locator("#flx-facts-props button[aria-label^='Apply']").first
                tried.append(btn.get_attribute("aria-label")); btn.click()
                after = pg.evaluate(f"() => {{ const S = {S}; return [S.history.length, JSON.stringify(S.plan)]; }}")
                if after[1] != before[1]: break
            assert after[0] == before[0] + 1 and after[1] != before[1], (tried, before[0], after[0])
            pg.evaluate(f"() => {S}.undo()")
            assert pg.evaluate(f"() => JSON.stringify({S}.plan)") == before[1], "Undo gives the plan back"
            if pg.locator("#flx-facts-props button[aria-label^='Discard']").count():
                pg.locator("#flx-facts-props button[aria-label^='Discard']").first.click()
        step("«Apply» sets the proposed value in one entry of Undo; «Discard» leaves the plan", apply_undo)

        def a11y():
            for coarse in (False, True):
                q = pg if not coarse else fresh(coarse=True)
                need = 40 if coarse else 24
                small = q.evaluate("n => [...document.querySelectorAll('#flx-facts button, #flx-facts summary')].filter(e => e.offsetParent).map(e => [e.getAttribute('aria-label') || e.textContent, e.getBoundingClientRect()]).filter(x => x[1].width < n || x[1].height < n).map(x => x[0] + ' ' + Math.round(x[1].width) + 'x' + Math.round(x[1].height))", need)
                assert not small, (coarse, small)
                if AXE and not coarse:
                    q.add_script_tag(path=AXE)
                    v = q.evaluate("axe.run(document.querySelector('#flx-facts'), {resultTypes: ['violations']}).then(r => r.violations.filter(x => ['serious', 'critical'].includes(x.impact)).map(x => x.id + ': ' + x.nodes.length))")
                    assert not v, v
                if coarse: q.context.close()
        step("axe 0 serious, targets of 24 px (40 on touch)", a11y)
        pg.context.close()

        def readers():
            q = fresh(on=True)
            q.wait_for_selector("#flx-assistant", timeout=30000); attendi(q, "!!window.MZLAB_LLM_AGENT")
            route = json.dumps({"route": "inspect", "need_data": ["inspection"], "language": "en", "suspicious": False})
            ok = json.dumps({"schema": "lettura/1", "kind": "acquisition", "findings": [{"id": "F1", "topic": "polarity", "statement": "f1 is ESI+ full scan at unit resolution",
                             "evidence": ["ispezione.files.f1.polarity", "ispezione.files.f1.class"], "confidence": "high", "action": {"type": "none"}}], "questions": [], "numbers": []})
            r = q.evaluate("""async ([route, ok]) => {
              const S = (() => { let n = document.querySelector('#flx-root'); while (n && !n.__flow) n = n.parentElement; return n.__flow; })();
              const { AgentRun } = await import('/static/llm/llm-agente.js');
              // the client of the page with no network: the router and the reader answer as ready requests, the orchestrator closes the turn
              const answers = [[{ ev: 'text_delta', text: route }], [{ ev: 'text_delta', text: ok }]];
              window.MZLAB_LLM = { client: { needsKey: async () => false, sendRaw: async (p, m, req, sink) => { for (const e of answers.shift()) sink(e); },
                send: async (c, sink) => { for (const e of [{ ev: 'text_delta', text: 'f1 is full scan.' }, { ev: 'stop', reason: 'end' }]) sink(e); } } };
              const seen = [], inputs = {};
              const host = { show() {}, log() {}, sending() {}, done() {}, call: async () => ({ ok: true, content: {} }), ask: async () => false,
                readerInput: async role => { const v = await FLOW.llmFacts.readerInput(S, role); inputs[role] = v ? Object.keys(v) : null; return v; }, reader: i => seen.push([i.role, i.status]) };
              const run = new AgentRun(host); await run.open(true); await run.enableAssist(S.reg.steps);
              const out = await run.user('What are the open files?');
              const par = await FLOW.llmFacts.readerInput(S, 'parametri');
              return { out, seen, inputs, par: par ? Object.keys(par) : null, none: await FLOW.llmFacts.readerInput(S, 'interprete') };
            }""", [route, ok])
            assert r["out"] == "end", r
            assert r["seen"] == [["metadati", "accepted"]], r["seen"]
            assert r["inputs"] == {"metadati": ["ispezione/1"]}, r["inputs"]
            assert r["par"] == ["proposta-param/1"] and r["none"] is None, r
            q.context.close()
        step("the readers get the inspection of the page and are accepted, not skipped", readers)

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
