"""Every Flow step of the registry (docs/schemi/steps-*-1.json, the exports of `mzlab_cli::flow::steps()`) runs in the browser too: it has an executor in
`F.liveJobs` (flow/lazy/flow-live-jobs.js, plus the ones other files add to it). The native side is guarded by `flow_rule_tests.rs` (`native: true` = an
executor of `flow run`). The steps the browser does not run yet stay in SOLO_APP with the reason (docs/agenti/parita.md); the list only shrinks."""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# step -> why the browser Flow has no executor for it
SOLO_APP = {
    "import.raw": "the readers module opens maker files in Data; the browser plan starts from index.mzidx of the files opened there",
    "features.table": "the whole chain in one step: the browser gives the same CSV with the base chain and export.table_csv (flow-web.md)",
    "structure.draw": "the Ketcher editor of the page: a drawing, not a computation (native: false)",
    "features.ms2_link": "inside features.detect on every face; no executor anywhere (BUCHI of flow_rule_tests.rs)",
    "annot.hazard": "descriptor only, no executor anywhere (BUCHI of flow_rule_tests.rs)",
    "convert.mzml": "gap: the site converts at upload (conversione.js), not as a Flow step",
    "features.estimate": "gap: run::preview is in the .wasm feat of the Pipeline panel, not wired as a Flow executor",
    "table.plausible": "gap: no Flow executor in the browser yet",
    "spectra.chimeric": "gap: no Flow executor in the browser yet",
    "rt.predict": "gap: no Flow executor in the browser yet",
    "structure.render": "gap: interp::redraw is in the .wasm interp, not wired as a Flow executor",
    "structure.reactivity": "gap: reactivity_json is in the .wasm chem, not wired as a Flow executor",
    "msn.figure": "gap: the figure of the .wasm msngrafo is not wired as a Flow executor",
    "meta.compare": "gap: flow-compare.js adds the comparison to the plan; the browser runner refuses it until the executor exists",
    "spectra.lookup": "gap: no Flow executor in the browser yet",
    "chrom.uv_peaks": "gap: no Flow executor in the browser yet",
    "quant.calibrate": "gap: no Flow executor in the browser yet",
}


def registry():
    steps = {}
    for f in sorted((ROOT / "docs/schemi").glob("steps-*1.json")):
        for line in f.read_text(encoding="utf-8").splitlines():
            line = line.strip().rstrip(",")
            if line.startswith("{"):
                s = json.loads(line)
                steps[s["id"]] = s
    return steps


def web_executors():
    keys = set()
    for f in sorted((ROOT / "flow/lazy").glob("*.js")):
        text = f.read_text(encoding="utf-8")
        for body in re.findall(r"F\.liveJobs\s*=\s*\{(.*?)\};", text) + re.findall(r"Object\.assign\(v,\s*\{(.*?)\}\)", text):
            keys |= set(re.findall(r'"([a-z]+\.[a-z0-9_]+)"\s*:', body))
    return keys


def test_the_registry_is_read():
    steps = registry()
    assert len(steps) > 40 and "features.detect" in steps, sorted(steps)
    assert {"index.mzidx", "features.detect", "formula.ms1"} <= web_executors()


def test_every_step_has_a_browser_executor():
    missing = sorted(set(registry()) - web_executors() - set(SOLO_APP))
    assert not missing, f"steps the browser Flow cannot run: {missing} (add the executor to F.liveJobs, or name the reason in SOLO_APP)"


def test_the_app_only_list_holds_no_stale_name():
    steps, web = registry(), web_executors()
    stale = sorted(s for s in SOLO_APP if s not in steps or s in web)
    assert not stale, f"SOLO_APP names steps the registry lacks or the browser runs: {stale}"


def test_a_browser_executor_is_a_step_of_the_registry():
    unknown = sorted(web_executors() - set(registry()))
    assert not unknown, f"F.liveJobs runs steps the registry does not have: {unknown}"
