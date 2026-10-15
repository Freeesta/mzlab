"""Instrument log of a Thermo .raw: the «Registro dello strumento» kind in the chromatogram selector (grouped channels, search box, drawn like an analog trace with its TDATI table)
and the error log in the Method window (times, a click moves the cursor). The answers of api/metodo and api/registro are faked on a synthetic mzML: the engine side is tested in Rust."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_registro.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = HERE / "shots" / "registro_tmp"; D.mkdir(parents=True, exist_ok=True)
F = D / "REG_t0.mzML"; ds.full_scan(F, 0, np.random.default_rng(5))
FAKE = """(() => {
  const orig = window.fetch.bind(window), ids = [
    {id: 'status:Ion Source/Spray Voltage (kV)', gruppo: 'Ion Source', nome: 'Spray Voltage', unita: 'kV', n: 40, min: 3.4, max: 3.6},
    {id: 'status:Ion Source/Sheath Gas (arb)', gruppo: 'Ion Source', nome: 'Sheath Gas', unita: 'arb', n: 40, min: 10, max: 12},
    {id: 'status:Vacuum/Fore Pressure (mbar)', gruppo: 'Vacuum', nome: 'Fore Pressure', unita: 'mbar', n: 40, min: 1.2, max: 1.4},
    {id: 'trailer:Ion Injection Time (ms)', gruppo: '', nome: 'Ion Injection Time', unita: 'ms', n: 100, min: 1, max: 50}];
  const json = o => new Response(JSON.stringify(o), {status: 200, headers: {'Content-Type': 'application/json'}});
  window.fetch = async (u, o) => {
    const s = String(u);
    if (s.startsWith('api/registro')) {
      const id = new URL(s, location.href).searchParams.get('id'), v = ids.find(x => x.id === id);
      if (!v) return json({error: 'no', error_key: 'x', params: {}});
      const t = [...Array(40).keys()].map(i => i * 0.5);
      return json({time: t, intensity: t.map(x => v.min + (v.max - v.min) * x / 20), unit: v.unita});
    }
    const r = await orig(u, o);
    if (s.startsWith('api/metodo')) {
      const j = await r.clone().json();
      if (j) {
        j.metodo = Object.assign({schema: 'metodo/1'}, j.metodo || {});
        j.metodo.serie_registro = ids;
        j.metodo.registro = [{t_min: 1.5, testo: 'Warning: spray unstable', livello: 'Warning'}, {t_min: 4, testo: 'Info: lock mass found', livello: 'Info'}, {t_min: 7.25, testo: 'Error: pump'}];
        return json(j);
      }
    }
    return r;
  };
})()"""
r = Run(port=8872, wd="/tmp/wd_registro")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.evaluate(FAKE)
        pg.set_input_files("#pick", [str(F)]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        TIC = "E.panels.find(p=>p.type==='chrom'&&p.tab==='full')"       # the first chromatogram of the tab, whatever its kind becomes
        def option():
            pg.wait_for_function(f"!!{TIC}.el.querySelector('select[data-o=kind] option[value=reg]')")
            t = pg.evaluate(f"{TIC}.el.querySelector('select[data-o=kind] option[value=reg]').textContent"); assert t == "Registro dello strumento", t
        step("the selector offers «Registro dello strumento» when a file has series", option)
        def pick():
            pg.evaluate(f"(() => {{const s={TIC}.el.querySelector('select[data-o=kind]'); s.value='reg'; s.dispatchEvent(new Event('change',{{bubbles:true}}))}})()")
            pg.wait_for_function(f"!!{TIC}.el.querySelector('select[data-o=reg]')")
            g = pg.evaluate(f"[...{TIC}.el.querySelectorAll('select[data-o=reg] optgroup')].map(o=>[o.label,o.children.length])")
            assert g == [["Ion Source", 2], ["Vacuum", 1], ["Per scansione", 1]], g
            assert pg.evaluate(f"{TIC}.el.querySelector('select[data-o=reg]').getAttribute('aria-label')") == "Canale del registro"
        step("channels are grouped by section, «Per scansione» for the trailer", pick)
        def search():
            pg.evaluate(f"(() => {{const i={TIC}.el.querySelector('input[data-regq]'); i.value='fore'; i.dispatchEvent(new Event('input',{{bubbles:true}}))}})()")
            n = pg.evaluate(f"{TIC}.el.querySelectorAll('select[data-o=reg] option').length"); assert n == 1, n
            pg.evaluate(f"(() => {{const i={TIC}.el.querySelector('input[data-regq]'); i.value='';i.dispatchEvent(new Event('input',{{bubbles:true}}))}})()")
            assert pg.evaluate(f"{TIC}.el.querySelectorAll('select[data-o=reg] option').length") == 4
        step("the search box filters the channels", search)
        def drawn():
            pg.evaluate(f"(() => {{const s={TIC}.el.querySelector('select[data-o=reg]'); s.value='status:Vacuum/Fore Pressure (mbar)'; s.dispatchEvent(new Event('change',{{bubbles:true}}))}})()")
            pg.wait_for_function(f"{TIC}._a && {TIC}._a.sr.length === 1 && {TIC}._a.sr[0].unit === 'mbar'"); ready(pg)
            v = pg.evaluate(f"(()=>{{const s={TIC}._a.sr[0];return [s.x.length,s.y[0],s.y[s.y.length-1]]}})()")
            assert v[0] == 40 and abs(v[1] - 1.2) < 1e-9 and v[2] > 1.39, v
        step("a channel is drawn as an analog trace with its unit", drawn)
        def table():
            pg.evaluate(f"{TIC}.el.querySelector('[data-a=tab]').click()"); pg.wait_for_selector("#tdati[open]")
            t = pg.evaluate("(()=>{const d=document.querySelector('#tdati');return {h:[...d.querySelectorAll('thead th')].map(x=>x.textContent),n:d.querySelectorAll('tbody tr').length}})()")
            assert t["n"] == 40 and any("mbar" in h for h in t["h"]), t
            pg.evaluate("document.querySelector('#tdati').close()")
        step("the chart comes with its table of data (TDATI), the unit in the header", table)
        def method():
            pg.click("#np-method"); pg.wait_for_selector("#bigdlg[open] button[data-mreg-rt]", state="attached"); pg.evaluate("document.querySelectorAll('#bigdlg details').forEach(d => d.open = true)")
            rows = pg.evaluate("[...document.querySelectorAll('#bigdlg button[data-mreg-rt]')].map(b=>[b.dataset.mregRt,b.getAttribute('aria-label')])")
            assert rows == [["1.5", "Vai a RT 1.5 min"], ["4", "Vai a RT 4 min"], ["7.25", "Vai a RT 7.25 min"]], rows
        step("the Method window lists the error log with its times", method)
        def go():
            pg.click("#bigdlg button[data-mreg-rt='4']"); pg.wait_for_timeout(500)
            assert not pg.evaluate("Q('#bigdlg').open")
            c = pg.evaluate("E.panels.find(p=>p.type==='chrom').cur"); assert abs(c - 4) < 1e-9, c
        step("a click on a time moves the chromatogram cursor", go)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
