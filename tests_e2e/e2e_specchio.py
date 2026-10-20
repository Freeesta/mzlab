"""Tabui, «Compare with library» in the open row of the Feature table (TP_Mine/js/37_feature_table.js, drawn by LIB.mirror): the table is mounted with a stub session (one row with MS2),
a synthetic MSP library is added through the Libraries window. Checks: no button without a library, the button with one, the mirror and its TDATI table, the picker of the best results driven from
the keyboard, the message without a match, axe, and the line «MISURA clic_specchio». Run from the repo root: python3 tests_e2e/e2e_specchio.py"""
import os, subprocess, sys, tempfile, urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:700]))

tmp = Path(tempfile.mkdtemp()); site = tmp / "site"
reuse = os.environ.get("MZLAB_PIPE_SITE")
if reuse: site = Path(reuse)
else: subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(100):
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1).read(); break
    except Exception: __import__("threading").Event().wait(0.1)
errs = []
PREC = 301.1
PEAKS = [(105.03, 100.0), (133.06, 60.0), (161.05, 40.0), (189.09, 30.0), (217.12, 20.0), (245.1, 15.0), (273.14, 10.0), (283.1, 8.0)]
def rec(name, prec, peaks):
    return f"Name: {name}\nPrecursorMZ: {prec}\nIon_mode: Positive\nPrecursor_type: [M+H]+\nCollision_energy: 30\nNum Peaks: {len(peaks)}\n" + "\n".join(f"{m:.4f}\t{i:.1f}" for m, i in peaks) + "\n"
