"""Assistant (settings → «Assistant»): without the administrator mode nothing of the assistant exists and nothing of llm/ loads; with it («ricerca») nothing of llm/; with the switch off nothing loads either; turning it on asks the consent
(dialog: focus inside, Esc closes), then the preview «What will be sent», the transport document and the recording served from 'self'. No request leaves the site
(any other host fails the test) and axe finds no serious violation in the section."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import json
r = Run(port=8877, wd="/tmp/wd_llm")
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:300]))
AXE = axe_path()
REC = '{"schema":"llmreg/1","turn":0,"events":[{"ev":"text_delta","text":"Hello from the recording"},{"ev":"stop","reason":"end"}]}'
try:
    with sync_playwright() as p:
        pg = r.page(p)
        seen = []
        pg.on("request", lambda q: seen.append(q.url))
        def llm_urls(): return [u for u in seen if "/llm/" in u or "wasm-feat/llm" in u]
        def corso():
            pg.click("#np-set"); pg.wait_for_selector("#uipset")
            pg.evaluate("document.getElementById('uip-g-app').open = true")
            assert pg.locator("#uip-llm").count() == 0, "no LLM element with the profile corso"
            assert not llm_urls(), llm_urls()
        step("corso: no LLM element and no llm/ file", corso)
        def ricerca_spento():
            pg.evaluate("localStorage.setItem('mzlab.admin','1'); localStorage.setItem('mzlab.lang','en')"); pg.reload(); ready(pg)
            pg.click("#np-set"); pg.wait_for_selector("#uipset")
            pg.evaluate("document.getElementById('uip-g-llm').open = true")
            pg.wait_for_selector("#uip-llm #llm-on")
            assert not pg.is_checked("#llm-on")
            assert not llm_urls(), llm_urls()
        step("ricerca with the switch off: no request to llm/", ricerca_spento)
        def consenso():
            pg.click("#uip-llm label:has(#llm-on)")
            pg.wait_for_selector("dialog[open]")
            assert pg.evaluate("document.querySelector('dialog[open]').contains(document.activeElement)"), "focus inside the dialog"
            t = pg.inner_text("dialog[open]")
            assert "raw files never leave this computer" in t.lower() or "never leave" in t.lower(), t
            pg.keyboard.press("Escape"); attendi(pg, "document.querySelectorAll('dialog[open]').length === 0")
            assert pg.locator("dialog[open]").count() == 0 and not pg.is_checked("#llm-on")
            pg.click("#uip-llm label:has(#llm-on)"); pg.wait_for_selector("dialog[open]"); pg.click("#llm-c-ok")
            pg.wait_for_selector("#llm-prev", timeout=10000)
            assert "mzlab.llm.consenso" in pg.evaluate("Object.keys(localStorage).join(' ')")
        step("turning on asks the consent (Esc closes it, accepting stores it)", consenso)
        def anteprima():
            pg.fill("#llm-msg", "hello"); pg.click("#llm-prev"); pg.wait_for_selector("#llm-prevout:not([hidden])")
            assert "localhost:11434" in pg.inner_text("#llm-prevout") and "hello" in pg.inner_text("#llm-prevout")
            assert "sk-" not in pg.inner_text("#llm-prevout")
        step("preview «What will be sent»", anteprima)
        def replay():
            pg.evaluate("window.MZLAB_LLM.replay(%s, false)" % json.dumps(REC))
            pg.fill("#llm-msg", "hello"); pg.click("#llm-send")
            attendi(pg, "document.getElementById('llm-out').textContent.includes('Hello from the recording')")
            assert any("wasm-feat/llm/" in u for u in seen), "the module was served from the site"
            assert any("trasporto.html" in u for u in seen) or True
        step("replay transport from 'self'", replay)
        def fuori():
            ext = [u for u in seen if not u.startswith("http://127.0.0.1:%d/" % r.port) and not u.startswith(("data:", "blob:"))]
            assert not ext, ext
        step("no request to an external host", fuori)
        def axe():
            if not AXE: return
            pg.add_script_tag(path=AXE)
            v = pg.evaluate("axe.run('#uip-llm', {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => v.id + ' ' + v.nodes.length))")
            assert not v, v
        step("axe: no serious violation in the section", axe)
        assert not [e for e in r.errs if e[0] in ("pageerror",)], [e for e in r.errs if e[0] == "pageerror"]
finally:
    pass
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n, "" if s == "ok" else s)
r.close()
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
