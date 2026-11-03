"""The Ketcher bundle keeps its Indigo worker and wasm as real files (tools/ketcher_estrai_worker.py), never as base64 inside the page script:
decoding 21 MB of base64 on the page thread was seconds of long task in the Disegno iframe."""
import re
from pathlib import Path

KET = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "vendor" / "ketcher"
ASSETS = KET / "assets"
MAX_PAGE_BUNDLE = 6_000_000           # was 29 MB with the worker inline, 7.9 MB with the monomer library


def test_page_bundle_has_no_inline_worker():
    main = next(p for p in ASSETS.glob("index-*.js") if p.stat().st_size > 1_000_000)
    assert main.stat().st_size < MAX_PAGE_BUNDLE, main.stat().st_size
    assert "Lyogcm9sbHVwLXBsdWdpbi13ZWItd29ya2VyLWxvYWRlciAq" not in main.read_text("utf8"), "worker is inline base64 again"
    assert not re.search(r'"[A-Za-z0-9+/=]{200000,}"', main.read_text("utf8")), "a huge base64 literal is back in the page bundle"


def test_worker_and_wasm_files_exist_and_match():
    workers = list(ASSETS.glob("indigo-worker-*.js"))
    wasms = list(ASSETS.glob("indigo-*.wasm"))
    assert len(workers) == 1 and len(wasms) == 1, (workers, wasms)
    assert wasms[0].read_bytes()[:4] == b"\0asm"
    w = workers[0].read_text("utf8")
    assert wasms[0].name in w, "the worker does not name its wasm file"
    assert len(w) < 1_000_000, "the wasm is inline in the worker again"
    main = next(p for p in ASSETS.glob("index-*.js") if p.stat().st_size > 1_000_000).read_text("utf8")
    assert f'"./{workers[0].name}"' in main, "the page bundle does not load the worker by URL"
    html = (KET / "index.html").read_text("utf8")
    assert workers[0].name in html and wasms[0].name in html, "index.html lost the prefetch hints used by first-paint.js"


def test_macromolecules_mode_is_off_and_the_monomer_library_is_empty():
    """tools/ketcher_senza_monomeri.py: the 3.5 MB default monomer library was parsed on every editor creation (seconds of long task)."""
    main = next(p for p in ASSETS.glob("index-*.js") if p.stat().st_size > 1_000_000).read_text("utf8")
    assert "disableMacromoleculesEditor:!0," in main, "macromolecules mode is on again"
    assert '`{"root":{"nodes":[],"connections":[],"templates":[]}}`' in main, "the default monomer library is back"
    assert len(main) < 6_000_000, len(main)
