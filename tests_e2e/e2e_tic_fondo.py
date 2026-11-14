# covers: mzlab/web/tic-fondo-ui.js, mzlab/web/rust-ticfondo.js
"""The TIC without the chemical background (api/tic_background, docs/agenti/tic-fondo.md) on the built site: the module `ticfondo`
(crates/mzlab-ticfondo-wasm) is fetched on the first request, never at start-up; the answer finds the six background lines of the synthetic run
(tools/tic_fondo_sintetico.py) and equals `mzlab-cli background` byte for byte, alone and with a blank sample (mode=blank).
The button of the TIC/BPC panel (tic-fondo-ui.js): one click draws the cleaned trace (the same bytes as the CLI), a second the raw one; right click
or ↓ opens the options (no text on screen), a blank sample there gives the CLI answer with --blank, Esc closes them, the ions table opens. Needs the built site (tools/prepara_sito.py) and cargo. Run from the repo root."""
import json, os, subprocess, sys, tempfile
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

D = Path(tempfile.mkdtemp(prefix="ticfondo_"))
for extra in ([], ["--blank"]):
    subprocess.run([sys.executable, str(ROOT / "tools" / "tic_fondo_sintetico.py"), str(D), "--scans", "600", *extra], check=True, capture_output=True)
RUN, BLANK = D / "tic_fondo-s1.mzML", D / "tic_fondo_bianco-s1.mzML"
subprocess.run(["cargo", "build", "-q", "--locked", "--release", "-p", "mzlab-cli"], cwd=ROOT, check=True)
CLI = ROOT / "target" / "release" / "mzlab-cli"
cli = lambda *a: subprocess.run([str(CLI), "background", str(RUN), *a], cwd=ROOT, check=True, capture_output=True, text=True).stdout.strip()

