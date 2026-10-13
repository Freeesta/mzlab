# e2e_carica_grandi: many big mzML loaded together with little room left in the browser. A file that does not fit says so (error.quota, in words), never the raw key
# error.badIndex of an index cut short; "Load data" waits until every file is read; no data of the laboratory: the files are made here (mzml_grande.py).
import os, sys, tempfile, time; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import mzml_grande
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
def free_mb(pg):          # the room left in the browser, after the page has stopped filling its own cache
    last, same = -1, 0
    while same < 4:
        u = pg.evaluate("navigator.storage.estimate().then(e => e.quota - e.usage)"); same = same + 1 if u == last else 0; last = u; time.sleep(1)
    return last / 1048576
def scenario(n_ms1, n_ms2, mb_each):
    """Loads together more files than the browser has room for (the room is asked first: the quota depends on the machine)."""
    r = Run(port=8847, wd="/tmp/wd_carica_grandi"); d = tempfile.mkdtemp(prefix="grandi_"); names = []
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            count = min(16, int(free_mb(pg) / mb_each) + 2)
            for k in range(count):
                f = os.path.join(d, f"grande{k + 1}.mzML"); mzml_grande.write(f, n_ms1=n_ms1, n_ms2=n_ms2, seed=k + 1); names.append(f)
            pg.set_input_files("#pick", names)
            pg.wait_for_function("UPBUSY > 0 || ST.files.length > 0 || QQQ_LOADFAIL.size > 0")
            t0, first = time.time(), None
            while time.time() - t0 < 600:
                if first is None and not pg.evaluate("document.querySelector('#opbtn').disabled"):
                    first = pg.evaluate("[ST.files.length, QQQ_LOADFAIL.size, UPBUSY]")        # the first moment the button is on
                if pg.evaluate("UPBUSY") == 0: break
                time.sleep(0.5)
            pg.wait_for_timeout(500)
            return count, first, pg.evaluate("[ST.files.length, [...QQQ_LOADFAIL], document.body.innerText]")
    finally:
        r.close()
        for f in names: os.remove(f)
        os.rmdir(d)
def check(count, first, got):
    ok, failed, text = got
    assert "error.badIndex" not in text and "badIndex" not in text, "raw key in the page"
    assert ok + len(failed) == count, f"{ok} loaded + {len(failed)} failed != {count} "
    assert len(failed) >= 1 and ok >= 1, f"the room should fit some of the {count} files and not all: {ok} loaded, {len(failed)} failed"
    assert "not enough space" in text or "abbastanza spazio" in text, "the failed files are explained as lack of space"
    assert first is None or first[0] + first[1] == count, f"the button was on before all files were read: {first}"
def too_many():           # under 50 MB each, so no check up front: a file or an index cut short by the browser is caught after writing
    check(*scenario(600, 6000, 80))
def room_enough():        # room for everything: the button waits for the last file, then opens them ALL and the loading screen goes away
    d = tempfile.mkdtemp(prefix="grandi_"); names = []
    for k in range(3):
        p = os.path.join(d, f"grande{k + 1}.mzML"); mzml_grande.write(p, n_ms1=600, n_ms2=6000, seed=k + 1); names.append(p)
    r = Run(port=8848, wd="/tmp/wd_carica_grandi2")
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            pg.set_input_files("#pick", names)
            pg.wait_for_function("UPBUSY > 0 || ST.files.length > 0")
            t0 = time.time()
            while pg.evaluate("document.querySelector('#opbtn').disabled") and time.time() - t0 < 300: time.sleep(0.5)
            assert pg.evaluate("ST.files.length") == 3, "the button was on before the last file was read: %s" % pg.evaluate("[ST.files.length, UPBUSY, ST.files.map(f=>f.name)]")
            pg.click("#opbtn"); ready(pg, timeout=240000)
            assert pg.evaluate("E.files.length") == 6, pg.evaluate("E.files.map(f=>f.file)")             # MS1 and MS2 of each of the 3 files
            assert pg.evaluate("document.getElementById('loading').hidden"), "the loading screen is still up"
    finally:
        r.close()
        for f in names: os.remove(f)
        os.rmdir(d)
step("more files than room: a clear message, nothing cut short", too_many)
step("room for all: the button waits for the last file, all the files open", room_enough)
print("\nSTEPS"); [print(" ", s) for s in steps]
if any(s[1] != "ok" for s in steps): sys.exit(1)
