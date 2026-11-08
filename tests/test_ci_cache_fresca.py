"""tools/ci_cache_fresca.py: main skips the save while its newest cache of the same prefix is younger than the limit, and saves on any doubt."""
import sys
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import ci_cache_fresca as f  # noqa: E402

NOW = datetime(2026, 11, 5, 12, tzinfo=timezone.utc).timestamp()
P = "rust-v3-Linux-lint-abc-"


def caches(*items):
    return lambda _prefix: [{"key": k, "created_at": t} for k, t in items]


def test_fresh_cache_skips_the_save():
    assert f.fresh(P, caches((P + "1", "2026-11-05T09:00:00Z")), now=NOW, hours=6)


def test_old_cache_saves():
    assert not f.fresh(P, caches((P + "1", "2026-11-05T05:00:00Z")), now=NOW, hours=6)


def test_newest_of_several_decides():
    assert f.fresh(P, caches((P + "1", "2026-11-04T01:00:00Z"), (P + "2", "2026-11-05T11:00:00Z")), now=NOW, hours=6)


def test_no_cache_saves():
    assert not f.fresh(P, caches(), now=NOW)


def test_other_prefix_is_ignored():
    assert not f.fresh(P, caches(("rust-v3-Linux-lintx-abc-1", "2026-11-05T11:00:00Z")), now=NOW)


def test_api_failure_saves():
    def broken(_prefix):
        raise OSError("network")
    assert not f.fresh(P, broken, now=NOW)


def test_main_writes_the_output(tmp_path, monkeypatch):
    out = tmp_path / "out"
    monkeypatch.setenv("GITHUB_OUTPUT", str(out))
    monkeypatch.setattr(f, "newest", caches((P + "1", datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"))))
    assert f.main(["x", P]) == 0
    assert out.read_text() == "fresh=true\n"
