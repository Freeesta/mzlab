"""Instrument files (.raw, .wiff, .d, .lcd) dropped in the start page: ONE notice for the whole loading (count + collapsible names, max 5 + «and N more»),
the openable mzML still loads, no repeated notice for the same group, and the guide «Non hai file mzML?» opens from the button and from the keyboard."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import synth
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_converti.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
def blob(name, data=b"X"): return {"name": name, "mimeType": "application/octet-stream", "buffer": data}
def lum(rgb):
    c = [(v / 255) ** 2.2 if v / 255 > 0.04 else v / 255 / 12.92 for v in rgb]
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
r = Run(port=8991, wd="/tmp/wd_converti", motore="python")
try:
    shutil.rmtree("/tmp/s_converti", ignore_errors=True)
    mz = synth.make_series("/tmp/s_converti", [0], blank=False, std=False)[0]
    mzbuf = open(mz, "rb").read()
    with sync_playwright() as p:
        pg = r.page(p)
        raws = [blob(f"campione{i}.raw") for i in range(1, 6)]
        def drop():
            pg.set_input_files("#pick", raws + [blob("s_t0.mzML", mzbuf)])
            pg.wait_for_function("document.querySelector('#unopen') && !document.querySelector('#unopen').hidden", timeout=15000)
            t = pg.inner_text("#unopen")
            assert "5" in t and ".raw" in t, t
            assert pg.locator("#unopen").count() == 1 and pg.locator("#up .fail, #up .recipe").count() == 0
            assert pg.locator("[aria-live=polite]:visible").count() <= 1 or pg.locator("#unopen[aria-live=polite]").count() == 1
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length==1", timeout=15000)
        step("5 .raw + 1 mzML: one notice with 5, the mzML is loaded, nothing else", drop)
        def names():
            assert pg.locator("#unopen details.unopen-names").count() == 1
            assert pg.locator("#unopen details.unopen-names li:visible").count() == 0       # collapsed
            pg.locator("#unopen details.unopen-names summary").click()
            lis = pg.locator("#unopen details.unopen-names li").all_inner_texts()
            assert len(lis) == 5 and all(".raw" in x for x in lis), lis      # exactly 5 names, none hidden by «and N more» when n == 5
        step("the names are in a collapsible list (max 5)", names)
        def guide():
            assert pg.locator("#convhelp").evaluate("d=>d.open"), "opens by itself, once"
            assert ".raw" in pg.inner_text("#convhint")
            assert pg.locator("#convhelp ol li").count() == 5
            a = pg.locator("#convhelp a")
            assert a.get_attribute("rel") == "noopener noreferrer" and a.get_attribute("target") == "_blank" and a.get_attribute("href").startswith("https://proteowizard")
        step("the guide opened by itself with 5 steps and a safe external link", guide)
        def button_kbd():
            pg.evaluate("document.querySelector('#convhelp').open=false")
            pg.focus("#unopen [data-a=help]"); pg.keyboard.press("Enter")
            assert pg.locator("#convhelp").evaluate("d=>d.open")
            assert pg.evaluate("document.activeElement.tagName") == "SUMMARY"
            pg.keyboard.press("Enter"); assert not pg.locator("#convhelp").evaluate("d=>d.open"), "the summary toggles from the keyboard"
        step("«How to convert» opens the guide; the summary works from the keyboard", button_kbd)
        def again():
            pg.locator("#unopen [data-a=close]").click(); assert pg.locator("#unopen").is_hidden()
            pg.set_input_files("#pick", raws)
            pg.wait_for_timeout(800); assert pg.locator("#unopen").is_hidden(), "same group dropped again: no second notice"
        step("close button; the same group dropped again gives no new notice", again)
        def wiff_mixed():
            pg.set_input_files("#pick", [blob("a.wiff"), blob("a.wiff.scan"), blob("b.wiff2"), blob("c.d"), blob("x.lcd"), blob("z.xyz"), blob("q.txt")])
            pg.wait_for_function("!document.querySelector('#unopen').hidden", timeout=10000)
            t = pg.inner_text("#unopen")
            assert "5" in t and ".wiff" in t and "2 file ignorati" in t, t
            pg.locator("#unopen details.unopen-names summary").click()
            assert pg.locator("#unopen details.unopen-names li").count() == 5, "5 names, no «and N more»"
            pg.focus("#unopen [data-a=close]")
            pg.keyboard.press("Escape"); assert pg.locator("#unopen").is_hidden(), "Esc closes"
        step("mixed .wiff/.d/.lcd and unknown formats: one notice, one line per kind; Esc closes", wiff_mixed)
        def more():
            pg.set_input_files("#pick", [blob(f"m{i}.raw") for i in range(8)])
            pg.wait_for_function("!document.querySelector('#unopen').hidden", timeout=10000)
            pg.locator("#unopen details.unopen-names summary").click()
            assert pg.locator("#unopen details.unopen-names li").count() == 6 and "3" in pg.locator("#unopen details.unopen-names li").last.inner_text()
        step("more than 5 names: «and 3 more»", more)
        def en():
            pg.goto(f"http://127.0.0.1:{r.port}/?lang=en"); pg.wait_for_timeout(800)
            assert pg.locator("#convhelp summary").inner_text() == "Don't have mzML files?", pg.locator("#convhelp summary").inner_text()
            pg.evaluate("document.querySelector('#convhelp').open=true"); assert "press Add" in pg.inner_text("#convhelp ol"), pg.inner_text("#convhelp ol")
            pg.set_input_files("#pick", [blob(f"e{i}.raw") for i in range(5)])
            pg.wait_for_function("!document.querySelector('#unopen').hidden", timeout=10000)
            t = pg.inner_text("#unopen"); assert "5 .raw files can't be opened in the browser" in t, t
        step("English text", en)
        def contrast():
            for th in ("light", "dark"):
                pg.evaluate(f"document.documentElement.dataset.theme='{th}'")
                pg.set_input_files("#pick", [blob(f"{th}{i}.raw") for i in range(3)])
                pg.wait_for_function("!document.querySelector('#unopen').hidden", timeout=10000)
                c = pg.evaluate("""()=>{const u=document.querySelector('#unopen'),cs=getComputedStyle(u);const p=s=>s.match(/\\d+(\\.\\d+)?/g).slice(0,3).map(Number);return [p(cs.color),p(cs.backgroundColor)]}""")
                a, b = lum(c[0]), lum(c[1]); ratio = (max(a, b) + .05) / (min(a, b) + .05)
                assert ratio >= 4.5, (th, c, ratio)
            pg.evaluate("delete document.documentElement.dataset.theme")
        step("contrast >= 4.5:1 in light and dark", contrast)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
