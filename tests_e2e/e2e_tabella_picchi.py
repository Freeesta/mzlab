"""Spectrum panel: the % axis ends at 100, an MS2 spectrum has no XIC button (MS1 keeps it), and the peak table (threshold, max rows, sort, only labelled,
Copy = TSV + html on the clipboard, Ctrl+C on the table, IT/EN, axe, target sizes)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
AXE = axe_path()
RUN = "axe.run(%s, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length]))"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_tabella_picchi.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
S1 = "E.panels.find(p=>p.type==='spec'&&p.tab==='full')"
S2 = "E.panels.find(p=>p.type==='spec'&&p.tab==='ms2')"
r = Run(port=8879, wd="/tmp/wd79")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        try: pg.context.grant_permissions(["clipboard-read", "clipboard-write"])
        except Exception: pass
        stage(pg, [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_MS2-t15")])
        pg.click("text=Carica dati"); ready(pg); pg.evaluate("setTab('full')"); ready(pg)
        def ms1_xic():
            assert pg.evaluate(f"{S1}.level") == 1 and not pg.evaluate(f"{S1}.el.querySelector('[data-a=xicb]').hidden"), "MS1 keeps the XIC button"
        step("MS1 spectrum: the XIC button is there", ms1_xic)
        def pct_axis(sp):
            pg.evaluate(f"(()=>{{const p={sp};p.rel=true;draw(p)}})()"); frames(pg, 3)
            t = pg.evaluate(f"{sp}._a.yticks"); assert t and max(t) == 100 and all(v <= 100 for v in t), t
        step("MS1 in %: the last tick of the y axis is 100", lambda: pct_axis(S1))
        open_kind(pg, "ms2")
        attendi(pg, f"(()=>{{const p={S2};return !!p&&!!p._a&&p._a.data[0].d.mz.length>0}})()")
        pg.evaluate(f"{S2}.el.scrollIntoView({{block:'center'}})"); frames(pg, 10)
        def ms2_noxic():
            assert pg.evaluate(f"{S2}.level") == 2 and pg.evaluate(f"{S2}.el.querySelector('[data-a=xicb]').hidden"), "no XIC button on MS2"
            assert pg.evaluate(f"getComputedStyle({S2}.el.querySelector('[data-a=xicb]')).display") == "none"
        step("MS2 spectrum: no XIC button", ms2_noxic)
        def ms2_pct():
            pg.evaluate(f"(()=>{{const p={S2};p.rel=true;draw(p)}})()"); frames(pg, 3)
            t = pg.evaluate(f"{S2}._a.yticks"); assert t and max(t) == 100, t
        step("MS2 in %: the last tick of the y axis is 100", ms2_pct)
        def opened():
            pg.evaluate(f"{S2}.el.querySelector('[data-a=tabp]').click()"); attendi(pg, "!!document.querySelector('.tp-pop')")
            head = pg.evaluate("[...document.querySelectorAll('.tp-pop th')].map(t=>t.textContent)"); print(head)
            assert head == ["m/z", "Intensità", "Intensità relativa (%)", "Δm/z dal precursore", "Perdita neutra"], head
            facts = pg.inner_text(".tp-facts"); print(facts)
            for w in ("Campione", "Scansione", "TR", "MS2", "Precursore", "Polarità"): assert w in facts, (w, facts)
        step("table opens on an MS2: facts and five columns", opened)
        def threshold():
            n0 = pg.evaluate("document.querySelectorAll('.tp-pop tbody tr').length")
            pg.fill("[data-f=thr]", "0"); n_all = pg.evaluate("document.querySelectorAll('.tp-pop tbody tr').length")
            pg.fill("[data-f=thr]", "2"); n2 = pg.evaluate("document.querySelectorAll('.tp-pop tbody tr').length")
            rel = pg.evaluate("[...document.querySelectorAll('.tp-pop tbody tr')].map(t=>+t.children[2].textContent)")
            print(n0, n_all, n2, min(rel)); assert n2 == n0 and n_all >= n2 and min(rel) >= 2, (n0, n_all, n2, min(rel))
            pg.fill("[data-f=thr]", "60"); n60 = pg.evaluate("document.querySelectorAll('.tp-pop tbody tr').length"); assert 1 <= n60 < n2, (n60, n2)
            pg.fill("[data-f=thr]", "2")
        step("threshold 2 % is the default and hides the small peaks", threshold)
        def maxrows_sort():
            pg.fill("[data-f=max]", "3"); assert pg.evaluate("document.querySelectorAll('.tp-pop tbody tr').length") == 3
            mzs = pg.evaluate("[...document.querySelectorAll('.tp-pop tbody tr')].map(t=>+t.children[0].textContent)"); assert mzs == sorted(mzs), mzs
            pg.select_option("[data-f=sort]", "int")
            rel = pg.evaluate("[...document.querySelectorAll('.tp-pop tbody tr')].map(t=>+t.children[2].textContent)"); assert rel == sorted(rel, reverse=True) and rel[0] == 100, rel
            pg.select_option("[data-f=sort]", "mz"); pg.fill("[data-f=max]", "50")
        step("max rows keeps the most intense; sort toggle", maxrows_sort)
        def labelled():
            pg.check("[data-f=lab]"); n = pg.evaluate("document.querySelectorAll('.tp-pop tbody tr').length")
            lb = pg.evaluate(f"{S2}._a.lbls.filter(l=>typeof l.m==='number'&&!l.ann).length"); assert 1 <= n <= lb, (n, lb)
            pg.uncheck("[data-f=lab]")
        step("only labelled peaks", labelled)
        def values():
            row = pg.evaluate("[...document.querySelector('.tp-pop tbody tr').children].map(t=>+t.textContent)")
            prec = pg.evaluate(f"{S2}.prec"); assert abs(row[3] - (row[0] - prec)) < 0.011 and abs(row[4] - (prec - row[0])) < 0.011, (row, prec)
        step("Δm/z and neutral loss are measured from the precursor", values)
        def copy_btn():
            pg.click("[data-f=copy]"); attendi(pg, "document.querySelector('.tp-ok').textContent==='Copiato'")
            t = pg.evaluate("navigator.clipboard.readText()"); L = t.split("\n"); n = pg.evaluate("document.querySelectorAll('.tp-pop tbody tr').length")
            assert L[0].split("\t") == ["m/z", "Intensità", "Intensità relativa (%)", "Δm/z dal precursore", "Perdita neutra"] and len(L) == n + 1 and "," not in t, (L[:2], n)
            assert all(len(x.split("\t")) == 5 for x in L), L[:3]
            h = pg.evaluate("navigator.clipboard.read().then(async i=>{const b=i[0].types.includes('text/html')?await (await i[0].getType('text/html')).text():'';return b})")
            assert "<table" in h and h.count("<tr>") == n + 1, h[:120]
            attendi(pg, "document.querySelector('.tp-ok').textContent===''", 4000)
        step("Copia: TSV with a header row and N rows, html table too, «Copiato» for 2 s", copy_btn)
        def ctrl_c():
            pg.click(".tp-sc"); pg.keyboard.press("Control+A"); pg.keyboard.press("Control+C"); frames(pg, 3)
            t = pg.evaluate("navigator.clipboard.readText()"); n = pg.evaluate("document.querySelectorAll('.tp-pop tbody tr').length")
            assert t.split("\n")[0].startswith("m/z\tIntensità") and len(t.split("\n")) == n + 1, t[:80]
        step("select all + Ctrl+C on the table gives the same TSV", ctrl_c)
        def a11y():
            for sel in ("[data-a=tabp]", ".tp-pop [data-f=copy]", ".tp-pop [data-f=no]", ".tp-pop [data-f=thr]", ".tp-pop [data-f=lab]", ".tp-pop [data-f=sort]"):
                sz = pg.evaluate("s=>{const r=document.querySelector(s).getBoundingClientRect();return [r.width,r.height]}", sel); assert sz[0] >= 24 and sz[1] >= 24, (sel, sz)
            assert pg.get_attribute("[data-a=tabp]", "aria-label")
            if AXE:
                pg.add_script_tag(path=AXE); v = pg.evaluate(RUN % "'#dpanels'"); assert not v, v
        step("targets >= 24 px, labels, axe: no serious violation", a11y)
        def esc():
            pg.keyboard.press("Escape"); assert pg.evaluate("!document.querySelector('.tp-pop')") and pg.evaluate("document.activeElement.dataset.a") == "tabp"
        step("Esc closes the table and gives the focus back", esc)
        def ms1_table():
            pg.evaluate(f"{S1}.el.querySelector('[data-a=tabp]').click()"); attendi(pg, "!!document.querySelector('.tp-pop')")
            head = pg.evaluate("[...document.querySelectorAll('.tp-pop th')].map(t=>t.textContent)"); assert head == ["m/z", "Intensità", "Intensità relativa (%)"], head
            pg.keyboard.press("Escape")
        step("MS1 table: three columns (no precursor)", ms1_table)
        def no_download():
            assert pg.evaluate(f"!{S1}.el.querySelector('[data-a=tabp]').closest('.tbs').querySelector('a[download]')")
        step("no download in the table", no_download)
        def english():
            pe = r.page(p); pe.goto(f"http://127.0.0.1:{r.port}/?lang=en"); attendi(pe, "!!document.querySelector('#pick')", 30000)
            stage(pe, [mz("B_FullMass-t0"), mz("B_MS2-t15")]); pe.click("text=Load data"); ready(pe); open_kind(pe, "ms2")
            attendi(pe, f"(()=>{{const p={S2};return !!p&&!!p._a&&p._a.data[0].d.mz.length>0}})()")
            pe.evaluate(f"{S2}.el.querySelector('[data-a=tabp]').click()"); attendi(pe, "!!document.querySelector('.tp-pop')")
            head = pe.evaluate("[...document.querySelectorAll('.tp-pop th')].map(t=>t.textContent)"); assert head == ["m/z", "Intensity", "Relative intensity (%)", "Δm/z from precursor", "Neutral loss"], head
            assert "Min. threshold %" in pe.inner_text(".tp-pop") and "Only labelled peaks" in pe.inner_text(".tp-pop"), pe.inner_text(".tp-pop")
            assert pe.get_attribute("[data-a=tabp]", "title").startswith("Peak table")
        step("English: the table speaks English", english)
        errs = [e for e in r.errs if "iframe" not in str(e)]
        print("--- BROWSER ERRORS", len(errs)); [print(e) for e in errs[:5]]
finally: r.close()
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("OK  " if s == "ok" else "FAIL"), n, "" if s == "ok" else s)
sys.exit(1 if bad else 0)
