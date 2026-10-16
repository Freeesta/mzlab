# e2e_carica_grandi: many big mzML loaded together with little room left in the browser. A file that does not fit says so (error.quota, in words), never the raw key
# error.badIndex of an index cut short; "Load data" can be pressed while the files are read: the overlay shows the phase and the percentage, then all of them open; no data of the laboratory: the files are made here (mzml_grande.py).
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
            pg.evaluate("window.__seen = []; new MutationObserver(() => window.__seen.push(document.getElementById('ldmsg').textContent)).observe(document.getElementById('ldmsg'), {childList: true, characterData: true, subtree: true})")          # what the overlay says, at every change
            pg.set_input_files("#pick", names)
            pg.wait_for_function("UPBUSY > 0 || ST.files.length > 0 || QQQ_LOADFAIL.size > 0")
            t0 = time.time()
            pg.click("#opbtn")                                              # pressed while the files are still being read
            while time.time() - t0 < 600 and pg.evaluate("UPBUSY") > 0: time.sleep(0.3)
            pg.wait_for_timeout(500)
            return count, pg.evaluate("window.__seen"), pg.evaluate("[ST.files.length, [...QQQ_LOADFAIL], document.body.innerText + ' ' + document.getElementById('start').textContent]")
    finally:
        r.close()
        for f in names: os.remove(f)
        os.rmdir(d)
def check(count, seen, got):
    ok, failed, text = got
    assert "error.badIndex" not in text and "badIndex" not in text, "raw key in the page"
    assert ok + len(failed) == count, f"{ok} loaded + {len(failed)} failed != {count} "
    assert len(failed) >= 1 and ok >= 1, f"the room should fit some of the {count} files and not all: {ok} loaded, {len(failed)} failed"
    assert "not enough space" in text or "abbastanza spazio" in text, "the failed files are explained as lack of space"
    assert any("%" in m for m in seen), f"the overlay never showed a percentage while reading: {seen[:5]}"
def too_many():           # the start screen is hidden once the files that fit have opened: its text is read too; under 50 MB each, so no check up front: a file or an index cut short by the browser is caught after writing
    check(*scenario(600, 6000, 80))
def room_enough():        # room for everything: pressed while reading, the button waits for the last file, then opens them ALL and the loading screen goes away
    d = tempfile.mkdtemp(prefix="grandi_"); names = []
    for k in range(3):
        p = os.path.join(d, f"grande{k + 1}.mzML"); mzml_grande.write(p, n_ms1=600, n_ms2=6000, seed=k + 1); names.append(p)
    r = Run(port=8848, wd="/tmp/wd_carica_grandi2")
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            pg.evaluate("window.__seen = []; new MutationObserver(() => window.__seen.push(document.getElementById('ldmsg').textContent)).observe(document.getElementById('ldmsg'), {childList: true, characterData: true, subtree: true})")
            pg.evaluate("addEventListener('mzlab-index', () => document.getElementById('opbtn').click(), {once: true})")     # pressed at the first sign of progress: the files are surely still being read
            pg.set_input_files("#pick", names)
            pg.wait_for_function("UPBUSY === 0 && ST.files.length === 3", timeout=240000)
            import lib; lib._tempi(pg)                              # the press while reading waited for the files and opens them; if «Open» still asked for the times (roles and times of the table) they are written and it is pressed again
            if pg.evaluate("!document.getElementById('start').hidden && document.getElementById('loading').hidden"): pg.click("#opbtn")
            ready(pg, timeout=240000)
            seen = pg.evaluate("window.__seen")
            assert any("%" in m for m in seen), f"no percentage on the overlay while reading: {seen[:5]}"
            assert pg.evaluate("E.files.length") == 6, pg.evaluate("E.files.map(f=>f.file)")             # MS1 and MS2 of each of the 3 files
            assert pg.evaluate("document.getElementById('loading').hidden"), "the loading screen is still up"
    finally:
        r.close()
        for f in names: os.remove(f)
        os.rmdir(d)
step("more files than room: a clear message, nothing cut short, percentage on the overlay", too_many)
step("room for all: pressed while reading, the overlay shows the phase and all the files open", room_enough)
print("\nSTEPS"); [print(" ", s) for s in steps]
if any(s[1] != "ok" for s in steps): sys.exit(1)
