"""Identification tab of the information bar: no explanatory texts and no «identify all» button, libraries as rows ticked by polarity and level, load file / folder,
the «Get libraries» menu (links only: the CSP does not allow fetching from the sources) and the table of candidates. Synthetic libraries; no request leaves the site."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_ident_librerie.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).replace("\n"," | ")[:400]))
D = synth("identlib")
r = Run(port=8988, wd="/tmp/wd_identlib")
PEAKS = [(80.0, 30.0), (120.0, 100.0), (150.0, 60.0), (210.0, 40.0)]
def msp(name, mode):
    return f"Name: {name}\nPrecursorMZ: 300.1\nIon_mode: {mode}\nPrecursor_type: [M+H]+\nNum Peaks: {len(PEAKS)}\n" + "\n".join(f"{m:.4f}\t{i:.1f}" for m, i in PEAKS) + "\n\n"
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1500})
        external = []
        pg.on("request", lambda q: external.append(q.url) if not q.url.startswith(("http://127.0.0.1", "http://localhost", "data:", "blob:")) else None)
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        body = lambda: pg.inner_text("#hri-body")
        def texts():
            pg.evaluate("BARRA.setTab('idn')"); pg.wait_for_selector("#idn-go", timeout=15000); t = body()
            for gone in ("Confronto con", "Identifica tutte", "mai una certezza", "Lo spettro deve essere"): assert gone not in t, t
            assert pg.locator("#hri-a").count() == 0 and "Carica file" in t and "Carica cartella" in t
        step("no introductory texts, no «identify all» button", texts)
        def autoselect():
            for n, m in (("Pos", "Positive"), ("Neg", "Negative")):
                pg.evaluate("(async t => { await LIB.call('add', {file: new File([t[0]], t[1])}) })(%s)" % __import__("json").dumps([msp("Entry " + n, m), n.lower() + ".msp"]))
            pg.evaluate("LIB.queryOf = () => ({peaks: %s, prec: 300.1, pol: 1, f: {k: 0, label: 'f'}, rt: 5.25})" % __import__("json").dumps(PEAKS))
            pg.evaluate("IDP.render(document.querySelector('#hri-body'), {level: 2}, () => false)"); pg.wait_for_selector("#hri-body input[data-id]", timeout=15000)
            assert "Spettro: RT 5.25 min · MS2 · precursore m/z 300.1000 · polarità +" in " ".join(body().split()), body()
            ck = pg.evaluate("[...document.querySelectorAll('#hri-body input[data-id]')].map(c => [c.getAttribute('aria-label'), c.checked, c.closest('tr').title])")
            d = {n: (c, t) for n, c, t in ck}; assert d["pos"][0] and not d["neg"][0], ck
            assert d["neg"][1] == "Altra polarità" and d["pos"][1] == "Compatibile con lo spettro", ck
        step("two libraries (positive, negative): the compatible one is ticked by itself, with the reason as tooltip", autoselect)
        def search():
            pg.click("#idn-go"); pg.wait_for_selector("#idn-res tbody tr", timeout=20000)
            names = pg.locator("#idn-res tbody tr th").all_inner_texts(); assert names == ["Entry Pos"], names
            assert float(pg.inner_text("#idn-res tbody tr td.num")) > 0.99
            pg.evaluate("document.querySelector('#hri-body input[data-id]:checked').click()"); pg.wait_for_function("!document.querySelector('#hri-body input[data-id]:checked')")
            assert pg.locator("#idn-res tbody tr").count() == 1, "the table stays while the ticks change"
            pg.click("#idn-go"); pg.wait_for_function("document.querySelector('#idn-msg').innerText.includes('Seleziona una libreria')", timeout=10000)
        step("search: the table shows the ticked libraries only; an unticked box leaves it out of the search", search)
        def folder():
            import tempfile; d = tempfile.mkdtemp(); open(d + "/c.msp", "w").write(msp("Entry C", "Positive")); open(d + "/nota.txt", "w").write("x")
            pg.set_input_files("#idn-din", d)
            pg.wait_for_function("document.querySelector('#idn-msg').innerText.includes('Caricate: 1 · ignorate: 1')", timeout=20000)
        step("load folder: what is recognised is loaded, the rest is counted", folder)
        def catalog():
            pg.evaluate("IDP.render(document.querySelector('#hri-body'), {level: 2}, () => false)"); pg.click("#cat-b"); pg.wait_for_selector("#cat-box .cat-it", timeout=10000)
            assert pg.locator("#cat-box .cat-it").count() >= 20 and "GNPS (All-no propagated)" in pg.inner_text("#cat-box") and "Zenodo" in pg.inner_text("#cat-box")
            assert pg.evaluate("document.querySelector('#cat-b').getAttribute('aria-expanded')") == "true"
            sites = pg.evaluate("[...document.querySelectorAll('#cat-box a[target=_blank]')].map(a => [a.rel, a.href.startsWith('https://'), a.getAttribute('aria-label')])")
            assert sites and all(x[0] == "noopener noreferrer" and x[1] and x[2].startswith("Apri il sito") for x in sites), sites[:2]
            dls = pg.evaluate("[...document.querySelectorAll('#cat-box a[download]')].map(a => [a.rel, a.href.startsWith('https://'), a.getAttribute('aria-label')])")
            assert len(dls) == len(sites) and all(x[0] == "noopener noreferrer" and x[2].startswith("Scarica") for x in dls)
            assert pg.evaluate("import('/static/ident-catalogo.js').then(m => [m.isSupported('mgf'), m.isSupported('parquet')])") == [True, False]
            pg.keyboard.press("Escape"); assert pg.evaluate("document.querySelector('#cat-box').hidden") is True
        step("«Get libraries»: entries with download and site links (noopener noreferrer), Escape closes", catalog)
        step("nothing left the site (the catalog menu opens no connection)", lambda: external and (_ for _ in ()).throw(AssertionError(external[:3])))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(0 if all(s == "ok" for _, s in steps) else 1)
