#!/usr/bin/env python3
"""Job «tipi»: runs tsc on the projects (pagina, moduli, worker, stretto, stretto-worker, stretto2, stretto2-worker) and compares the errors per file with tools/tipi/base_errori.json.

Fails when a file has more errors than its base, or has errors and is not in the base. The base only goes down:
`--aggiorna` rewrites it with min(base, now) and refuses to raise a number or add a file. Errors in vendor/ and wasm/ are not counted.
"""
import json, os, re, signal, subprocess, sys
from concurrent.futures import ThreadPoolExecutor

RADICE = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
BASE = os.path.join(RADICE, "tools", "tipi", "base_errori.json")
PROGETTI = ("pagina", "moduli", "worker", "stretto", "stretto-worker", "stretto2", "stretto2-worker")
TSC = ["npx", "--yes", "-p", "typescript@5.9.3", "tsc", "-p"]
INSTALLA_S, INSTALLA_PROVE = 60, 3      # the install of tsc by npx sometimes hangs on the runner: 60 s per try, the first plus 2 retries
RIGA = re.compile(r"^(\S+?)\(\d+,\d+\): error TS\d+")


def installa(cmd=None, timeout=INSTALLA_S, prove=INSTALLA_PROVE):
    """Runs `npx ... tsc -v` (which installs tsc) with a timeout; a hung try is killed with its whole process group (npx leaves children behind) and retried."""
    cmd = cmd or TSC[:-1] + ["-v"]
    for n in range(1, prove + 1):
        p = subprocess.Popen(cmd, cwd=RADICE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
        try:
            p.communicate(timeout=timeout)
            return
        except subprocess.TimeoutExpired:
            os.killpg(p.pid, signal.SIGKILL)
            p.communicate()
            print("tsc install: try %d of %d hung for %d s, killed" % (n, prove, timeout), flush=True)
    sys.exit("tsc could not be installed by npx in %d tries of %d s (the npm registry or the runner is stuck): run the job again" % (prove, timeout))


def esegui(nome):
    r = subprocess.run(TSC + ["tools/tipi/%s.json" % nome], cwd=RADICE, capture_output=True, text=True)
    if r.returncode not in (0, 1, 2) or (r.returncode and not r.stdout.strip()):
        sys.exit("tsc failed on %s:\n%s%s" % (nome, r.stdout, r.stderr))
    conta = {}
    for riga in r.stdout.splitlines():
        m = RIGA.match(riga)
        if m and "/vendor/" not in m.group(1) and "/wasm/" not in m.group(1):
            k = "%s:%s" % (nome, m.group(1))
            conta[k] = conta.get(k, 0) + 1
    return conta


def misura():
    installa()   # installs tsc once: three npx at the same time corrupt its cache
    ris = {}
    with ThreadPoolExecutor(len(PROGETTI)) as ex:
        for c in ex.map(esegui, PROGETTI):
            ris.update(c)
    return ris


def main():
    base = json.load(open(BASE)) if os.path.exists(BASE) else {}
    ora = misura()
    # «stretto*:» keys (files marked `// @ts-strict`) are never in the base: any error there is a failure
    base = {k: n for k, n in base.items() if not k.startswith("stretto")}
    peggio = ["%s: %d errors (base %d)" % (k, n, base.get(k, 0)) for k, n in sorted(ora.items()) if n > base.get(k, 0)]
    print("tsc errors: %d in %d files (base %d in %d)" % (sum(ora.values()), len(ora), sum(base.values()), len(base)))
    if "--aggiorna" in sys.argv:
        if peggio:
            print("\n".join(peggio)); return 1
        nuova = {k: n for k, n in ora.items() if n > 0}
        with open(BASE, "w") as f:
            json.dump(dict(sorted(nuova.items())), f, indent=1, ensure_ascii=False); f.write("\n")
        return 0
    if peggio:
        print("Worse than tools/tipi/base_errori.json (fix the new errors; the base only goes down):\n" + "\n".join(peggio)); return 1
    meglio = [k for k, n in base.items() if ora.get(k, 0) < n]
    if meglio:
        print("Better than the base in %d files: run python3 tools/tipi/controlla_tsc.py --aggiorna and commit it." % len(meglio))
    return 0


if __name__ == "__main__":
    sys.exit(main())