r = Run(port=8987, wd="/tmp/wd_tic_fondo")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        asked = []
        pg.on("request", lambda q: asked.append(q.url))
        pg.wait_for_selector("#drop", timeout=120000)
        mod = lambda: [u for u in asked if "/wasm-feat/ticfondo/" in u and u.endswith("_bg.wasm")]
        pg.set_input_files("#pick", [str(RUN), str(BLANK)])
        pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 2", timeout=120000)
        pg.click("#opbtn")
        pg.wait_for_function("E.files.length >= 2 && E.pending.length === 0", timeout=120000)
        assert not mod(), mod()
        print("OK  the module is not fetched at start-up")

        get = """async u => { const x = await fetch(u); return [x.status, await x.text()]; }"""
        k, b = pg.evaluate("[0, 1].map(i => E.files.find(f => String(f.file).startsWith(i ? 'tic_fondo_bianco' : 'tic_fondo-')).k)")
        st, txt = pg.evaluate(get, f"api/tic_background?k={k}&level=1")
        assert st == 200, (st, txt[:300])
        ions = sorted(round(i["mz"]) for i in json.loads(txt)["ions"])
        assert ions == [141, 149, 158, 198, 214, 236], ions
        assert txt == cli(), "the site and mzlab-cli differ"
        b_k = b
        print("OK  six background lines, the same JSON as mzlab-cli")

        st, txt = pg.evaluate(get, f"api/tic_background?k={k}&level=1&mode=blank&blank={b}&kind=bpc")
        assert st == 200, (st, txt[:300])
        assert txt == cli("--blank", str(BLANK), "--set", "mode=blank", "--set", "kind=bpc"), "blank: the site and mzlab-cli differ"
        assert len(mod()) == 1, mod()
        print("OK  with a blank sample, the same JSON as mzlab-cli; the module fetched once")

        st, txt = pg.evaluate(get, f"api/tic_background?k={k}&level=1&mode=nope")
        assert st == 400 and json.loads(txt)["error_key"] == "err.request.badValue", (st, txt[:300])
        print("OK  a bad value is a keyed error")
        BTN = ".pnl.chrom .tbg:not([hidden]) > [data-a=ticbg]"
        pg.wait_for_selector(BTN, timeout=60000)
        b = pg.locator(BTN).first
        tip, box = b.get_attribute("title"), b.bounding_box()
        assert tip and len(tip) <= 90 and b.get_attribute("aria-label") == tip and b.get_attribute("aria-pressed") == "false", tip
        assert box["width"] >= 24 and box["height"] >= 24, box
        assert not b.inner_text().strip(), "the button has no text"
        shown = """async k => { const p = E.panels.find(q => q.type === "chrom" && q.el.querySelector("[data-a=ticbg]"));
          const s = (await seriesOf(p)).find(x => x.k === k), f = E.files.find(x => x.k === k), u = TICBG.url(p, f);
          return { y: Array.from(s.y), chrom: Array.from((await getChrom(k, p.kind, f.lv || 1)).y), on: TICBG.on(p), u, txt: await (await fetch(u)).text() }; }"""
        b.click()
        pg.wait_for_function(f"document.querySelector('{BTN}').getAttribute('aria-pressed') === 'true'", timeout=60000)
        got = pg.evaluate(shown, k)
        assert got["on"] and got["txt"] == cli(), ("the button and mzlab-cli differ", got["u"])
        assert got["y"] == json.loads(got["txt"])["y"], "the panel draws the cleaned y of the answer"
        print("OK  one click: the panel draws the TIC without the background, the same bytes as mzlab-cli")
        b.click()
        pg.wait_for_function(f"document.querySelector('{BTN}').getAttribute('aria-pressed') === 'false'", timeout=60000)
        raw = pg.evaluate(shown, k)
        assert not raw["on"] and raw["y"] == raw["chrom"], "a second click draws the raw trace"
        print("OK  a second click: the raw trace again")
        b.click(button="right")
        pg.wait_for_selector(".tb-pop", timeout=10000)
        texts = pg.evaluate("""() => { const w = document.createTreeWalker(document.querySelector('.tb-pop'), NodeFilter.SHOW_TEXT); const o = [];
          for (let n; (n = w.nextNode());) if (n.textContent.trim() && !n.parentElement.closest('option')) o.push(n.textContent.trim()); return o; }""")
        assert not texts, texts
        unl = pg.evaluate("[...document.querySelectorAll('.tb-pop select, .tb-pop input, .tb-pop button')].filter(x => !x.getAttribute('aria-label')).length")
        assert unl == 0, unl
        pg.select_option(".tb-pop [data-t=mode]", "blank")
        pg.select_option(".tb-pop [data-t=blank]", str(b_k))
        pg.wait_for_function(f"document.querySelector('{BTN}').getAttribute('aria-pressed') === 'true'", timeout=60000)
        got = pg.evaluate(shown, k)
        assert "blank=" in got["u"] and got["txt"] == cli("--blank", str(BLANK), "--set", "mode=blank"), ("blank: the options and mzlab-cli differ", got["u"])
        assert got["y"] == json.loads(got["txt"])["y"]
        pg.keyboard.press("Escape")
        pg.wait_for_selector(".tb-pop", state="detached", timeout=5000)
        assert pg.evaluate("document.activeElement && document.activeElement.dataset.a") == "ticbg", "Esc gives the focus back to the button"
        print("OK  right click: options with no text on screen; a blank sample = mzlab-cli --blank; Esc closes")
        pg.keyboard.press("ArrowDown")
        pg.wait_for_selector(".tb-pop", timeout=5000)
        pg.click(".tb-pop [data-t=table]")
        pg.wait_for_function("[...document.querySelectorAll('table')].some(t => t.querySelectorAll('tbody tr').length >= 6)", timeout=10000)
        print("OK  ↓ opens the options; the table of the background ions")
        assert not pg.evaluate("window.MZLAB_LR_MISS && window.MZLAB_LR_MISS.length"), pg.evaluate("window.MZLAB_LR_MISS")
finally:
    r.close()
