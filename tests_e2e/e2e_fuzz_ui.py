"""Smoke round of the interaction fuzzer (tools/fuzz_ui.py): two fixed seeds of random keys, clicks, drags, wheel, resizes, views, files, language, theme
and reloads (Data on a desktop window, every view on a 1024x768 touch tablet) and a leak run of open/close cycles. Any page error, rejection, stuck busy
state, focus lost to the body, duplicated row, NaN shown or growth of DOM nodes/listeners fails; the replay of a failure is printed (seed + steps).
Needs the built site (tools/prepara_sito.py). Run from the repo root: python3 tests_e2e/e2e_fuzz_ui.py"""
# covers: tools/fuzz_ui.py
import argparse, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import fuzz_ui as fz          # tools/ is on the path through lib

OUT = HERE / "shots" / "fuzz"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:600]))

def args(**k):
    a = dict(seed=1, passi=40, vista="data", tablet=False, busy=30.0, cicli=0, minimizza=False, replay=None, out=str(OUT), v=False, ignora=None)
    a.update(k); return argparse.Namespace(**a)

def seed_run(p, **k):
    a = args(**k)
    f = fz.Fuzz(a, fz.files_for(a.vista, OUT / "dati"))
    done, i, probs = f.play(p, seed=a.seed)
    assert i is None, f"seed {a.seed} step {i}: {probs}; last steps {done[-4:]}"
    assert sum(1 for x in f.log if x["ran"]) >= a.passi // 2, "most steps found no target: the fuzzer tests nothing"

def leak_run(p):
    a = args(cicli=6)
    rows = fz.leak(fz.Fuzz(a, fz.files_for("data", OUT / "dati")), p)
    grow = {k: (rows[-1][k] - rows[1][k]) / (len(rows) - 2) for k in ("nodes", "listeners")}
    assert grow["nodes"] <= 50 and grow["listeners"] <= 10, (grow, rows)

with sync_playwright() as p:
    step("seed 3, Data with three synthetic files, desktop", lambda: seed_run(p, seed=3, passi=50))
    step("seed 103, every view, 1024x768 touch tablet", lambda: seed_run(p, seed=103, passi=50, vista="mix", tablet=True))
    step("6 open/close cycles: DOM nodes and listeners do not grow", lambda: leak_run(p))
fails = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("OK   " if s == "ok" else "FAIL ") + n + ("" if s == "ok" else "  -> " + s))
sys.exit(1 if fails else 0)
