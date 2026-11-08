"""bench_visore takes the engine wasm cap from pages.yml, never a copy of the number."""
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))


def test_engine_cap_matches_pages_yml():
    import bench_visore
    cap = bench_visore.engine_cap()
    assert cap.isdigit() and int(cap) > 0
    assert f" {cap} engine" in (ROOT / ".github" / "workflows" / "pages.yml").read_text()
    assert "1884160" not in (ROOT / "tools" / "bench_visore.py").read_text().replace('"1884160" not in', "")
