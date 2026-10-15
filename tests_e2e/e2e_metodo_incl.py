"""The inclusion list in the Method window (metodo-incl.js): the table sits at the head of «Metodo registrato nel file» (not inside a <details>), each row has the short buttons «XIC» and «MS2»
(details in title/aria-label) that run the actions metodo.xic / metodo.ms2 of the registry azioni/1; XIC opens an XIC panel without the dialog, MS2 opens the MS2 scan of the precursor inside
the RT window of the entry, an entry with no MS2 has the button grey with the reason; right click and Shift+F10 give the same actions; «Dati del grafico» opens the copyable table.
The engine answers /api/metodo in the page, so the test puts a document with an `inclusione` in its place (the document of a real .raw is checked by crates/mzlab-core/tests/metodo_raw_parity.rs
and the unit tests of metodo_raw/inclusione.rs). Synthetic HR DDA file."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
import json
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_metodo_incl.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("metodoincl")
r = Run(port=8897, wd="/tmp/wdmetodoincl")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 6000, 2)
        pg.wait_for_timeout(2000)
        par = pg.evaluate("(async()=>{const k=E.files.findIndex(f=>f.dda&&f.kind==='full');const d=await DDA.get(k);const i=d.sid.findIndex((_,i)=>d.prec[i]!=null&&d.rt[i]>14&&d.rt[i]<14.6);return {k, mz:d.prec[i], rt:d.rt[i]}})()")
        doc = {"schema": "metodo/1", "origine": {"formato": "thermo-raw"}, "ms": {"periodi": [{"esperimenti": [{"indice": 1, "mz": [100, 1000]}, {"indice": 2, "inclusione": [
            {"mz": par["mz"], "formula": "C10H12N2O", "specie": "[M+H]+", "carica": 1, "polarita": "positive", "t_inizio_min": par["rt"] - 0.5, "t_fine_min": par["rt"] + 0.5, "nce": [20, 35, 60], "nome": "entry one"},
            {"mz": 123.4567, "polarita": "positive", "t_inizio_min": 1, "t_fine_min": 2, "nome": "entry two"},
            {"mz": par["mz"], "t_inizio_min": 1, "t_fine_min": 2, "nome": "right m/z, wrong window"}]}]}]}}
        pg.evaluate("(d => { const f = window.fetch; window.fetch = function (u, ...a) { return String(u).startsWith('api/metodo') ? Promise.resolve(new Response(JSON.stringify({ source: 'raw', metodo: d }), { headers: { 'content-type': 'application/json' } })) : f.call(this, u, ...a); }; })(%s)" % json.dumps(doc))
        pg.evaluate("E.cur = E.files.findIndex(f => f.dda && f.kind === 'full')")
        def table():
            pg.click("#np-method"); pg.wait_for_selector("#mincl", timeout=60000)
            assert pg.evaluate("!document.querySelector('#mincl').closest('details')"), "the list is inside a <details>"
            assert pg.evaluate("document.querySelector('#mincl').parentElement.firstElementChild.tagName") == "H4" and pg.evaluate("document.querySelector('#mincl').parentElement.children[2] === document.querySelector('#mincl')"), "not at the head"
            t = pg.inner_text("#mincl")
            assert "entry one" in t and "20, 35, 60" in t and "C10H12N2O" in t, t
            labels = pg.evaluate("[...document.querySelectorAll('#mincl tr[data-i]')].map(r => [...r.querySelectorAll('button')].map(b => b.textContent))")
            assert labels == [["XIC", "MS2"]] * 3, labels
            sz = pg.evaluate("[...document.querySelectorAll('#mincl button')].map(b => { const r = b.getBoundingClientRect(); return Math.min(r.width, r.height) })")
            assert min(sz) >= 24, sz
        step("the list is at the head of the section, with the buttons XIC and MS2 on every row", table)
        def ms2_state():
            pg.wait_for_function("!document.querySelector('#mincl tr[data-i=\"0\"] button[data-ia=ms2]').disabled", timeout=30000)
            b = pg.evaluate("[...document.querySelectorAll('#mincl tr[data-i] button[data-ia=ms2]')].map(b => [b.disabled, b.title, b.getAttribute('aria-label')])")
            assert b[0][0] is False and "MS" in b[0][1] and b[0][1] == b[0][2], b
            assert b[1][0] is True and b[1][1] and b[2][0] is True and b[2][1], b                    # no MS2 for the other precursor / outside the window: grey, with the reason
        step("the MS2 button wakes up only where a scan fits the entry; the others are grey with the reason", ms2_state)
        def menu():
            pg.click("#mincl tr[data-i='0'] td", button="right")
            assert pg.evaluate("[...document.querySelectorAll('#ctx [data-azione]')].map(d => d.dataset.azione)") == ["metodo.ms2", "metodo.xic"] or pg.evaluate("[...document.querySelectorAll('#ctx [data-azione]')].map(d => d.dataset.azione).sort()") == ["metodo.ms2", "metodo.xic"]
            assert pg.evaluate("(() => { const m = document.querySelector('#ctx'), r = m.getBoundingClientRect(); const e = document.elementFromPoint(r.left + 10, r.top + 10); return m.contains(e) })()"), "the menu is under the modal backdrop"
            pg.keyboard.press("Escape")
            pg.focus("#mincl tr[data-i='1'] button[data-ia=xic]"); pg.keyboard.press("Shift+F10")
            assert pg.evaluate("document.querySelector('#ctx [data-azione=\"metodo.ms2\"]').getAttribute('aria-disabled')") == "true"
            pg.keyboard.press("Escape")
        step("right click and Shift+F10 give the same actions (the MS2 one grey where nothing fits)", menu)
        def tab():
            pg.click("#mincl ~ div button[data-ia=tab]"); pg.wait_for_selector("#tdati table")
            assert pg.evaluate("document.querySelectorAll('#tdati tbody tr').length") == 3
            pg.keyboard.press("Escape"); pg.evaluate("document.getElementById('tdati') && document.getElementById('tdati').remove()")
        step("«Dati del grafico» opens the copyable table of the list", tab)
        def xic():
            n0 = pg.evaluate("E.panels.filter(p => p.type === 'xic').length")
            pg.click("#mincl tr[data-i='0'] button[data-ia=xic]"); pg.wait_for_timeout(1500)
            assert pg.evaluate("document.getElementById('xicdlg').open") is False and not pg.evaluate("document.getElementById('bigdlg').open"), "dialogs still open"
            q = pg.evaluate("E.panels.filter(p => p.type === 'xic').map(p => p.traces.map(t => t.mz))")
            assert len(q) == n0 + 1 and abs(q[-1][0] - par["mz"]) < 0.01, (q, par)
        step("XIC: the panel appears at once, without the dialog, on the m/z of the entry", xic)
        def ms2():
            pg.click("#np-method"); pg.wait_for_selector("#mincl"); pg.wait_for_function("!document.querySelector('#mincl tr[data-i=\"0\"] button[data-ia=ms2]').disabled", timeout=30000)
            pg.click("#mincl tr[data-i='0'] button[data-ia=ms2]"); pg.wait_for_timeout(2500)
            v = pg.evaluate("(() => { const s2 = E.panels.find(p => p.dda != null); return s2 && { sid: s2.sid, ion: s2.ion } })()")
            assert v and v["sid"] is not None and abs(v["ion"] - par["mz"]) < 0.01, v
            assert pg.evaluate("!document.getElementById('bigdlg').open")
        step("MS2: the scan of the precursor inside the window opens in the MS2 panel", ms2)
        def noerr():
            bad = [e for e in r.errs if e[0] in ("pageerror",)]
            assert not bad, bad
            assert pg.evaluate("(window.MZLAB_LR_MISS || []).length") == 0
        step("no page error, no engine miss", noerr)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
