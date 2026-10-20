"""Flow (administrator page): the JavaScript checker of the plan agrees with the Rust checker `mzlab_core::flow::check` on the shared cases
(crates/mzlab-feat/tests/data/flow_check_cases.json, written by `cargo test -p mzlab-feat --test flow_cases`), and the model of the page behaves
(tests/flow_modello_nodo.js). Needs node."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
CASES = ROOT / "crates/mzlab-feat/tests/data/flow_check_cases.json"
COMPARE_CASES = ROOT / "crates/mzlab-cli/tests/data/flow_compare_cases.json"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node not available")


def node(script):
    r = subprocess.run(["node", str(ROOT / "tests" / script)], capture_output=True, text=True, check=False)
    assert r.returncode == 0, r.stderr[-2000:]
    return r.stdout


def test_the_js_checker_gives_the_answers_of_the_rust_checker():
    cases = json.loads(CASES.read_text(encoding="utf-8"))
    got = json.loads(node("flow_grafico_nodo.js"))["cases"]
    assert len(got) == len(cases) >= 10
    for case, g in zip(cases, got):
        assert g.get("parse_error") == case.get("parse_error"), case["name"]
        assert g.get("got") == case.get("expect"), case["name"]


def test_the_js_checker_gives_the_answers_of_the_rust_checker_for_meta_compare():
    # crates/mzlab-cli/src/flow_compare.rs `check` writes the answers (MZLAB_UPDATE_FLOW_CASES=1 cargo test -p mzlab-cli --lib the_shared_cases)
    cases = json.loads(COMPARE_CASES.read_text(encoding="utf-8"))
    got = json.loads(node("flow_grafico_nodo.js"))["compare"]
    assert len(got) == len(cases) >= 10
    for case, g in zip(cases, got):
        assert g.get("got") == case["expect"], case["name"]


def test_every_case_has_a_text():
    # the keys of the cases are the ones the page has a text for (flow-ui.js F.message)
    ui = (ROOT / "flow/lazy/flow-ui.js").read_text(encoding="utf-8")
    for case in json.loads(CASES.read_text(encoding="utf-8")) + json.loads(COMPARE_CASES.read_text(encoding="utf-8")):
        for key in [line.split("|")[2] for line in case.get("expect") or []] + [case.get("parse_error")]:
            assert key is None or f'"{key}"' in ui, key


def test_the_model_of_the_page_behaves():
    assert node("flow_modello_nodo.js").strip() == "ok"


def test_flow_is_english_only():
    # same rule and same detector as Tabui (tests/test_tabui_inglese.py): no Italian in the strings of the page, no catalogue key in lang/
    import importlib.util
    spec = importlib.util.spec_from_file_location("tabui_inglese", ROOT / "tests/test_tabui_inglese.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    bad = []
    for p in sorted((ROOT / "flow").rglob("*.js")):
        for s in mod.literals(p):
            text = " ".join(mod.re.sub(r"<[^>]*>", " ", s).split())
            if " " in text and mod.ITALIAN.search(text):
                bad.append(f"{p.relative_to(ROOT)}: {s[:80]}")
    assert not bad, bad


def test_flow_is_shipped_with_the_descriptors():
    site = (ROOT / "tools/build_site.py").read_text(encoding="utf-8")
    for name in ("steps-1.json", "steps-attributio-1.json", "steps-tabui-1.json", '"flow"'):
        assert name in site, name
    tool = (ROOT / "flow/js/flow-tool.js").read_text(encoding="utf-8")
    for part in (ROOT / "flow/lazy").glob("flow-*.js"):
        assert part.name in tool, part.name


def test_the_tabui_views_read_their_files():
    assert node("flow_viste_tabui.js").strip() == "ok"


def test_the_review_view_queues_and_words_the_decisions():
    assert node("flow_review.js").strip() == "ok"


def test_the_feature_table_view_reads_the_names_of_its_files():
    assert node("flow_feat_view.js").strip() == "ok"


def test_the_settings_digest_is_read_from_the_evidence_summary():
    assert node("flow_digest.js").strip() == "ok"
