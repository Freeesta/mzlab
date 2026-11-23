"""Rate-limit handling of the stat-ci API reads (tools/ci_metrics_api.py) and how the poll behaves when the API denies it."""
import io
import json
import sys
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import ci_metrics as s  # noqa: E402


@pytest.fixture(autouse=True)
def _fresh_api_state():
    s.ghapi.reset()
    yield
    s.ghapi.reset()


class _Denied(urllib.error.HTTPError):
    def __init__(self, code, headers, body=b""):
        super().__init__("http://x", code, "denied", headers, io.BytesIO(body))


def test_api_retries_on_403_honouring_retry_after(monkeypatch):
    calls, sleeps = [], []

    class Resp:
        def __enter__(self): return self
        def __exit__(self, *a): return False
        def read(self, *a): return b'{"ok": true}'
        headers = {"X-RateLimit-Remaining": "900"}

    def fake(req, timeout=0):
        calls.append(1)
        if len(calls) < 3:
            raise _Denied(403 if len(calls) == 1 else 429, {"Retry-After": "7"})
        return Resp()
    monkeypatch.setattr(urllib.request, "urlopen", fake)
    monkeypatch.setattr(s.time, "sleep", sleeps.append)
    assert s.api_get("/commits/x") == {"ok": True}
    assert sleeps == [7.0, 7.0]


def test_api_gives_up_with_apidenied_and_reset_wait(monkeypatch):
    sleeps = []
    err = _Denied(403, {"X-RateLimit-Remaining": "0", "X-RateLimit-Reset": "1060"})
    assert s.denied_wait(err, now=1000.0) == 60.0
    monkeypatch.setattr(urllib.request, "urlopen", lambda req, timeout=0: (_ for _ in ()).throw(_Denied(403, {})))
    monkeypatch.setattr(s.time, "sleep", sleeps.append)
    with pytest.raises(s.ApiDenied):
        s.api_get("/commits/x")
    assert sleeps == [30.0, 30.0, 30.0]


def _fake_urlopen(monkeypatch, *errors):
    calls = []

    def fake(req, timeout=0):
        calls.append(1)
        raise errors[min(len(calls), len(errors)) - 1]
    monkeypatch.setattr(urllib.request, "urlopen", fake)
    return calls


def test_permission_denial_is_not_retried_and_says_why(monkeypatch):
    sleeps = []
    monkeypatch.setattr(s.time, "sleep", sleeps.append)
    calls = _fake_urlopen(monkeypatch, _Denied(403, {}, b'{"message": "Resource not accessible by integration"}'))
    with pytest.raises(s.ApiDenied, match="missing permission"):
        s.api_get("/pulls/1/files")
    assert len(calls) == 1 and sleeps == []


def test_long_wait_is_not_taken_and_later_calls_fail_fast(monkeypatch):
    sleeps = []
    monkeypatch.setattr(s.time, "sleep", sleeps.append)
    calls = _fake_urlopen(monkeypatch, _Denied(429, {"Retry-After": "900"}, b'{"message": "secondary rate limit"}'))
    with pytest.raises(s.ApiDenied, match="asked to wait 900 s.*secondary rate limit"):
        s.api_get("/commits/x")
    with pytest.raises(s.ApiDenied, match="not tried"):
        s.api_get("/commits/y")
    assert len(calls) == 1 and sleeps == [] and s.ghapi.denied()


def test_poll_stops_at_the_quota_reserve(monkeypatch):
    class Resp:
        headers = {"X-RateLimit-Remaining": str(s.ghapi.RESERVE - 1)}
        def __enter__(self): return self
        def __exit__(self, *a): return False
        def read(self, *a): return b"{}"
    monkeypatch.setattr(urllib.request, "urlopen", lambda req, timeout=0: Resp())
    assert s.api_get("/a") == {}
    with pytest.raises(s.ApiDenied, match="reserve"):
        s.api_get("/b")


def test_recent_rows_keep_the_runs_read_before_a_denial(tmp_path):
    runs = [{"id": i, "name": "test", "updated_at": f"2026-11-23T10:0{i}:00Z"} for i in (1, 2, 3)]

    def api(p):
        if p.startswith("/actions/runs?"):
            return {"workflow_runs": runs}
        raise AssertionError(p)

    def rows(run, api, fetch):
        if run["id"] == 3:
            raise s.ApiDenied("HTTP 403 on jobs")
        return [{"tipo": "run", "run_id": run["id"]}]
    s.run_obj_rows, keep = rows, s.run_obj_rows
    try:
        out, cursor = s.recent_rows(str(tmp_path / "e.jsonl"), api=api, ora=datetime(2026, 11, 23, 11, 0, tzinfo=timezone.utc))
    finally:
        s.run_obj_rows = keep
    assert [r["run_id"] for r in out] == [1, 2] and cursor == "2026-11-23T10:02:00Z"


def test_runner_load_is_not_read_twice_in_a_bucket(tmp_path):
    data = tmp_path / "e.jsonl"
    data.write_text(json.dumps({"tipo": "carico", "k": "2026-11-23T11:0"}) + "\n")

    def api(p):
        assert p.startswith("/actions/runs?status=completed"), p
        return {"workflow_runs": []}
    out, _ = s.recent_rows(str(data), api=api, ora=datetime(2026, 11, 23, 11, 3, tzinfo=timezone.utc))
    assert out == []


def test_fill_green_leaves_a_denied_row_untried():
    rows = [{"tipo": "pr", "azione": "unita", "sha": "a", "ts_utc": "2026-11-23T10:00:00Z"} for _ in range(3)]

    def api(p):
        raise s.ApiDenied("quota")
    assert s.fill_green(rows, api=api) == 1
    assert not any(r.get("verde_provato") for r in rows)


def test_denials_exit_only_when_persistent():
    def row(minute):
        return {"tipo": "api_denied", "ts_utc": f"2026-11-23T10:{minute:02d}:00Z", "k": f"2026-11-23T10:{minute // 10}-registra-recenti", "reason": "x"}
    now = datetime(2026, 11, 23, 10, 55, tzinfo=timezone.utc)
    assert s.ghapi.denials_exit([row(5), row(15)], now) == 0
    assert s.ghapi.denials_exit([row(5), row(15), row(25)], now) == 3
    assert s.ghapi.denials_exit([row(5), row(15), row(25)], datetime(2026, 11, 23, 12, 0, tzinfo=timezone.utc)) == 0


def test_download_denial_is_retried_then_raised_not_swallowed(monkeypatch):
    sleeps = []
    monkeypatch.setattr(s.time, "sleep", sleeps.append)

    class Opener:
        def open(self, req, timeout=0):
            raise _Denied(403, {"Retry-After": "5"})
    monkeypatch.setattr(urllib.request, "build_opener", lambda *a: Opener())
    with pytest.raises(s.ApiDenied, match="after the retries"):
        s.dati_ci.download("/actions/jobs/1/logs")
    assert sleeps == [5.0, 5.0, 5.0]
