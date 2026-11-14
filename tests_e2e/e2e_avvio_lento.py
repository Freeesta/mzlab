# e2e_avvio_lento: a first visit on a slow link (the engine .wasm arrives late) with nothing stored does not end in «reopening the saved files is not answering»
# (browser.js lrInit): the limit runs from when the worker has its engine (rust-bridge.js api.up), and with no stored file a late boot shows «the first visit downloads about 15 MB»
# and the page keeps waiting. Needs the site: python3 tools/prepara_sito.py, then python3 tests_e2e/e2e_avvio_lento.py
# covers: mzlab/web/browser, mzlab/web/rust-bridge
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:600]))
# the engine .wasm held back `ms` (what a 1.6 Mbit/s link does to it), and every text of the loading phase kept in window.__fasi
SLOW = """(() => { const ms = %d, f = window.fetch.bind(window); window.__fasi = [];
  window.fetch = (u, i) => { const s = typeof u === "string" ? u : (u && u.url) || String(u); return /mzlab_wasm_bg\\.wasm/.test(s) ? new Promise(r => setTimeout(r, ms)).then(() => f(u, i)) : f(u, i); };
  new MutationObserver(() => { const e = document.getElementById("avvio-fase"); const t = e && e.textContent.trim(); if (t && window.__fasi[window.__fasi.length - 1] !== t) window.__fasi.push(t); })
    .observe(document, { subtree: true, childList: true, characterData: true }); })()"""
ERR = "(() => { const e = document.getElementById('avvio-errore'); return e && !e.hidden ? e.innerText : ''; })()"
def visit(port, query, hold_ms):
    r = Run(port=port, wd=f"/tmp/wd_avvio_lento_{port}")
    try:
        with sync_playwright() as p:
            b = p.chromium.launch(); ctx = b.new_context(locale="it-IT", viewport={"width": 1300, "height": 900})
            ctx.add_init_script(SLOW % hold_ms)
            pg = ctx.new_page(); pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
            pg.goto(f"http://127.0.0.1:{r.port}/{query}")
            wait_for_js(pg, f"window.MZLAB_LR !== undefined || !!window.MZLAB_LR_ERR || {ERR} !== ''", 120000)
            out = {"err": pg.evaluate(ERR), "lrErr": pg.evaluate("window.MZLAB_LR_ERR || ''"), "ready": pg.evaluate("window.MZLAB_LR !== undefined"), "fasi": pg.evaluate("window.__fasi")}
            assert not [e for e in r.errs if e[0] == "pageerror"], r.errs
            b.close()
            return out
    finally:
        r.close()
def late_wasm():
    o = visit(8868, "?ripresa=2000", 7000)       # the limit (2 s) is shorter than the download (7 s): it must not count the download
    assert o["ready"] and not o["err"] and not o["lrErr"], o
def slow_note():
    o = visit(8869, "?ripresa=1", 3000)          # a limit shorter than any boot: nothing stored, so a note and the page goes on
    assert o["ready"] and not o["err"] and not o["lrErr"], o
    assert any("può volerci un minuto" in t for t in o["fasi"]), o["fasi"]
    assert not any("non risponde" in t for t in o["fasi"]), o["fasi"]
step("engine .wasm late on a first visit: no «not answering», the page gets ready", late_wasm)
step("nothing stored and a late boot: «the first visit downloads about 15 MB», then ready", slow_note)
print("\nSTEPS"); [print(" ", s) for s in steps]
if any(s[1] != "ok" for s in steps): sys.exit(1)
