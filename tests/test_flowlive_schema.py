"""flowlive/1: the events of the live scheduler (crates/mzlab-core/src/flow/sched.rs) recorded in the shared cases validate against
docs/schemi/flowlive-1.json, `seq` grows by one and `rev` never goes back. The cases are written by `cargo test -p mzlab-core --lib flow::sched`."""
import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = json.loads((ROOT / "docs/schemi/flowlive-1.json").read_text(encoding="utf-8"))
CASES = json.loads((ROOT / "crates/mzlab-core/tests/data/sched_cases.json").read_text(encoding="utf-8"))


def logged():
    return [c["expect"] for case in CASES for c in case["calls"] if c["op"] == "events" and c.get("full")]


def test_the_events_of_a_case_follow_the_schema():
    logs = logged()
    assert logs, "a shared case has to record its events"
    Draft202012Validator.check_schema(SCHEMA)
    validator = Draft202012Validator(SCHEMA)
    for events in logs:
        assert len(events) >= 10
        for e in events:
            validator.validate(e)
        assert [e["seq"] for e in events] == list(range(1, len(events) + 1))
        revs = [e["rev"] for e in events]
        assert revs == sorted(revs) and revs[-1] >= 1
        assert {e["kind"] for e in events} >= {"job", "node"}
        assert {"error", "paused", "stale"} <= {e["state"] for e in events}
        assert all(e["item"] is None for e in events if e["kind"] == "node")