MSP = "\n".join([rec("Match", PREC, PEAKS), rec("Partial", PREC, PEAKS[:4]), rec("Shifted", PREC, [(m + 3, i) for m, i in PEAKS]), rec("Few", PREC, PEAKS[:2]), rec("Alt", PREC, PEAKS[:6]), rec("Far", PREC + 100, PEAKS)])
STUB = """() => {
  const ms2 = { mz: %s, y: %s };
  const session = {
    view: async () => ({ conteggi: { tenute: 1, righe: 1 } }), qc: async () => ({ scartate: [] }),
    rows: async () => [["id"], [1, %s, 5.0, 4.9, 5.1, 1, 1000, "DETECTED"]],
    row: async () => ({ rt: 5.0, rt_lo: 4.9, rt_hi: 5.1, cells: [{ type: "sample", state: "detected", area: 1000, time: 0, apex_rt: 5.0 }], quality: null, ms2: { file: "a" } }),
    traces: async () => ({ xic: [{ file: "a", rt: [4.8, 5, 5.2], y: [10, 1000, 10] }], ms1: { mz: [%s], y: [1000] }, ms2 }),
  };
  const host = document.createElement("div"); host.id = "spc-host"; host.style.cssText = "position:fixed;left:0;top:0;width:100%%;max-width:900px;height:100%%;overflow:auto;background:var(--panel)"; document.body.appendChild(host);
  window.PIPET.mount(host, { session, names: ["a"], gate: { ok: true, table: true }, say: e => String(e && e.message || e), cs: null, ppm: () => 10, pol: () => 1, formula: "", levels: null, identsOf: () => [] });
}""" % ([m for m, _ in PEAKS], [i for _, i in PEAKS], PREC, PREC)
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        def fresh(scheme="light", width=1100):
            c = b.new_context(viewport={"width": width, "height": 900}, reduced_motion="reduce", color_scheme=scheme); pg = c.new_page()
            pg.on("pageerror", lambda e: errs.append(str(e)))
            pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');}catch(_){}")
            pg.goto(f"http://127.0.0.1:{PORT}/?lang=en"); pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=60000)
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_function("window.PIPET && window.TPRIGA && window.LIB", timeout=60000)
            pg.evaluate(STUB); pg.wait_for_selector("#spc-host #ft-t tbody tr", timeout=20000)
            return pg
        def open_row(pg):
            pg.click("#spc-host #ft-t tbody tr"); pg.wait_for_selector("#spc-host .ft-cmp", state="attached", timeout=20000)
        def add_lib(pg):
            pg.evaluate("LIB.open()"); pg.wait_for_function("document.querySelector('#libdlg') && document.querySelector('#libdlg').open")
            pg.set_input_files("#lib-file", {"name": "prova.msp", "mimeType": "text/plain", "buffer": MSP.encode()})
            pg.wait_for_function("document.querySelector('#lib-list') && /prova/.test(document.querySelector('#lib-list').innerText)", timeout=30000)
            pg.click("#lib-x")
        pg = fresh(); open_row(pg)
        def nolib():
            assert not pg.evaluate("LIB.call('list').then(l => l.libs.length)"), "the library list must be empty"
            assert pg.locator(".ft-cmp-go:visible").count() == 0, "the button must not show without a library"
        step("no installed library: no «Compare with library» button", nolib)
        add_lib(pg)
        pg.click("#spc-host #ft-t tbody tr"); lib.attendi(pg, "!document.querySelector('#spc-host .ft-cmp')"); open_row(pg)
        clicks = 0
        def button():
            pg.wait_for_selector(".ft-cmp-go:visible", timeout=10000)
        step("with a library the button shows under the MS2", button)
        def mirror():
            global clicks
            pg.click(".ft-cmp-go"); clicks += 1
            pg.wait_for_selector(".ft-cmp canvas:visible", timeout=30000)
            assert pg.locator(".ft-cmp-b").count() == 5, pg.locator(".ft-cmp-b").count()
            assert pg.locator(".ft-cmp-b").first.get_attribute("aria-pressed") == "true"
            assert pg.locator(".ft-cmp-b").first.inner_text().startswith("1. Match"), pg.locator(".ft-cmp-b").first.inner_text()
            assert pg.locator(".ft-cmp canvas").get_attribute("aria-label"), "the mirror has a name"
            assert pg.locator(".ft-cmp .tdbtn").count() == 1, "the mirror has its table of the data"
        step("the best match is drawn as a mirror with its TDATI table and 5 results", mirror)
        def keyboard():
            pg.locator(".ft-cmp-b").nth(1).focus(); pg.keyboard.press("Enter")
            assert pg.locator(".ft-cmp-b").nth(1).get_attribute("aria-pressed") == "true"
            assert pg.locator(".ft-cmp-b").first.get_attribute("aria-pressed") == "false"
            assert pg.locator(".ft-cmp-b").nth(1).evaluate("e => document.activeElement === e"), "focus stays on the choice"
            assert pg.locator(".ft-cmp-m").inner_text().split(" · ")[0] == pg.locator(".ft-cmp-b").nth(1).inner_text().split(". ", 1)[1].split(" · ")[0]
        step("another result is chosen from the keyboard", keyboard)
        def clic():
            print(f"MISURA clic_specchio: {clicks} click from the open row")
            assert clicks == 1
        step("one click from the open row", clic)
        def tdati():
            pg.click(".ft-cmp .tdbtn"); pg.wait_for_selector("dialog[open] table, .tdati table", timeout=10000)
            t = pg.inner_text("dialog[open] table, .tdati table"); assert "m/z" in t and "Library" in t or "library" in t.lower(), t[:200]
            pg.keyboard.press("Escape")
        step("the table of the data lists the experimental and the library peaks", tdati)
        AXE = lib.axe_path()
        def axe():
            pg.add_script_tag(path=AXE) if not pg.evaluate("!!window.axe") else None
            v = pg.evaluate("axe.run(document.querySelector('.ft-cmp'), {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.nodes.length]))")
            assert not v, v
        if AXE: step("axe: no serious or critical violation", axe)
        else: print("SKIP axe-core not available")
        def narrow():
            pn = fresh(width=390); add_lib(pn); open_row(pn); pn.click(".ft-cmp-go"); pn.wait_for_selector(".ft-cmp canvas:visible", timeout=30000)
            r = pn.evaluate("() => [...document.querySelectorAll('.ft-cmp, .ft-cmp *')].map(e => Math.round(e.getBoundingClientRect().right)).reduce((a, b) => Math.max(a, b), 0)")
            assert r <= 390, r
        step("390 px: the comparison stays inside the screen", narrow)
        def nomatch():
            pn = fresh(); add_lib(pn); open_row(pn)
            pn.evaluate("() => { /* force a precursor far from every entry */ }")
            pn.evaluate("() => { const s = window.LIB.call; window.LIB.call = (k, a) => s(k, k === 'search' ? { ...a, prec: 900 } : a); }")
            pn.wait_for_selector(".ft-cmp-go:visible", timeout=10000); pn.click(".ft-cmp-go")
            pn.wait_for_function("document.querySelector('.ft-cmp-m').innerText.includes('No library spectrum')", timeout=30000)
            assert pn.locator(".ft-cmp canvas:visible").count() == 0
        step("no match: «No library spectrum within the tolerances»", nomatch)
        b.close()
finally:
    srv.terminate()
for n, r in steps: print(("PASS " if r == "ok" else "FAIL ") + n + ("" if r == "ok" else " :: " + r))
if errs: print("pageerrors:", errs[:3])
sys.exit(1 if any(r != "ok" for _, r in steps) or errs else 0)
