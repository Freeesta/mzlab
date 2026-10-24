"""The errors that stop the assistant have a plain text in both catalogs and on the Flow panel (never the raw key)."""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import lang_catalogo  # noqa: E402
# keys that can end a turn or a send; Rust emits them, the page must say them in words
TERMINAL = ["auth", "rate", "context", "network", "bad_response", "aborted", "provider", "module", "limit.workspace", "limit.spend",
            "limit.calls", "plan.tries", "profile.course", "level.denied", "context.budget", "consent", "key.missing"]


def test_catalogs_and_flow_panel_have_every_terminal_error():
    it = lang_catalogo.testo("it")
    en = lang_catalogo.testo("en")
    flow = (ROOT / "flow/lazy/flow-llm-errors.js").read_text(encoding="utf-8")
    for k in TERMINAL:
        key = f'"err.llm.{k}"'
        assert key in it, f"lang/it.js misses {key}"
        assert key in en, f"lang/en.js misses {key}"
        assert key in flow, f"flow-llm-errors.js misses {key}"


def test_errors_module_is_loaded_with_the_assistant():
    shell = (ROOT / "flow/lazy/flow-shell.js").read_text(encoding="utf-8")
    assert re.search(r'"flow-llm-errors\.js",\s*"flow-llm\.js"', shell)
