"""pytest plugin of the CI (`-p ci_pytest_tempi` with PYTHONPATH=tools): per test file, the seconds spent in setup, call and teardown and the
xdist worker that ran it, written to tempi-test.json for tools/ci_metrics.py. Runs in the xdist controller only; no test is added or dropped."""
import json
import os
from collections import defaultdict

PERCORSO = os.environ.get("MZLAB_TEMPI_TEST", "tempi-test.json")
_per_file = defaultdict(lambda: {"setup": 0.0, "call": 0.0, "teardown": 0.0, "n": 0, "worker": set()})


def pytest_runtest_logreport(report):
    if report.when not in ("setup", "call", "teardown"):
        return
    f = _per_file[report.nodeid.split("::", 1)[0]]
    f[report.when] += report.duration
    if report.when == "call":
        f["n"] += 1
    nodo = getattr(report, "node", None)
    if nodo is not None:
        f["worker"].add(nodo.workerinput.get("workerid", "?"))


def pytest_sessionfinish(session):
    # a xdist worker has `workerinput`; only the controller writes
    if hasattr(session.config, "workerinput"):
        return
    out = {k: {"setup": round(v["setup"], 2), "call": round(v["call"], 2), "teardown": round(v["teardown"], 2), "n": v["n"],
               "worker": sorted(v["worker"])} for k, v in sorted(_per_file.items())}
    with open(PERCORSO, "w", encoding="utf-8") as fh:
        json.dump(out, fh, separators=(",", ":"))
