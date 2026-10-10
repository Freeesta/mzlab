# TPMINE-PRIVATE
"""«Dig» live feed (public, synthetic): the engines tell the phases in order, with valid payloads; the emitter coalesces and caps them."""
import json

import pytest

import lc_synth as LS
import lr_synth
import msn_synth as MS
from tpmine.api import _Emitter
from tpmine.engine import Experiment
from tpmine.hr import engine as EN

LR_PHASES = ["calibration", "candidates", "xic", "unexpected", "ms2", "tables"]
HR_PHASES = ["tree", "features", "align", "candidates", "families", "network"]


def _record(run):
    got = []
    sink = lambda text, frac, phase, payload: got.append((text, frac, phase, None if payload is None else json.loads(payload)))   # noqa: E731
    em = _Emitter(sink)
    run(em)
    em.flush()
    return got


def _order(got):
    out = []
    for _, _, ph, _ in got:
        if ph and ph not in out:
            out.append(ph)
    return out


def _check_sizes(got, kinds):
    for _, _, _, pl in got:
        if pl is not None:
            assert isinstance(pl, list) and len(json.dumps(pl)) <= 50_000 and all(p["kind"] in kinds for p in pl)


def test_lr_phases_and_candidates(tmp_path):
    files = lr_synth.write_series(tmp_path)
    got = _record(lambda em: Experiment(files, {"name": "Parent", "neutral": "CC(C)N1C(=O)C2=CC=CC=C2NS1(=O)=O"}, {}, progress=em).run())
    order = _order(got)
    assert [p for p in LR_PHASES if p in order] == order and {"calibration", "candidates", "xic", "tables"} <= set(order)
    _check_sizes(got, {"candidate"})
    cands = [p for _, _, _, pl in got if pl for p in pl]
    assert cands and all({"id", "name", "mz", "rt", "score"} <= set(c) and 0 <= c["score"] <= 100 for c in cands)


def test_hr_phases_tree_features_candidates(tmp_path):
    files = LS.write_series(tmp_path)
    msn = MS.write_msn(tmp_path / "cafe_msn.mzML", MS.caffeine_nodes())
    allf = files + [{"name": "cafe_msn.mzML", "path": str(msn), "time": None, "type": "sample"}]
    got = _record(lambda em: EN.ExperimentHR(allf, {"smiles": "Cn1cnc2c1c(=O)n(C)c(=O)n2C"}, progress=em).run())
    assert _order(got) == HR_PHASES
    _check_sizes(got, {"node", "features", "candidate"})
    pl = [p for _, _, _, x in got if x for p in x]
    nodes = [p for p in pl if p["kind"] == "node"]
    ids = {n["id"] for n in nodes}
    assert nodes[0]["id"] == "MS1" and len(ids) == len(nodes) > 1 and all(n["parent"] in ids for n in nodes[1:])
    assert len([p for p in pl if p["kind"] == "features"]) == len(files)
    assert any(p["kind"] == "candidate" and {"id", "name", "mz", "rt", "score"} <= set(p) for p in pl)


def test_emitter_coalesces_and_caps():
    sent = []
    em = _Emitter(lambda *a: sent.append(a))
    em("a", 0.1, "x")
    for i in range(300):
        em("b", 0.2, "x", {"kind": "candidate", "i": i, "pad": "z" * 300})
    em("c", 0.3, "y")                                   # a new phase leaves at once and flushes the old one first
    em.flush()
    assert len(sent) < 12 and sent[0][2] == "x" and sent[-1][2] == "y"
    got = [p for s in sent if s[3] for p in json.loads(s[3])]
    assert [p["i"] for p in got] == list(range(300)) and all(len(s[3]) <= 50_000 for s in sent if s[3])
    assert [s[2] for s in sent if s[0] == "c"] == ["y"]
