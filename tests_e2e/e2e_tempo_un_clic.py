# e2e_tempo_un_clic: in the load table, typing in the time of one row and clicking the time of another row focuses it with ONE click (the table is rebuilt on the commit)
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8847, wd="/tmp/wd_tempo_un_clic")
T = "#flist input[data-k=time]:not([disabled])"
ACT = "(()=>{const a=document.activeElement;return a&&a.dataset&&a.dataset.k?[ST.files[+a.dataset.i].name,a.dataset.k,a.value]:null})()"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [mz(f) for f in FILES[:3]])
        def click_other():
            name = pg.evaluate("ST.files[2].name")
            pg.locator(T).nth(0).fill("7")
            pg.locator(T).nth(1).click()   # the t0 time is fixed (disabled): two editable times
            attendi(pg, f"(()=>{{const a={ACT};return !!a&&a[0]===ST.files.find(f=>f.name===({name!r})).name&&a[1]==='time'}})()")
            pg.keyboard.type("5")
            attendi(pg, f"{ACT}&&{ACT}[2].endsWith('5')")
        step("typing in a time, one click on the time of another row: the focus is there and typing goes in", click_other)
        def tab_next():
            pg.locator(T).nth(0).fill("9"); pg.keyboard.press("Tab")
            attendi(pg, "document.activeElement && document.activeElement.dataset.rm !== undefined")      # the next stop after the time is the remove button
        step("Tab from a changed time keeps the focus on the next stop", tab_next)
        def select():
            pg.locator(T).nth(0).fill("3")
            pg.locator("#flist select[data-k=type]").nth(1).click()
            attendi(pg, f"{ACT}&&{ACT}[1]==='type'")
        step("one click on the type select of another row", select)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
