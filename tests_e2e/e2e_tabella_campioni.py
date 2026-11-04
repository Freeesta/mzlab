# e2e_tabella_campioni: the sample table of Data (synthetic data) - headers with room (Time + its unit select, Size), file names not underlined, a header with only the count,
# ONE toolbar row (count, Columns, Groups, download, Remove all), the last column always in view (also in a narrow window), the download icon with a menu of the two formats,
# «Remove all files» with a confirmation, and «Back to work!» right under the toolbar of the work area (no blank band, desktop and narrow)
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))
NAMES = ["CBZ_t%d_MS2_180_251.mzML" % t for t in (0, 10, 15)]
r = Run(port=8872, wd="/tmp/wd_tabcamp")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_viewport_size({"width": 1400, "height": 900})
        pg.set_input_files("#pick", [{"name": n, "mimeType": "application/octet-stream", "buffer": open(mz("B_FullMass-t%d" % t), "rb").read()} for n, t in zip(NAMES, (0, 10, 15))])
        frames(pg, 3); pg.click("text=Carica dati"); ready(pg)
        # two converted files make the download controls appear (the maker's files of the lab are converted on request)
        pg.evaluate("n => { n.forEach(x => CONV.files.set(x, new Blob(['x']))); S.table = true; syncWork(); renderFiles(); }", NAMES[:2]); frames(pg, 3)
        def head_count():
            assert pg.inner_text("#flist .samphead b").strip() == "3 campioni", pg.inner_text("#flist .samphead b")
            assert "clicca" not in pg.inner_text("#flist .samptool")
            assert pg.evaluate("I18N.t('load.samples.head', {n: 24})") == "24 campioni"
        step("the header says only the count", head_count)
        def one_row():
            mids = pg.evaluate("""[...document.querySelectorAll('#flist .samptool button, #flist .samptool summary')].filter(e => e.getBoundingClientRect().width > 0 && getComputedStyle(e).visibility !== 'hidden')
                .map(e => { const r = e.getBoundingClientRect(); return [e.id || e.textContent.trim(), (r.top + r.bottom) / 2]; })""")
            ids = [m[0] for m in mids]
            for k in ("colsbtn", "tb-by", "dlall", "rmall"): assert k in ids, (k, ids)
            assert max(m[1] for m in mids) - min(m[1] for m in mids) <= 3, mids
        step("count, Columns, Groups, download and Remove all are on ONE row", one_row)
        def headers_fit():
            bad = pg.evaluate("""[...document.querySelectorAll('#flist thead th')].filter(t => t.scrollWidth > t.clientWidth + 1 ||
                [...t.querySelectorAll('select')].some(s => s.getBoundingClientRect().right > t.getBoundingClientRect().right + 1)).map(t => t.innerText)""")
            assert not bad, bad
        step("no header is clipped (Time with its unit, Size)", headers_fit)
        def no_underline():
            for n in NAMES:
                d = pg.evaluate("n => getComputedStyle(document.querySelector('[data-open=\"' + n + '\"]')).textDecorationLine", n); assert d == "none", (n, d)
            assert NAMES[2] in pg.inner_text("#flist")
        step("file names are not underlined (the underscores read)", no_underline)
        def last_column():
            for w in (1400, 900, 600):
                pg.set_viewport_size({"width": w, "height": 900}); frames(pg, 3)
                for sel in ("[data-rm]",):
                    ok = pg.evaluate("""s => { const f = document.getElementById('flist').getBoundingClientRect();
                        return [...document.querySelectorAll('#flist ' + s)].every(b => { const r = b.getBoundingClientRect(); return r.right <= f.right + 1 && r.left >= f.left && r.width >= 24 && r.height >= 24; }); }""", sel)
                    assert ok, (w, "the remove button is clipped")
            pg.set_viewport_size({"width": 1400, "height": 900}); frames(pg, 3)
        step("the × of every row is always in view (1400, 900, 600 px)", last_column)
        def menu():
            b = pg.locator("#flist [data-dl]").first; assert b.inner_text().strip() == "" and b.get_attribute("aria-label"), "icon only, with a name"
            b.click(); frames(pg, 2)
            assert pg.locator(".dlmenu button").all_inner_texts() == ["mzML", "mzML + Numpress"], pg.locator(".dlmenu button").all_inner_texts()
            pg.keyboard.press("Escape"); frames(pg, 2); assert pg.locator(".dlmenu").count() == 0
            pg.click("#dlall"); frames(pg, 2); assert pg.locator(".dlmenu button").count() == 2
            pg.keyboard.press("Escape"); frames(pg, 2)
        step("the download icons open a menu with mzML and mzML + Numpress", menu)
        def pick_numpress():
            pg.locator("#flist [data-dl]").first.click()
            with pg.expect_download(timeout=30000): pg.click(".dlmenu [data-dlfmt=smallest]")
            assert pg.evaluate("CONV.compression") == "smallest"
        step("a format of the menu saves with that compression", pick_numpress)
        def gap(w):
            pg.set_viewport_size({"width": w, "height": 900}); pg.evaluate("S.table = false; S.adding = true; syncWork()"); frames(pg, 4)
            g = pg.evaluate("document.getElementById('backwork').getBoundingClientRect().top - document.getElementById('tools').getBoundingClientRect().bottom")
            assert 0 <= g < 40, (w, "blank band between the toolbar and «Back to work!»", g)
            pg.evaluate("S.adding = false; S.table = true; syncWork()"); frames(pg, 3)
        step("«Back to work!» sits right under the toolbar (1400 px)", lambda: gap(1400))
        step("«Back to work!» sits right under the toolbar (700 px)", lambda: gap(700))
        def remove_all():
            pg.set_viewport_size({"width": 1400, "height": 900}); frames(pg, 3)
            pg.click("#rmall"); pg.wait_for_selector("#askdlg[open]"); pg.click("#askno"); frames(pg, 3)
            assert pg.evaluate("E.files.every(f => !f.gone)"), "Cancel keeps the files"
            pg.click("#rmall"); pg.wait_for_selector("#askdlg[open]"); pg.click("#askok")
            attendi(pg, "E.files.every(f => f.gone)", 30000)
        step("«Remove all files» asks first, Cancel keeps them, OK removes them all", remove_all)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
