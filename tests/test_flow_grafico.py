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
    for name in ("steps-1.json", "steps-attributio-1.json", "steps-tabui-1.json", "steps-cli-1.json", '"flow"'):
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


def test_every_start_template_is_a_valid_plan():
    assert node("flow_modelli_partenza.js").strip() == "ok"


def test_every_step_of_the_base_has_a_valid_theory_anchor():
    import re
    src = (ROOT / "flow" / "lazy" / "flow-issues.js").read_text(encoding="utf-8")
    entries = {k: (f, a) for k, f, a in re.findall(r'"([a-z0-9_.]+)": \["([^"]+)", "([^"]+)"\]', src)}
    names = (ROOT / "flow" / "lazy" / "flow-names.js").read_text(encoding="utf-8")
    base = re.findall(r'steps: \[([^\]]*)\] \},\n', names.split("F.BASE = [")[1].split("];")[0] + "\n")
    base_steps = [s for group in base for s in re.findall(r'"([a-z0-9_.]+)"', group)]
    assert base_steps, "no step of the base found"
    for step in base_steps:
        assert step in entries, f"{step} has no entry of the Theory"
    for step, (file, anchor) in entries.items():
        html = (ROOT / "mzlab" / "web" / "teoria" / file).read_text(encoding="utf-8")
        assert f'id="{anchor}"' in html, (step, file, anchor)


def test_the_msn_graph_view_reads_the_modifications_and_the_ions():
    assert node("flow_msn_view.js").strip() == "ok"


def test_the_import_card_reads_a_metadata_table_and_lays_it_over_the_files():
    assert node("flow_import_meta.js").strip() == "ok"


def test_the_xlsx_reader_gives_the_first_sheet_of_a_workbook():
    assert node("flow_xlsx.js").strip() == "ok"


def test_the_cli_box_puts_the_folder_in_the_command_once_and_quoted():
    assert node("flow_cli_fill.js").strip() == "ok"
