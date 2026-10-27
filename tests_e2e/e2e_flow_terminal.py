"""Flow, the Terminal panel (flow/lazy/flow-terminal.js, route api/flow/line): the line is copied when the panel opens («Copied ✓»), a --set typed in the line changes the plan that «Run here» runs,
a wrong --set shows the text of the CLI, and everything works from the keyboard. Run from the repo root: python3 tests_e2e/e2e_flow_terminal.py"""
import json, subprocess, sys, socket
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from lib import free_port, attendi; PORT = free_port()
subprocess.run([sys.executable, str(ROOT / "tools" / "prepara_sito.py")], check=True)
site = ROOT / "site"
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(2000):
    try: socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError: pass
bad = []
def check(name, ok, info=""):
    if not ok: bad.append(f"{name}: {info}")
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); ctx = b.new_context(viewport={"width": 1400, "height": 1000}); ctx.grant_permissions(["clipboard-read", "clipboard-write"], origin=f"http://127.0.0.1:{PORT}")
        pg = ctx.new_page(); pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.click('#nav button[data-v="flow"]'); attendi(pg, "document.querySelectorAll('.fln').length === 2")
        pg.click('#flx-cli-show'); attendi(pg, "!!document.querySelector('.fl-term .fl-line') && document.querySelector('.fl-term .fl-line').value.length > 0")
        line = pg.locator(".fl-term .fl-line").input_value()
        check("line", line.startswith("mzlab-cli flow run plan.json --in ") and " --out " in line, line)
        attendi(pg, "document.querySelector('.fl-term .fl-copied').textContent === 'Copied ✓'")
        check("clipboard", pg.evaluate("navigator.clipboard.readText()") == line)
        check("output live region", pg.get_attribute(".fl-term pre", "aria-live") == "polite")
        # a --set typed in the line is applied by the engine to the plan: the value is in the plan the route gives back
        pick = pg.evaluate("""() => { const S = FLOW.current; for (const inst of S.plan.steps) { const st = FLOW.stepOf(S.reg, inst), d = FLOW.defaults(inst, st);
            for (const [n, v] of Object.entries(d)) if (typeof v === 'number') return { id: inst.id, name: n, value: v + 1 }; } return null; }""")
        check("a numeric parameter", pick is not None)
        if pick:
            typed = f"{line} --set {pick['id']}.{pick['name']}={pick['value']}"
            pg.fill(".fl-term .fl-line", typed)
            back = pg.evaluate("async l => FLOW.lineCall(FLOW.current, l)", typed)
            check("set applied", f'"{pick["name"]}": {pick["value"]}' in back or f'"{pick["name"]}":{pick["value"]}' in back, back[:300])
            check("the plan of the page is unchanged", pg.evaluate("n => !JSON.stringify(FLOW.current.plan).includes(n)", f'"{pick["name"]}":{pick["value"]}'))
        # a wrong set: the CLI's own words, in one line
        pg.fill(".fl-term .fl-line", "mzlab-cli flow run plan.json --set nowhere.x=1")
        pg.keyboard.press("Enter"); attendi(pg, "document.querySelector('.fl-term pre').textContent.includes('no instance or step of that name')")
        txt = pg.inner_text(".fl-term pre"); check("wrong set", "no instance or step of that name" in txt and "\n" not in txt.strip(), txt)
        pg.fill(".fl-term .fl-line", "ls -l"); pg.click('.fl-term button:has-text("Run here")')
        attendi(pg, "document.querySelector('.fl-term pre').textContent.includes('mzlab-cli flow run')")
        # a valid line runs the plan here: the page says which files it uses, and the plan of the page is the same afterwards
        before = pg.evaluate("JSON.stringify(FLOW.current.plan)")
        pg.click('.fl-term button:has-text("Reset")'); attendi(pg, "document.querySelector('.fl-term .fl-line').value.startsWith('mzlab-cli flow run plan.json')")
        pg.click('.fl-term button:has-text("Run here")'); attendi(pg, "document.querySelector('.fl-term pre').textContent.length > 0 && !document.querySelector('.fl-term button[aria-label^=\"Run the plan here\"]').disabled", 60000)
        out = pg.inner_text(".fl-term pre")
        check("run here", "Files: the ones open in Data." in out or "No runner" in out, out)
        check("plan restored", pg.evaluate("JSON.stringify(FLOW.current.plan)") == before)
        # Reset gives the line of the plan back; Show script shows the script and a button to copy it
        pg.click('.fl-term button:has-text("Reset")'); attendi(pg, "document.querySelector('.fl-term .fl-line').value.startsWith('mzlab-cli flow run plan.json')")
        pg.click('.fl-term button:has-text("Show script")'); attendi(pg, "!document.querySelector('.fl-term textarea.fl-script').hidden")
        check("script", "<<'MZFLOW'" in pg.locator(".fl-term textarea.fl-script").input_value() and pg.locator(".fl-term button:has-text('Copy script')").is_visible())
        # every control is reachable by keyboard and at least 24 px
        small = pg.evaluate("[...document.querySelectorAll('.fl-term button, .fl-term input')].filter(e => !e.hidden && (e.getBoundingClientRect().height < 24 || e.getBoundingClientRect().width < 24)).map(e => e.outerHTML.slice(0, 60))")
        check("targets", not small, str(small))
        check("labels", pg.evaluate("[...document.querySelectorAll('.fl-term button')].every(b => b.getAttribute('aria-label'))"))
        pg.click('#flx-cli-show'); check("closes", pg.locator(".fl-term").count() == 0)
        b.close()
finally:
    srv.terminate()
print("FAIL\n" + "\n".join(bad) if bad else "OK e2e_flow_terminal")
sys.exit(1 if bad else 0)
